//! Streaming bindings trait for top-level table scans.
//!
//! Storage engines implement [`BindingsScan`] on their custom `EvalContext`
//! to expose tables as **lazy iterators** of [`Value`] rather than fully
//! materialised `MapBindings<Value>`. The evaluator's `TableScan` operator
//! checks [`EvalContext::bindings_scan`] before falling back to the eager
//! `MapBindings` lookup.
//!
//! This is the column-resolution counterpart of [`crate::eval::CollCount`]:
//! both are optional capabilities advertised by `EvalContext` so that the
//! evaluator can short-circuit allocation-heavy paths when the storage
//! engine knows how to do better.
//!
//! See `docs/adr/ADR-009-streaming-bindings-partiql-eval.md` in the FDE
//! repository for the design rationale and the migration plan.

use partiql_value::{BindingsName, Value};

/// Lazy table-iteration trait.
///
/// Storage engines implement this on their custom `EvalContext` to provide
/// row-by-row iteration over a named table without materialising the whole
/// table as a `Value::Bag`. The evaluator calls this for top-level
/// `FROM table_name` references; complex `FROM` expressions
/// (subqueries, path navigations) keep using the eager `Bindings<Value>` path.
///
/// ## Send / Sync
///
/// The trait itself is **not** `Send + Sync` because typical
/// `EvalContext` implementations hold `partiql_value::Value` (which embeds
/// `Rc<SimpleGraph>` in graph variants and is therefore `!Sync`). Forcing
/// `Self: Send + Sync` would make most contexts unable to implement the
/// trait at all.
///
/// The **iterator return type** keeps `+ Send` so storage engines can
/// produce iterators that move across thread boundaries (e.g. for future
/// parallel scans). FDE's `FdeBindings` already returns a `Send` iterator
/// by collecting `Arc<Vec<u8>>` clones into an owned `Vec` before mapping
/// to `Value` — see the spike at `fde/fde-core/tests/streaming_spike.rs`.
pub trait BindingsScan {
    /// Iterate the rows of a top-level table by name.
    ///
    /// Implementations should return an empty iterator (not an error) when
    /// the named table is unknown — this matches the behaviour of the
    /// eager `Bindings<Value>::get` returning `None`, which the evaluator
    /// already treats as "empty input".
    fn scan<'a>(
        &'a self,
        name: &BindingsName<'_>,
    ) -> Box<dyn Iterator<Item = Value> + Send + 'a>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::basic::MapBindings;
    use crate::eval::{BasicContext, EvalContext, EvaluationError};
    use partiql_catalog::context::{Bindings, SessionContext, SystemContext};
    use partiql_value::{DateTime, Value};
    use std::any::Any;
    use std::borrow::Cow;
    use std::cell::RefCell;

    /// Test context that implements [`BindingsScan`] over a fixed list of
    /// integer row IDs. We store `Vec<i64>` instead of `Vec<Value>` because
    /// `partiql_value::Value` is `!Sync` (it embeds `Rc<SimpleGraph>` in
    /// the graph variants), so a borrowing iterator like `Iter<'_, Value>`
    /// can never be `Send`. Producing `Value::Integer` inline inside the
    /// iterator closure sidesteps the constraint — `IntoIter<i64>` is
    /// trivially `Send`, and the closure captures nothing.
    #[derive(Debug)]
    struct TestStreamingContext {
        bindings: MapBindings<Value>,
        sys: SystemContext,
        errors: RefCell<Vec<EvaluationError>>,
        row_ids: Vec<i64>,
    }

    impl TestStreamingContext {
        fn new(bindings: MapBindings<Value>, row_ids: Vec<i64>) -> Self {
            Self {
                bindings,
                sys: SystemContext {
                    now: DateTime::from_system_now_utc(),
                },
                errors: RefCell::new(vec![]),
                row_ids,
            }
        }
    }

    impl BindingsScan for TestStreamingContext {
        fn scan<'a>(
            &'a self,
            _name: &BindingsName<'_>,
        ) -> Box<dyn Iterator<Item = Value> + Send + 'a> {
            // Clone the owned ids out of self so the returned iterator
            // doesn't borrow from `self.row_ids` — borrowing slices of
            // `Value` would force `Value: Sync`, which it isn't.
            let ids: Vec<i64> = self.row_ids.clone();
            Box::new(ids.into_iter().map(Value::from))
        }
    }

    impl SessionContext for TestStreamingContext {
        fn system_context(&self) -> &SystemContext {
            &self.sys
        }
        fn user_context(&self, _name: &str) -> Option<&dyn Any> {
            None
        }
    }

    impl Bindings<Value> for TestStreamingContext {
        fn get<'a>(&'a self, name: &BindingsName<'_>) -> Option<Cow<'a, Value>> {
            self.bindings.get(name)
        }
    }

    impl EvalContext for TestStreamingContext {
        fn add_error(&self, error: EvaluationError) {
            self.errors.borrow_mut().push(error);
        }
        fn has_errors(&self) -> bool {
            !self.errors.borrow().is_empty()
        }
        fn errors(&self) -> Vec<EvaluationError> {
            self.errors.take()
        }
        fn bindings_scan(&self) -> Option<&dyn BindingsScan> {
            Some(self)
        }
    }

    /// Default `EvalContext` impls (e.g. `BasicContext`) MUST return `None`
    /// from `bindings_scan` so existing consumers stay on the eager path.
    /// This is the behavioural contract for backward compatibility.
    #[test]
    fn basic_context_does_not_advertise_streaming() {
        let ctx = BasicContext::new(
            MapBindings::default(),
            SystemContext {
                now: DateTime::from_system_now_utc(),
            },
        );
        assert!(ctx.bindings_scan().is_none());
    }

    /// A custom context that implements `BindingsScan` should advertise it
    /// via the new accessor.
    #[test]
    fn custom_context_advertises_streaming() {
        let ctx = TestStreamingContext::new(MapBindings::default(), vec![1, 2, 3]);
        assert!(ctx.bindings_scan().is_some());
    }

    /// The advertised `BindingsScan` returns the rows we registered.
    #[test]
    fn streaming_iterator_yields_registered_rows() {
        let ctx = TestStreamingContext::new(MapBindings::default(), vec![10, 20, 30]);
        let scanner = ctx.bindings_scan().expect("streaming advertised");
        let name = BindingsName::CaseInsensitive(Cow::Borrowed("anything"));
        let collected: Vec<Value> = scanner.scan(&name).collect();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], Value::from(10));
        assert_eq!(collected[2], Value::from(30));
    }

    /// The streaming iterator must be `Send` so the evaluator may move it
    /// across thread boundaries — this is a *compile-time* contract.
    #[test]
    fn streaming_iterator_is_send() {
        fn assert_send<T: Send>(_: &T) {}

        let ctx = TestStreamingContext::new(MapBindings::default(), vec![1]);
        let scanner = ctx.bindings_scan().expect("streaming advertised");
        let name = BindingsName::CaseInsensitive(Cow::Borrowed("anything"));
        let iter = scanner.scan(&name);
        assert_send(&iter);
    }

    /// The trait object is intentionally **not** `Send + Sync` — see the
    /// trait-level docs. What FDE actually needs is for the **iterator
    /// returned by `scan`** to be `Send`, which is enforced by the trait
    /// method's return type. This test pins that contract: any iterator
    /// produced via the trait method satisfies `Send`.
    #[test]
    fn returned_iterator_satisfies_send_via_trait_method() {
        fn assert_send<T: Send>(_: &T) {}

        let ctx = TestStreamingContext::new(MapBindings::default(), vec![1, 2, 3]);
        // Look up via trait object — exercises the same path the evaluator
        // will use through `EvalContext::bindings_scan()`.
        let scanner: &dyn BindingsScan = ctx.bindings_scan().expect("streaming advertised");
        let name = BindingsName::CaseInsensitive(Cow::Borrowed("anything"));
        let iter = scanner.scan(&name);
        assert_send(&iter);
    }
}

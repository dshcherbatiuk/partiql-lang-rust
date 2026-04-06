//! Aggregate pushdown trait for COUNT.
//!
//! Storage engines implement [`CollCount`] to return row count without
//! materializing all rows into a `Bag`. The evaluator checks
//! [`EvalContext::as_coll_count`] before falling back to iteration.

/// Trait for optimized COUNT pushdown.
///
/// Storage engines implement this on their custom `EvalContext` to provide
/// O(1) row counts. The evaluator calls this before iterating the collection.
pub trait CollCount {
    /// Returns the count of rows in the current collection.
    fn coll_count(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::basic::MapBindings;
    use crate::eval::{BasicContext, EvalContext, EvaluationError};
    use partiql_catalog::context::{Bindings, SessionContext, SystemContext};
    use partiql_value::{BindingsName, DateTime, Value};
    use std::any::Any;
    use std::borrow::Cow;
    use std::cell::RefCell;

    /// Test context that implements CollCount with a fixed count.
    #[derive(Debug)]
    struct TestCountContext {
        bindings: MapBindings<Value>,
        sys: SystemContext,
        errors: RefCell<Vec<EvaluationError>>,
        count: usize,
    }

    impl TestCountContext {
        fn new(bindings: MapBindings<Value>, count: usize) -> Self {
            Self {
                bindings,
                sys: SystemContext {
                    now: DateTime::from_system_now_utc(),
                },
                errors: RefCell::new(vec![]),
                count,
            }
        }
    }

    impl CollCount for TestCountContext {
        fn coll_count(&self) -> usize {
            self.count
        }
    }

    impl SessionContext for TestCountContext {
        fn system_context(&self) -> &SystemContext {
            &self.sys
        }
        fn user_context(&self, _name: &str) -> Option<&dyn Any> {
            None
        }
    }

    impl Bindings<Value> for TestCountContext {
        fn get<'a>(&'a self, name: &BindingsName<'_>) -> Option<Cow<'a, Value>> {
            self.bindings.get(name)
        }
    }

    impl EvalContext for TestCountContext {
        fn add_error(&self, error: EvaluationError) {
            self.errors.borrow_mut().push(error);
        }
        fn has_errors(&self) -> bool {
            !self.errors.borrow().is_empty()
        }
        fn errors(&self) -> Vec<EvaluationError> {
            self.errors.take()
        }
        fn as_coll_count(&self) -> Option<&dyn CollCount> {
            Some(self)
        }
    }

    #[test]
    fn test_basic_context_returns_none() {
        let ctx = BasicContext::new(
            MapBindings::default(),
            SystemContext {
                now: DateTime::from_system_now_utc(),
            },
        );
        assert!(ctx.as_coll_count().is_none());
    }

    #[test]
    fn test_custom_context_returns_count() {
        let ctx = TestCountContext::new(MapBindings::default(), 42);
        let counter = ctx.as_coll_count().expect("should return CollCount");
        assert_eq!(counter.coll_count(), 42);
    }

    #[test]
    fn test_custom_context_returns_zero() {
        let ctx = TestCountContext::new(MapBindings::default(), 0);
        let counter = ctx.as_coll_count().expect("should return CollCount");
        assert_eq!(counter.coll_count(), 0);
    }

    #[test]
    fn test_count_pushdown_via_eval_plan() {
        // Full integration: parse + plan + execute SELECT COUNT(u) FROM t u
        // with a TestCountContext that returns 100, even though bindings are empty.
        use crate::plan::EvaluationMode;
        use partiql_catalog::catalog::{PartiqlCatalog, PartiqlSharedCatalog};
        use partiql_logical_planner::LogicalPlanner;
        use partiql_parser::Parser;
        use partiql_value::Bag;

        let parser = Parser::default();
        let shared_catalog = PartiqlCatalog::default().to_shared_catalog();

        // Empty bindings — no rows materialized
        let mut bindings = MapBindings::default();
        bindings.insert("t", Value::Bag(Box::new(Bag::from(vec![]))));

        let parsed = parser.parse("SELECT COUNT(u) FROM t u").unwrap();
        let planner = LogicalPlanner::new(&shared_catalog);
        let logical = planner.lower(&parsed).unwrap();
        let mut eval_planner =
            crate::plan::EvaluatorPlanner::new(EvaluationMode::Permissive, &shared_catalog);
        let eval_plan = eval_planner.compile(&logical).unwrap();

        // Execute with pushdown context — should return 100 from CollCount,
        // NOT 0 from the empty Bag
        let ctx = TestCountContext::new(bindings, 100);
        let result = eval_plan.execute(&ctx).unwrap();

        // Result should be a Bag with one Tuple containing _1: 100
        match result.result {
            Value::Bag(bag) => {
                let values: Vec<_> = bag.iter().collect();
                assert_eq!(values.len(), 1, "Should have one result row");
                if let Value::Tuple(tuple) = &values[0] {
                    // The COUNT alias may be _1 or similar
                    let count_val = tuple.pairs().next().map(|(_, v)| v).unwrap();
                    assert_eq!(
                        count_val,
                        &Value::Integer(100),
                        "CollCount pushdown should return 100, not 0 from empty Bag"
                    );
                } else {
                    panic!("Expected Tuple in result, got {:?}", values[0]);
                }
            }
            other => panic!("Expected Bag result, got {:?}", other),
        }
    }
}

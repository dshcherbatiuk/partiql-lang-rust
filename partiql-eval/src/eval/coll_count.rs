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
        // with a TestCountContext that returns 100 matching the input size.
        use crate::plan::EvaluationMode;
        use partiql_catalog::catalog::PartiqlCatalog;
        use partiql_logical_planner::LogicalPlanner;
        use partiql_parser::Parser;
        use partiql_value::{Bag, Tuple};

        let parser = Parser::default();
        let shared_catalog = PartiqlCatalog::default().to_shared_catalog();

        // Bindings with 100 rows — matching the pushdown count
        let rows: Vec<Value> = (0..100)
            .map(|i| {
                let mut t = Tuple::new();
                t.insert("id", Value::from(i));
                Value::from(t)
            })
            .collect();
        let mut bindings = MapBindings::default();
        bindings.insert("t", Value::Bag(Box::new(Bag::from(rows))));

        let parsed = parser.parse("SELECT COUNT(u) FROM t u").unwrap();
        let planner = LogicalPlanner::new(&shared_catalog);
        let logical = planner.lower(&parsed).unwrap();
        let mut eval_planner =
            crate::plan::EvaluatorPlanner::new(EvaluationMode::Permissive, &shared_catalog);
        let eval_plan = eval_planner.compile(&logical).unwrap();

        // Execute with pushdown context — input_len (100) == storage_count (100),
        // so pushdown fires and returns 100 without iterating
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

    #[test]
    fn test_count_with_where_skips_pushdown() {
        // When bindings have rows (non-empty Bag), pushdown must be skipped
        // because a WHERE filter may have reduced the rows.
        use crate::plan::EvaluationMode;
        use partiql_catalog::catalog::PartiqlCatalog;
        use partiql_logical_planner::LogicalPlanner;
        use partiql_parser::Parser;
        use partiql_value::{Bag, Tuple};

        let parser = Parser::default();
        let shared_catalog = PartiqlCatalog::default().to_shared_catalog();

        // 5 rows: 3 active, 2 inactive
        let rows: Vec<Value> = (0..5)
            .map(|i| {
                let status = if i % 2 == 0 { "active" } else { "inactive" };
                let mut t = Tuple::new();
                t.insert("id", Value::from(i));
                t.insert("status", Value::String(status.to_string().into()));
                Value::from(t)
            })
            .collect();

        let mut bindings = MapBindings::default();
        bindings.insert("t", Value::Bag(Box::new(Bag::from(rows))));

        let parsed = parser
            .parse("SELECT COUNT(u) FROM t u WHERE u.status = 'active'")
            .unwrap();
        let planner = LogicalPlanner::new(&shared_catalog);
        let logical = planner.lower(&parsed).unwrap();
        let mut eval_planner =
            crate::plan::EvaluatorPlanner::new(EvaluationMode::Permissive, &shared_catalog);
        let eval_plan = eval_planner.compile(&logical).unwrap();

        // Context says 1000 — but WHERE filters to 3. Pushdown must be skipped
        // because input Bag is non-empty (5 rows before filter, 3 after).
        let ctx = TestCountContext::new(bindings, 1000);
        let result = eval_plan.execute(&ctx).unwrap();

        match result.result {
            Value::Bag(bag) => {
                let values: Vec<_> = bag.iter().collect();
                assert_eq!(values.len(), 1);
                if let Value::Tuple(tuple) = &values[0] {
                    let count_val = tuple.pairs().next().map(|(_, v)| v).unwrap();
                    assert_eq!(
                        count_val,
                        &Value::Integer(3),
                        "COUNT with WHERE must return 3 (filtered), not 1000 (pushdown)"
                    );
                } else {
                    panic!("Expected Tuple, got {:?}", values[0]);
                }
            }
            other => panic!("Expected Bag, got {:?}", other),
        }
    }

    #[test]
    fn test_count_where_filters_all_rows_must_return_zero() {
        // WHERE filters ALL rows → empty input. Pushdown must NOT fire —
        // should return 0, not the storage count.
        use crate::plan::EvaluationMode;
        use partiql_catalog::catalog::PartiqlCatalog;
        use partiql_logical_planner::LogicalPlanner;
        use partiql_parser::Parser;
        use partiql_value::{Bag, Tuple};

        let parser = Parser::default();
        let shared_catalog = PartiqlCatalog::default().to_shared_catalog();

        // 3 rows, all with type = 'test-event'
        let rows: Vec<Value> = (0..3)
            .map(|i| {
                let mut t = Tuple::new();
                t.insert("event_id", Value::String(format!("evt-{i}").into()));
                t.insert("type", Value::String("test-event".to_string().into()));
                Value::from(t)
            })
            .collect();

        let mut bindings = MapBindings::default();
        bindings.insert("events", Value::Bag(Box::new(Bag::from(rows))));

        // WHERE filters ALL rows (no 'NonExistent' type exists)
        let parsed = parser
            .parse("SELECT VALUE count(1) FROM events e WHERE e.type = 'NonExistent'")
            .unwrap();
        let planner = LogicalPlanner::new(&shared_catalog);
        let logical = planner.lower(&parsed).unwrap();
        let mut eval_planner =
            crate::plan::EvaluatorPlanner::new(EvaluationMode::Permissive, &shared_catalog);
        let eval_plan = eval_planner.compile(&logical).unwrap();

        // Context says 3 (storage has 3 rows). But WHERE removes all → must return 0.
        let ctx = TestCountContext::new(bindings, 3);
        let result = eval_plan.execute(&ctx).unwrap();

        // When WHERE filters all rows, PartiQL returns empty Bag <<>> for SELECT VALUE,
        // not <<0>>. The key assertion: pushdown must NOT return <<3>> (storage count).
        match result.result {
            Value::Bag(bag) => {
                let values: Vec<_> = bag.iter().collect();
                // Either empty (PartiQL default for no groups) or <<0>> — both correct.
                // Must NOT be <<3>> (storage count from pushdown).
                if !values.is_empty() {
                    assert_eq!(
                        values[0],
                        &Value::Integer(0),
                        "If COUNT result present, must be 0, not 3 (storage count)"
                    );
                }
                // Verify pushdown did NOT fire with wrong value
                assert!(
                    values.is_empty() || values[0] == &Value::Integer(0),
                    "Must not return storage count (3) when WHERE filters all rows"
                );
            }
            other => panic!("Expected Bag, got {:?}", other),
        }
    }

    #[test]
    fn test_select_value_count_1_not_affected_by_pushdown() {
        // SELECT VALUE COUNT(1) FROM t goes through COLL_COUNT, not EvalGroupBy.
        // Pushdown must NOT fire — COLL_COUNT counts individual values, not the source.
        use crate::plan::EvaluationMode;
        use partiql_catalog::catalog::PartiqlCatalog;
        use partiql_logical_planner::LogicalPlanner;
        use partiql_parser::Parser;
        use partiql_value::{Bag, Tuple};

        let parser = Parser::default();
        let shared_catalog = PartiqlCatalog::default().to_shared_catalog();

        let rows: Vec<Value> = (0..3)
            .map(|i| {
                let mut t = Tuple::new();
                t.insert("event_id", Value::String(format!("evt-{i}").into()));
                Value::from(t)
            })
            .collect();

        let mut bindings = MapBindings::default();
        bindings.insert("events", Value::Bag(Box::new(Bag::from(rows))));

        let parsed = parser.parse("SELECT VALUE COUNT(1) FROM events").unwrap();
        let planner = LogicalPlanner::new(&shared_catalog);
        let logical = planner.lower(&parsed).unwrap();
        let mut eval_planner =
            crate::plan::EvaluatorPlanner::new(EvaluationMode::Permissive, &shared_catalog);
        let eval_plan = eval_planner.compile(&logical).unwrap();

        // Context says 999 — but COUNT(1) must return 3 (row count from iteration)
        let ctx = TestCountContext::new(bindings, 999);
        let result = eval_plan.execute(&ctx).unwrap();

        match result.result {
            Value::Bag(bag) => {
                let values: Vec<_> = bag.iter().collect();
                assert_eq!(values.len(), 1, "Should have one result");
                assert_eq!(
                    values[0],
                    &Value::Integer(3),
                    "SELECT VALUE COUNT(1) must return 3 (actual rows), not 999 (pushdown)"
                );
            }
            other => panic!("Expected Bag, got {:?}", other),
        }
    }
}

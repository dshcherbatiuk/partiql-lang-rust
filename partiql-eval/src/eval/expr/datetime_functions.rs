use crate::eval::expr::{BindError, BindEvalExpr, EvalExpr};
use crate::eval::EvalContext;
use partiql_value::{DateTime, Value};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct EvalFnCurrentTime;

impl BindEvalExpr for EvalFnCurrentTime {
    fn bind<const STRICT: bool>(
        self,
        args: Vec<Box<dyn EvalExpr>>,
    ) -> Result<Box<dyn EvalExpr>, BindError> {
        if !args.is_empty() {
            return Err(BindError::ArgNumMismatch {
                expected: vec![0],
                found: args.len(),
            });
        }
        Ok(Box::new(EvalExprCurrentTime))
    }
}

#[derive(Debug)]
struct EvalExprCurrentTime;

impl EvalExpr for EvalExprCurrentTime {
    fn evaluate<'a, 'c, 'o>(
        &'a self,
        _bindings: &'a dyn partiql_value::datum::RefTupleView<'a, Value>,
        ctx: &'c dyn EvalContext,
    ) -> Cow<'o, Value>
    where
        'c: 'a,
        'a: 'o,
    {
        // Extract time component from SystemContext.now
        let time_value = match &ctx.system_context().now {
            DateTime::Time(t) => DateTime::Time(*t),
            DateTime::TimeWithTz(t, tz) => DateTime::TimeWithTz(*t, *tz),
            DateTime::Timestamp(ts) => DateTime::Time(ts.time()),
            DateTime::TimestampWithTz(ts) => DateTime::TimeWithTz(ts.time(), ts.offset()),
            DateTime::Date(_) => return Cow::Owned(Value::Missing),
        };
        Cow::Owned(Value::DateTime(Box::new(time_value)))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct EvalFnCurrentTimestamp;

impl BindEvalExpr for EvalFnCurrentTimestamp {
    fn bind<const STRICT: bool>(
        self,
        args: Vec<Box<dyn EvalExpr>>,
    ) -> Result<Box<dyn EvalExpr>, BindError> {
        if !args.is_empty() {
            return Err(BindError::ArgNumMismatch {
                expected: vec![0],
                found: args.len(),
            });
        }
        Ok(Box::new(EvalExprCurrentTimestamp))
    }
}

#[derive(Debug)]
struct EvalExprCurrentTimestamp;

impl EvalExpr for EvalExprCurrentTimestamp {
    fn evaluate<'a, 'c, 'o>(
        &'a self,
        _bindings: &'a dyn partiql_value::datum::RefTupleView<'a, Value>,
        ctx: &'c dyn EvalContext,
    ) -> Cow<'o, Value>
    where
        'c: 'a,
        'a: 'o,
    {
        // Return the full timestamp from SystemContext.now (with timezone)
        Cow::Owned(Value::DateTime(Box::new(ctx.system_context().now.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::BasicContext;
    use partiql_catalog::context::SystemContext;
    use partiql_value::Tuple;

    #[test]
    fn test_current_time_evaluation() {
        use crate::env::basic::MapBindings;
        use partiql_value::datum::DatumTupleRef;
        let now = DateTime::from_system_now_utc();
        let sys_ctx = SystemContext { now: now.clone() };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let expr = EvalExprCurrentTime;
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        let result = expr.evaluate(&binding, &ctx);

        match result.as_ref() {
            Value::DateTime(dt) => match dt.as_ref() {
                DateTime::TimeWithTz(_, _) => {} // Expected
                other => panic!("Expected TimeWithTz, got {:?}", other),
            },
            other => panic!("Expected DateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_current_time_no_args() {
        let result = EvalFnCurrentTime.bind::<true>(vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_current_time_with_args_fails() {
        use crate::eval::expr::EvalLitExpr;
        let arg = EvalLitExpr::new(Value::Null).bind::<true>(vec![]).unwrap();
        let result = EvalFnCurrentTime.bind::<true>(vec![arg]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }

    #[test]
    fn test_current_timestamp_evaluation() {
        use crate::env::basic::MapBindings;
        use partiql_value::datum::DatumTupleRef;
        let now = DateTime::from_system_now_utc();
        let sys_ctx = SystemContext { now: now.clone() };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let expr = EvalExprCurrentTimestamp;
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        let result = expr.evaluate(&binding, &ctx);

        match result.as_ref() {
            Value::DateTime(dt) => match dt.as_ref() {
                DateTime::TimestampWithTz(_) => {} // Expected
                other => panic!("Expected TimestampWithTz, got {:?}", other),
            },
            other => panic!("Expected DateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_current_timestamp_no_args() {
        let result = EvalFnCurrentTimestamp.bind::<true>(vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_current_timestamp_with_args_fails() {
        use crate::eval::expr::EvalLitExpr;
        let arg = EvalLitExpr::new(Value::Null).bind::<true>(vec![]).unwrap();
        let result = EvalFnCurrentTimestamp.bind::<true>(vec![arg]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }
}

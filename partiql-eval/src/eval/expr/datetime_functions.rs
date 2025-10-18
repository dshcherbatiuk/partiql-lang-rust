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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct EvalFnUnixTimestamp;

impl BindEvalExpr for EvalFnUnixTimestamp {
    fn bind<const STRICT: bool>(
        self,
        args: Vec<Box<dyn EvalExpr>>,
    ) -> Result<Box<dyn EvalExpr>, BindError> {
        match args.len() {
            0 => Ok(Box::new(EvalExprUnixTimestampNoArgs)),
            1 => Ok(Box::new(EvalExprUnixTimestampWithArgs {
                arg: args.into_iter().next().unwrap(),
            })),
            _ => Err(BindError::ArgNumMismatch {
                expected: vec![0, 1],
                found: args.len(),
            }),
        }
    }
}

#[derive(Debug)]
struct EvalExprUnixTimestampNoArgs;

impl EvalExpr for EvalExprUnixTimestampNoArgs {
    fn evaluate<'a, 'c, 'o>(
        &'a self,
        _bindings: &'a dyn partiql_value::datum::RefTupleView<'a, Value>,
        ctx: &'c dyn EvalContext,
    ) -> Cow<'o, Value>
    where
        'c: 'a,
        'a: 'o,
    {
        // Get current timestamp from SystemContext and convert to Unix timestamp
        let unix_ts = datetime_to_unix_timestamp(&ctx.system_context().now);
        Cow::Owned(unix_ts)
    }
}

#[derive(Debug)]
struct EvalExprUnixTimestampWithArgs {
    arg: Box<dyn EvalExpr>,
}

impl EvalExpr for EvalExprUnixTimestampWithArgs {
    fn evaluate<'a, 'c, 'o>(
        &'a self,
        bindings: &'a dyn partiql_value::datum::RefTupleView<'a, Value>,
        ctx: &'c dyn EvalContext,
    ) -> Cow<'o, Value>
    where
        'c: 'a,
        'a: 'o,
    {
        use partiql_value::datum::DatumLower;

        // Evaluate the argument
        let arg_value = self.arg.evaluate(bindings, ctx);

        // Extract DateTime from the argument
        match arg_value.as_ref() {
            Value::DateTime(dt) => {
                let unix_ts = datetime_to_unix_timestamp(dt);
                Cow::Owned(unix_ts)
            }
            Value::Variant(variant) => {
                // Try to lower the Variant to extract the actual value (e.g., Ion timestamp -> DateTime)
                match variant.lower() {
                    Ok(lowered_value) => {
                        // Recursively handle the lowered value
                        match lowered_value.as_ref() {
                            Value::DateTime(dt) => {
                                let unix_ts = datetime_to_unix_timestamp(dt);
                                Cow::Owned(unix_ts)
                            }
                            _ => Cow::Owned(Value::Missing),
                        }
                    }
                    Err(_) => Cow::Owned(Value::Missing),
                }
            }
            _ => Cow::Owned(Value::Missing),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct EvalFnFromUnixtime;

impl BindEvalExpr for EvalFnFromUnixtime {
    fn bind<const STRICT: bool>(
        self,
        args: Vec<Box<dyn EvalExpr>>,
    ) -> Result<Box<dyn EvalExpr>, BindError> {
        if args.len() != 1 {
            return Err(BindError::ArgNumMismatch {
                expected: vec![1],
                found: args.len(),
            });
        }
        Ok(Box::new(EvalExprFromUnixtime {
            arg: args.into_iter().next().unwrap(),
        }))
    }
}

#[derive(Debug)]
struct EvalExprFromUnixtime {
    arg: Box<dyn EvalExpr>,
}

impl EvalExpr for EvalExprFromUnixtime {
    fn evaluate<'a, 'c, 'o>(
        &'a self,
        bindings: &'a dyn partiql_value::datum::RefTupleView<'a, Value>,
        ctx: &'c dyn EvalContext,
    ) -> Cow<'o, Value>
    where
        'c: 'a,
        'a: 'o,
    {
        // Evaluate the argument
        let arg_value = self.arg.evaluate(bindings, ctx);

        // Convert Unix timestamp (Integer or Decimal) to DateTime
        match arg_value.as_ref() {
            Value::Integer(seconds) => {
                let dt = unix_timestamp_to_datetime(*seconds, 0);
                Cow::Owned(Value::DateTime(Box::new(dt)))
            }
            Value::Decimal(dec) => {
                // Convert decimal to total nanoseconds, then split into seconds and nanos
                use rust_decimal::prelude::*;

                // Convert to nanoseconds (may overflow for very large values)
                let total_nanos = **dec * Decimal::from(1_000_000_000);
                let total_nanos_i128 = total_nanos.to_i128().unwrap_or(0);

                // Split into seconds and nanoseconds using Euclidean division
                // This ensures nanos is always in [0, 1_000_000_000) range
                let seconds = total_nanos_i128.div_euclid(1_000_000_000);
                let nanos = total_nanos_i128.rem_euclid(1_000_000_000) as u32;

                let dt = unix_timestamp_to_datetime(seconds as i64, nanos);
                Cow::Owned(Value::DateTime(Box::new(dt)))
            }
            _ => Cow::Owned(Value::Missing),
        }
    }
}

/// Converts Unix timestamp (seconds since epoch) to DateTime
/// Returns TimestampWithTz in UTC (offset 0)
fn unix_timestamp_to_datetime(seconds: i64, nanos: u32) -> DateTime {
    // Create OffsetDateTime from Unix timestamp
    let offset_dt = time::OffsetDateTime::from_unix_timestamp_nanos(
        (seconds as i128) * 1_000_000_000 + (nanos as i128),
    )
    .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);

    // Wrap in DateTime::TimestampWithTz
    DateTime::TimestampWithTz(offset_dt)
}

/// Converts a DateTime to Unix timestamp (seconds since epoch)
/// Returns Integer if no fractional seconds, Decimal otherwise
fn datetime_to_unix_timestamp(dt: &DateTime) -> Value {
    match dt {
        DateTime::TimestampWithTz(ts) => {
            let seconds = ts.unix_timestamp();
            let nanos = ts.nanosecond();

            if nanos == 0 {
                // No fractional seconds, return Integer
                Value::Integer(seconds)
            } else {
                // Has fractional seconds, return Decimal
                use rust_decimal::prelude::*;
                let decimal_seconds = Decimal::from(seconds) + Decimal::new(nanos as i64, 9);
                // Normalize to remove trailing zeros (e.g., 1577836800.100000000 -> 1577836800.1)
                Value::Decimal(Box::new(decimal_seconds.normalize()))
            }
        }
        DateTime::Timestamp(ts) => {
            // Assume UTC for timestamps without timezone
            let offset_ts = ts.assume_utc();
            let seconds = offset_ts.unix_timestamp();
            let nanos = offset_ts.nanosecond();

            if nanos == 0 {
                Value::Integer(seconds)
            } else {
                use rust_decimal::prelude::*;
                let decimal_seconds = Decimal::from(seconds) + Decimal::new(nanos as i64, 9);
                // Normalize to remove trailing zeros
                Value::Decimal(Box::new(decimal_seconds.normalize()))
            }
        }
        DateTime::Date(date) => {
            // Convert date to timestamp at midnight UTC
            let ts = date.midnight().assume_utc();
            Value::Integer(ts.unix_timestamp())
        }
        _ => Value::Missing, // Time and TimeWithTz don't have a date component
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

    #[test]
    fn test_unix_timestamp_no_args() {
        use crate::env::basic::MapBindings;
        use partiql_value::datum::DatumTupleRef;
        let now = DateTime::from_system_now_utc();
        let sys_ctx = SystemContext { now: now.clone() };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let expr = EvalExprUnixTimestampNoArgs;
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        let result = expr.evaluate(&binding, &ctx);

        // Should return an Integer or Decimal (depending on fractional seconds)
        match result.as_ref() {
            Value::Integer(_) | Value::Decimal(_) => {} // Either is valid
            other => panic!("Expected Integer or Decimal, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_binding_no_args() {
        let result = EvalFnUnixTimestamp.bind::<true>(vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unix_timestamp_binding_one_arg() {
        use crate::eval::expr::EvalLitExpr;
        let now = DateTime::from_system_now_utc();
        let arg = EvalLitExpr::new(Value::DateTime(Box::new(now)))
            .bind::<true>(vec![])
            .unwrap();
        let result = EvalFnUnixTimestamp.bind::<true>(vec![arg]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unix_timestamp_with_too_many_args_fails() {
        use crate::eval::expr::EvalLitExpr;
        let arg1 = EvalLitExpr::new(Value::Null).bind::<true>(vec![]).unwrap();
        let arg2 = EvalLitExpr::new(Value::Null).bind::<true>(vec![]).unwrap();
        let result = EvalFnUnixTimestamp.bind::<true>(vec![arg1, arg2]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }

    #[test]
    fn test_unix_timestamp_conversion() {
        use std::num::NonZeroU8;

        // Test timestamp without fractional seconds (2020-01-01T00:00:00Z)
        let dt = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            0,
            Some(0), // UTC
        );
        let result = datetime_to_unix_timestamp(&dt);

        // 2020-01-01T00:00:00Z = 1577836800 seconds since epoch
        match result {
            Value::Integer(ts) => assert_eq!(ts, 1577836800),
            other => panic!(
                "Expected Integer for timestamp without fractional seconds, got {:?}",
                other
            ),
        }

        // Test timestamp with fractional seconds (2020-01-01T00:00:00.1Z)
        let dt_with_fraction = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            100_000_000, // 0.1 seconds in nanoseconds
            Some(0),     // UTC
        );
        let result_with_fraction = datetime_to_unix_timestamp(&dt_with_fraction);

        match result_with_fraction {
            Value::Decimal(_) => {} // Expected Decimal for fractional seconds
            other => panic!(
                "Expected Decimal for timestamp with fractional seconds, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_unix_timestamp_before_epoch() {
        use std::num::NonZeroU8;

        // Test timestamp before epoch (1969-01-01T00:00:00Z)
        let dt = DateTime::from_ymdhms_nano_offset_minutes(
            1969,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            0,
            Some(0), // UTC
        );
        let result = datetime_to_unix_timestamp(&dt);

        // Should return negative value (-31536000 for 1969-01-01)
        match result {
            Value::Integer(ts) => {
                assert!(ts < 0);
                assert_eq!(ts, -31536000);
            }
            other => panic!(
                "Expected negative Integer for timestamp before epoch, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_unix_timestamp_date_type() {
        use std::num::NonZeroU8;

        // Test Date type (2020-01-01)
        let dt = DateTime::from_ymd(2020, NonZeroU8::new(1).unwrap(), 1);
        let result = datetime_to_unix_timestamp(&dt);

        // Date should convert to midnight UTC
        match result {
            Value::Integer(ts) => assert_eq!(ts, 1577836800),
            other => panic!("Expected Integer for Date type, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_timestamp_without_tz() {
        use std::num::NonZeroU8;

        // Test Timestamp without timezone (assumes UTC)
        // Create using from_ymdhms_nano_offset_minutes with None for offset
        let dt = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            0,
            None, // No timezone offset
        );
        let result = datetime_to_unix_timestamp(&dt);

        // Should assume UTC
        match result {
            Value::Integer(ts) => assert_eq!(ts, 1577836800),
            other => panic!("Expected Integer for Timestamp without TZ, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_time_types_return_missing() {
        // Test Time type (no date component)
        let dt = DateTime::from_hms(12, 30, 45);
        let result = datetime_to_unix_timestamp(&dt);

        match result {
            Value::Missing => {} // Expected
            other => panic!("Expected Missing for Time type, got {:?}", other),
        }

        // Test TimeWithTz type (no date component)
        let dt_tz = DateTime::from_hms_nano_tz(12, 30, 45, 0, Some(0), Some(0));
        let result_tz = datetime_to_unix_timestamp(&dt_tz);

        match result_tz {
            Value::Missing => {} // Expected
            other => panic!("Expected Missing for TimeWithTz type, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_decimal_precision() {
        use rust_decimal::prelude::*;
        use std::num::NonZeroU8;

        // Test various fractional second precisions

        // 100 milliseconds (0.1 seconds)
        let dt1 = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            100_000_000,
            Some(0),
        );
        let result1 = datetime_to_unix_timestamp(&dt1);
        match result1 {
            Value::Decimal(dec) => {
                let expected = Decimal::from(1577836800) + Decimal::new(1, 1); // 1577836800.1
                assert_eq!(*dec, expected);
            }
            other => panic!("Expected Decimal, got {:?}", other),
        }

        // 1 nanosecond precision
        let dt2 = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            1, // 1 nanosecond
            Some(0),
        );
        let result2 = datetime_to_unix_timestamp(&dt2);
        match result2 {
            Value::Decimal(dec) => {
                // Should preserve nanosecond precision
                let expected = Decimal::from(1577836800) + Decimal::new(1, 9);
                assert_eq!(*dec, expected);
            }
            other => panic!("Expected Decimal for nanosecond precision, got {:?}", other),
        }

        // 999,999,999 nanoseconds (just under 1 second)
        let dt3 = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            999_999_999,
            Some(0),
        );
        let result3 = datetime_to_unix_timestamp(&dt3);
        match result3 {
            Value::Decimal(dec) => {
                let expected = Decimal::from(1577836800) + Decimal::new(999_999_999, 9);
                assert_eq!(*dec, expected);
            }
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_with_args_evaluation() {
        use crate::env::basic::MapBindings;
        use crate::eval::expr::EvalLitExpr;
        use partiql_value::datum::DatumTupleRef;
        use std::num::NonZeroU8;

        // Create a timestamp literal
        let dt = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            0,
            Some(0),
        );

        // Create the evaluation context
        let sys_ctx = SystemContext {
            now: DateTime::from_system_now_utc(),
        };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        // Create the expression with DateTime argument
        let arg_expr = EvalLitExpr::new(Value::DateTime(Box::new(dt)));
        let expr = EvalExprUnixTimestampWithArgs {
            arg: Box::new(arg_expr),
        };

        // Evaluate
        let result = expr.evaluate(&binding, &ctx);

        // Should return Integer 1577836800
        match result.as_ref() {
            Value::Integer(ts) => assert_eq!(*ts, 1577836800),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_with_non_datetime_returns_missing() {
        use crate::env::basic::MapBindings;
        use crate::eval::expr::EvalLitExpr;
        use partiql_value::datum::DatumTupleRef;

        // Create the evaluation context
        let sys_ctx = SystemContext {
            now: DateTime::from_system_now_utc(),
        };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        // Test with String argument
        let arg_expr = EvalLitExpr::new(Value::String(Box::new("not a timestamp".to_string())));
        let expr = EvalExprUnixTimestampWithArgs {
            arg: Box::new(arg_expr),
        };

        let result = expr.evaluate(&binding, &ctx);
        match result.as_ref() {
            Value::Missing => {} // Expected
            other => panic!(
                "Expected Missing for non-DateTime argument, got {:?}",
                other
            ),
        }

        // Test with Integer argument
        let arg_expr2 = EvalLitExpr::new(Value::Integer(42));
        let expr2 = EvalExprUnixTimestampWithArgs {
            arg: Box::new(arg_expr2),
        };

        let result2 = expr2.evaluate(&binding, &ctx);
        match result2.as_ref() {
            Value::Missing => {} // Expected
            other => panic!("Expected Missing for Integer argument, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_with_ion_variant() {
        use crate::env::basic::MapBindings;
        use crate::eval::expr::EvalLitExpr;
        use partiql_extension_ion::boxed_ion::BoxedIonType;
        use partiql_value::boxed_variant::DynBoxedVariantTypeFactory;
        use partiql_value::datum::DatumTupleRef;
        use partiql_value::Variant;

        // Create an Ion timestamp variant for `2020T`
        let ion_text = "2020T";
        let ion_typ = BoxedIonType::default().to_dyn_type_tag();
        let variant = Variant::new(ion_text, ion_typ).expect("Ion variant creation");

        // Create the evaluation context
        let sys_ctx = SystemContext {
            now: DateTime::from_system_now_utc(),
        };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        // Create the expression with Variant argument
        let arg_expr = EvalLitExpr::new(Value::Variant(Box::new(variant)));
        let expr = EvalExprUnixTimestampWithArgs {
            arg: Box::new(arg_expr),
        };

        // Evaluate
        let result = expr.evaluate(&binding, &ctx);

        // Should return Integer for year 2020 (2020-01-01T00:00:00Z = 1577836800)
        match result.as_ref() {
            Value::Integer(ts) => assert_eq!(*ts, 1577836800),
            other => panic!(
                "Expected Integer for Ion timestamp `2020T`, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_unix_timestamp_timezone_handling() {
        use std::num::NonZeroU8;

        // Test timestamp with positive timezone offset (UTC+5:30)
        let dt_plus = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            5, // 5:30 AM in UTC+5:30
            30,
            0,
            0,
            Some(330), // +5:30 in minutes
        );
        let result_plus = datetime_to_unix_timestamp(&dt_plus);

        // Unix timestamp should account for timezone
        match result_plus {
            Value::Integer(ts) => {
                // 5:30 AM UTC+5:30 = midnight UTC
                assert_eq!(ts, 1577836800);
            }
            other => panic!("Expected Integer, got {:?}", other),
        }

        // Test timestamp with negative timezone offset (UTC-8)
        let dt_minus = DateTime::from_ymdhms_nano_offset_minutes(
            2019,
            NonZeroU8::new(12).unwrap(),
            31,
            16, // 4 PM on Dec 31
            0,
            0,
            0,
            Some(-480), // -8 hours in minutes
        );
        let result_minus = datetime_to_unix_timestamp(&dt_minus);

        match result_minus {
            Value::Integer(ts) => {
                // 4 PM Dec 31 UTC-8 = midnight Jan 1 UTC
                assert_eq!(ts, 1577836800);
            }
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn test_unix_timestamp_2024_june_15() {
        use std::num::NonZeroU8;

        // Test the specific timestamp from failing component test: 2024-06-15T12:30:45Z
        let dt = DateTime::from_ymdhms_nano_offset_minutes(
            2024,
            NonZeroU8::new(6).unwrap(),
            15,
            12,
            30,
            45,
            0,
            Some(0), // UTC
        );
        let result = datetime_to_unix_timestamp(&dt);

        match result {
            Value::Integer(ts) => {
                println!("Unix timestamp for 2024-06-15T12:30:45Z: {}", ts);
                // This will show us what the correct value should be
            }
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    // FROM_UNIXTIME tests
    #[test]
    fn test_from_unixtime_zero() {
        let dt = unix_timestamp_to_datetime(0, 0);
        // Should be 1970-01-01T00:00:00Z
        match dt {
            DateTime::TimestampWithTz(ts) => {
                assert_eq!(ts.unix_timestamp(), 0);
                assert_eq!(ts.nanosecond(), 0);
            }
            other => panic!("Expected TimestampWithTz, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_positive() {
        // FROM_UNIXTIME(1577836800) -> 2020-01-01T00:00:00Z
        let dt = unix_timestamp_to_datetime(1577836800, 0);
        match dt {
            DateTime::TimestampWithTz(ts) => {
                assert_eq!(ts.unix_timestamp(), 1577836800);
                assert_eq!(ts.nanosecond(), 0);
            }
            other => panic!("Expected TimestampWithTz, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_negative() {
        // FROM_UNIXTIME(-1) -> 1969-12-31T23:59:59Z
        let dt = unix_timestamp_to_datetime(-1, 0);
        match dt {
            DateTime::TimestampWithTz(ts) => {
                assert_eq!(ts.unix_timestamp(), -1);
                assert_eq!(ts.nanosecond(), 0);
            }
            other => panic!("Expected TimestampWithTz, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_with_fractional_seconds() {
        // FROM_UNIXTIME(0.1) -> 1970-01-01T00:00:00.1Z
        let dt = unix_timestamp_to_datetime(0, 100_000_000); // 0.1 seconds in nanoseconds
        match dt {
            DateTime::TimestampWithTz(ts) => {
                assert_eq!(ts.unix_timestamp(), 0);
                assert_eq!(ts.nanosecond(), 100_000_000);
            }
            other => panic!("Expected TimestampWithTz, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_with_milliseconds() {
        // FROM_UNIXTIME(0.001) -> 1970-01-01T00:00:00.001Z
        let dt = unix_timestamp_to_datetime(0, 1_000_000); // 0.001 seconds in nanoseconds
        match dt {
            DateTime::TimestampWithTz(ts) => {
                assert_eq!(ts.unix_timestamp(), 0);
                assert_eq!(ts.nanosecond(), 1_000_000);
            }
            other => panic!("Expected TimestampWithTz, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_negative_with_fractional() {
        // FROM_UNIXTIME(-0.1) -> 1969-12-31T23:59:59.9Z
        // -0.1 seconds = -1 second + 900,000,000 nanoseconds
        let dt = unix_timestamp_to_datetime(-1, 900_000_000);
        match dt {
            DateTime::TimestampWithTz(ts) => {
                // The timestamp should be -1 second with 900ms nanoseconds
                assert_eq!(ts.unix_timestamp(), -1);
                assert_eq!(ts.nanosecond(), 900_000_000);
            }
            other => panic!("Expected TimestampWithTz, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_function_binding() {
        let result = EvalFnFromUnixtime.bind::<true>(vec![]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));

        use crate::eval::expr::EvalLitExpr;
        let arg = EvalLitExpr::new(Value::Integer(0))
            .bind::<true>(vec![])
            .unwrap();
        let result = EvalFnFromUnixtime.bind::<true>(vec![arg]);
        assert!(result.is_ok());

        let arg1 = EvalLitExpr::new(Value::Integer(0))
            .bind::<true>(vec![])
            .unwrap();
        let arg2 = EvalLitExpr::new(Value::Integer(1))
            .bind::<true>(vec![])
            .unwrap();
        let result = EvalFnFromUnixtime.bind::<true>(vec![arg1, arg2]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }

    #[test]
    fn test_from_unixtime_evaluation_integer() {
        use crate::env::basic::MapBindings;
        use crate::eval::expr::EvalLitExpr;
        use partiql_catalog::context::SystemContext;
        use partiql_value::datum::DatumTupleRef;
        use partiql_value::Tuple;

        // Create the evaluation context
        let sys_ctx = SystemContext {
            now: DateTime::from_system_now_utc(),
        };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        // Create the expression with Integer argument (1577836800 = 2020-01-01T00:00:00Z)
        let arg_expr = EvalLitExpr::new(Value::Integer(1577836800));
        let expr = EvalExprFromUnixtime {
            arg: Box::new(arg_expr),
        };

        // Evaluate
        let result = expr.evaluate(&binding, &ctx);

        // Should return DateTime
        match result.as_ref() {
            Value::DateTime(dt) => match dt.as_ref() {
                DateTime::TimestampWithTz(ts) => {
                    assert_eq!(ts.unix_timestamp(), 1577836800);
                    assert_eq!(ts.nanosecond(), 0);
                }
                other => panic!("Expected TimestampWithTz, got {:?}", other),
            },
            other => panic!("Expected DateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_from_unixtime_round_trip() {
        use std::num::NonZeroU8;

        // Test round-trip: DateTime -> Unix timestamp -> DateTime
        let original_dt = DateTime::from_ymdhms_nano_offset_minutes(
            2020,
            NonZeroU8::new(1).unwrap(),
            1,
            0,
            0,
            0,
            0,
            Some(0), // UTC
        );

        // Convert to Unix timestamp
        let unix_ts = datetime_to_unix_timestamp(&original_dt);

        // Convert back to DateTime
        let seconds = match unix_ts {
            Value::Integer(s) => s,
            _ => panic!("Expected Integer"),
        };
        let recovered_dt = unix_timestamp_to_datetime(seconds, 0);

        // Verify round-trip
        match (&original_dt, &recovered_dt) {
            (DateTime::TimestampWithTz(ts1), DateTime::TimestampWithTz(ts2)) => {
                assert_eq!(ts1.unix_timestamp(), ts2.unix_timestamp());
                assert_eq!(ts1.nanosecond(), ts2.nanosecond());
            }
            (DateTime::Timestamp(ts1), DateTime::TimestampWithTz(ts2)) => {
                let ts1_utc = ts1.assume_utc();
                assert_eq!(ts1_utc.unix_timestamp(), ts2.unix_timestamp());
                assert_eq!(ts1_utc.nanosecond(), ts2.nanosecond());
            }
            _ => panic!("Unexpected DateTime types"),
        }
    }
}

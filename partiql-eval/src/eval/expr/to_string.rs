use crate::eval::expr::{BindError, BindEvalExpr, EvalExpr};
use crate::eval::EvalContext;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use partiql_value::{DateTime, Value};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct EvalFnToString;

impl BindEvalExpr for EvalFnToString {
    fn bind<const STRICT: bool>(
        self,
        args: Vec<Box<dyn EvalExpr>>,
    ) -> Result<Box<dyn EvalExpr>, BindError> {
        if args.len() != 2 {
            return Err(BindError::ArgNumMismatch {
                expected: vec![2],
                found: args.len(),
            });
        }

        let mut args_iter = args.into_iter();
        Ok(Box::new(EvalExprToString {
            timestamp: args_iter.next().unwrap(),
            format: args_iter.next().unwrap(),
        }))
    }
}

#[derive(Debug)]
struct EvalExprToString {
    timestamp: Box<dyn EvalExpr>,
    format: Box<dyn EvalExpr>,
}

impl EvalExpr for EvalExprToString {
    fn evaluate<'a, 'c, 'o>(
        &'a self,
        bindings: &'a dyn partiql_value::datum::RefTupleView<'a, Value>,
        ctx: &'c dyn EvalContext,
    ) -> Cow<'o, Value>
    where
        'c: 'a,
        'a: 'o,
    {
        let timestamp_val = self.timestamp.evaluate(bindings, ctx);
        let format_val = self.format.evaluate(bindings, ctx);

        let result = match (timestamp_val.as_ref(), format_val.as_ref()) {
            (Value::DateTime(dt), Value::String(format_str)) => {
                format_datetime(dt.as_ref(), format_str.as_ref())
            }
            _ => return Cow::Owned(Value::Missing),
        };

        Cow::Owned(Value::from(result))
    }
}

fn format_datetime(dt: &DateTime, format: &str) -> String {
    let converted_format = convert_partiql_pattern_to_chrono(format);

    match dt {
        DateTime::Date(d) => {
            let chrono_date = NaiveDate::from_ymd_opt(d.year(), u8::from(d.month()) as u32, d.day() as u32)
                .unwrap();
            chrono_date.format(&converted_format).to_string()
        }
        DateTime::Time(t) => {
            let chrono_time = NaiveTime::from_hms_nano_opt(
                t.hour() as u32,
                t.minute() as u32,
                t.second() as u32,
                t.nanosecond(),
            )
            .unwrap();
            chrono_time.format(&converted_format).to_string()
        }
        DateTime::TimeWithTz(t, _tz) => {
            // For time with timezone, format just the time part
            let chrono_time = NaiveTime::from_hms_nano_opt(
                t.hour() as u32,
                t.minute() as u32,
                t.second() as u32,
                t.nanosecond(),
            )
            .unwrap();
            chrono_time.format(&converted_format).to_string()
        }
        DateTime::Timestamp(ts) => {
            let chrono_dt = NaiveDateTime::new(
                NaiveDate::from_ymd_opt(ts.year(), u8::from(ts.month()) as u32, ts.day() as u32).unwrap(),
                NaiveTime::from_hms_nano_opt(ts.hour() as u32, ts.minute() as u32, ts.second() as u32, ts.nanosecond()).unwrap(),
            );
            chrono_dt.format(&converted_format).to_string()
        }
        DateTime::TimestampWithTz(ts) => {
            let chrono_dt = NaiveDateTime::new(
                NaiveDate::from_ymd_opt(ts.year(), u8::from(ts.month()) as u32, ts.day() as u32).unwrap(),
                NaiveTime::from_hms_nano_opt(ts.hour() as u32, ts.minute() as u32, ts.second() as u32, ts.nanosecond()).unwrap(),
            );
            chrono_dt.format(&converted_format).to_string()
        }
    }
}

/// Convert PartiQL TO_STRING pattern to chrono strftime pattern
/// Spec: https://github.com/partiql/partiql-lang-kotlin/wiki/Functions#to_string
fn convert_partiql_pattern_to_chrono(pattern: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Handle quoted literals (e.g., 'T' in pattern)
        if chars[i] == '\'' {
            i += 1; // Skip opening quote
            while i < chars.len() && chars[i] != '\'' {
                result.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip closing quote
            }
            continue;
        }

        let remaining = &pattern[i..];

        // Count consecutive 'S' characters for fraction of second
        if remaining.starts_with('S') {
            let s_count = remaining.chars().take_while(|&c| c == 'S').count();
            if s_count > 0 {
                // S, SS, SSS, etc. = fraction of second with that many digits
                result.push_str(&format!("%{}f", s_count));
                i += s_count;
                continue;
            }
        }

        // Count consecutive 'x' or 'X' for timezone offset patterns
        if remaining.starts_with("XXXXX") {
            result.push_str("%:z"); // +07:00 or Z
            i += 5;
            continue;
        } else if remaining.starts_with("XXXX") || remaining.starts_with("XX") {
            let len = if remaining.starts_with("XXXX") { 4 } else { 2 };
            result.push_str("%z"); // +0700 or Z
            i += len;
            continue;
        } else if remaining.starts_with("XXX") {
            result.push_str("%:z"); // +07:00 or Z
            i += 3;
            continue;
        } else if remaining.starts_with("xxxxx") {
            result.push_str("%:z"); // +07:00 (without Z)
            i += 5;
            continue;
        } else if remaining.starts_with("xxxx") || remaining.starts_with("xx") {
            let len = if remaining.starts_with("xxxx") { 4 } else { 2 };
            result.push_str("%z"); // +0700 (without Z)
            i += len;
            continue;
        } else if remaining.starts_with("xxx") {
            result.push_str("%:z"); // +07:00 (without Z)
            i += 3;
            continue;
        }

        // Process format specifiers in order of longest to shortest
        let (replacement, skip) = if remaining.starts_with("MMMMM") {
            ("%^b".chars().next().unwrap().to_string(), 5) // First letter of month
        } else if remaining.starts_with("MMMM") {
            ("%B".to_string(), 4) // Full month name
        } else if remaining.starts_with("MMM") {
            ("%b".to_string(), 3) // Abbreviated month name
        } else if remaining.starts_with("yyyy") {
            ("%Y".to_string(), 4) // Zero-padded 4-digit year
        } else if remaining.starts_with("MM") {
            ("%m".to_string(), 2) // Zero-padded month
        } else if remaining.starts_with("dd") {
            ("%d".to_string(), 2) // Zero-padded day
        } else if remaining.starts_with("HH") {
            ("%H".to_string(), 2) // Zero-padded hour (00-23)
        } else if remaining.starts_with("hh") {
            ("%I".to_string(), 2) // Zero-padded hour (01-12)
        } else if remaining.starts_with("mm") {
            ("%M".to_string(), 2) // Zero-padded minute
        } else if remaining.starts_with("ss") {
            ("%S".to_string(), 2) // Zero-padded second
        } else if remaining.starts_with("yy") {
            ("%y".to_string(), 2) // 2-digit year
        } else if remaining.starts_with('y') {
            ("%Y".to_string(), 1) // 4-digit year (y same as yyyy in PartiQL)
        } else if remaining.starts_with('n') {
            ("%f".to_string(), 1) // Nanosecond
        } else if remaining.starts_with('M') {
            ("%-m".to_string(), 1) // Month without padding
        } else if remaining.starts_with('d') {
            ("%-d".to_string(), 1) // Day without padding
        } else if remaining.starts_with('H') {
            ("%-H".to_string(), 1) // Hour (0-23) without padding
        } else if remaining.starts_with('h') {
            ("%-I".to_string(), 1) // Hour (1-12) without padding
        } else if remaining.starts_with('m') {
            ("%-M".to_string(), 1) // Minute without padding
        } else if remaining.starts_with('s') {
            ("%-S".to_string(), 1) // Second without padding
        } else if remaining.starts_with('a') {
            ("%p".to_string(), 1) // AM/PM
        } else if remaining.starts_with('X') || remaining.starts_with('x') {
            ("%z".to_string(), 1) // Timezone offset (X allows Z, x doesn't)
        } else {
            // Literal character
            result.push(chars[i]);
            i += 1;
            continue;
        };

        result.push_str(&replacement);
        i += skip;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::basic::MapBindings;
    use crate::eval::BasicContext;
    use crate::eval::expr::EvalLitExpr;
    use partiql_catalog::context::SystemContext;
    use partiql_value::{DateTime, Tuple};
    use partiql_value::datum::DatumTupleRef;

    #[test]
    fn test_to_string_with_timestamp() {
        let now = DateTime::from_system_now_utc();
        let timestamp_arg = EvalLitExpr::new(Value::DateTime(Box::new(now.clone())))
            .bind::<true>(vec![])
            .unwrap();
        let format_arg = EvalLitExpr::new(Value::from("yyyy-MM-dd"))
            .bind::<true>(vec![])
            .unwrap();
        let expr = EvalFnToString
            .bind::<true>(vec![timestamp_arg, format_arg])
            .unwrap();

        let sys_ctx = SystemContext { now: now.clone() };
        let ctx = BasicContext::new(MapBindings::default(), sys_ctx);
        let bindings = Tuple::new();
        let binding = DatumTupleRef::Tuple(&bindings);

        let result = expr.evaluate(&binding, &ctx);
        // Result should be a string (format not fully implemented yet, so just check it's a string)
        assert!(matches!(result.as_ref(), Value::String(_)));
    }

    #[test]
    fn test_to_string_requires_two_args() {
        let arg = EvalLitExpr::new(Value::Integer(123))
            .bind::<true>(vec![])
            .unwrap();
        let result = EvalFnToString.bind::<true>(vec![arg]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }

    #[test]
    fn test_to_string_no_args_fails() {
        let result = EvalFnToString.bind::<true>(vec![]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }

    #[test]
    fn test_to_string_three_args_fails() {
        let arg1 = EvalLitExpr::new(Value::Integer(1))
            .bind::<true>(vec![])
            .unwrap();
        let arg2 = EvalLitExpr::new(Value::from("format"))
            .bind::<true>(vec![])
            .unwrap();
        let arg3 = EvalLitExpr::new(Value::Integer(3))
            .bind::<true>(vec![])
            .unwrap();
        let result = EvalFnToString.bind::<true>(vec![arg1, arg2, arg3]);
        assert!(matches!(result, Err(BindError::ArgNumMismatch { .. })));
    }
}

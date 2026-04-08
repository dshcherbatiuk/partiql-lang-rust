//! Ion number parsing — integers, decimals, floats.
//!
//! Ion has three numeric types with distinct syntax:
//!
//! ## Integers
//! ```text
//! int       ::= ['-'] dec_int | ['-'] hex_int | ['-'] bin_int
//! dec_int   ::= '0' | [1-9] ([0-9_])*
//! hex_int   ::= '0' ('x' | 'X') hex_digit (hex_digit | '_')*
//! bin_int   ::= '0' ('b' | 'B') bin_digit (bin_digit | '_')*
//! ```
//! Underscores allowed between digits for readability: `1_000_000`.
//!
//! ## Decimals
//! ```text
//! decimal   ::= ['-'] digits '.' digits? ('d' | 'D') ['+' | '-'] digits
//!             | ['-'] digits '.' digits
//!             | ['-'] digits '.'
//! ```
//! Decimals use `d`/`D` exponent (base-10). Trailing dot makes it decimal.
//! Preserve exact precision: `1.0` ≠ `1.00`.
//!
//! ## Floats
//! ```text
//! float     ::= ['-'] digits '.' digits ('e' | 'E') ['+' | '-'] digits
//!             | 'nan' | '+inf' | '-inf'
//! ```
//! Floats MUST have `e`/`E` exponent. Special values: `nan`, `+inf`, `-inf`.

use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::take_while;

/// Parsed numeric value.
#[derive(Debug, Clone, PartialEq)]
pub enum IonNumber {
    Integer(i64),
    Decimal(rust_decimal::Decimal),
    Float(f64),
}

/// Parse an Ion integer: decimal, hex, or binary.
// BNF: int ::= ['-'] (dec_int | hex_int | bin_int)
pub fn ion_integer<'a>(input: &mut &'a str) -> PResult<IonNumber> {
    let sign = opt('-').parse_next(input)?;
    let negative = sign.is_some();

    let value = alt((hex_integer, bin_integer, dec_integer)).parse_next(input)?;

    // Must NOT be followed by '.', 'e', 'E', 'd', 'D' (those are decimal/float)
    if let Some(c) = input.chars().next() {
        if c == '.' || c == 'e' || c == 'E' || c == 'd' || c == 'D' {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    }

    Ok(IonNumber::Integer(if negative { -value } else { value }))
}

/// Decimal integer: `0`, `42`, `1_000_000`
///
/// Avoids heap allocation when no underscores present (common case).
fn dec_integer<'a>(input: &mut &'a str) -> PResult<i64> {
    let digits = take_while(1.., |c: char| c.is_ascii_digit() || c == '_').parse_next(input)?;
    parse_int_no_alloc(digits, 10)
}

/// Hexadecimal integer: `0xFACE`, `0X1a2b`
fn hex_integer<'a>(input: &mut &'a str) -> PResult<i64> {
    let _ = alt(("0x", "0X")).parse_next(input)?;
    let digits = take_while(1.., |c: char| c.is_ascii_hexdigit() || c == '_').parse_next(input)?;
    parse_int_no_alloc(digits, 16)
}

/// Binary integer: `0b1010`, `0B1001_0110`
fn bin_integer<'a>(input: &mut &'a str) -> PResult<i64> {
    let _ = alt(("0b", "0B")).parse_next(input)?;
    let digits = take_while(1.., |c: char| c == '0' || c == '1' || c == '_').parse_next(input)?;
    parse_int_no_alloc(digits, 2)
}

/// Parse integer digits with given radix. Avoids heap allocation when
/// no underscores are present (the common case). Only allocates a
/// temporary String when underscores need to be stripped.
fn parse_int_no_alloc(digits: &str, radix: u32) -> PResult<i64> {
    let err = || backtrack();
    if digits.contains('_') {
        let clean: String = digits.chars().filter(|c| *c != '_').collect();
        i64::from_str_radix(&clean, radix).map_err(|_| err())
    } else {
        i64::from_str_radix(digits, radix).map_err(|_| err())
    }
}

/// Parse an Ion float: `1.0e0`, `nan`, `+inf`, `-inf`
// BNF: float ::= special_float | ['-'] digits '.' digits ('e'|'E') ['+'/'-'] digits
pub fn ion_float<'a>(input: &mut &'a str) -> PResult<IonNumber> {
    alt((special_float, numeric_float)).parse_next(input)
}

/// Special float values: `nan`, `+inf`, `-inf`
fn special_float<'a>(input: &mut &'a str) -> PResult<IonNumber> {
    alt((
        "nan".map(|_| IonNumber::Float(f64::NAN)),
        "+inf".map(|_| IonNumber::Float(f64::INFINITY)),
        "-inf".map(|_| IonNumber::Float(f64::NEG_INFINITY)),
    ))
    .parse_next(input)
}

/// Numeric float with e/E exponent.
///
/// Common case (no underscores): parses directly from the input slice.
/// Only allocates when underscores need stripping.
fn numeric_float<'a>(input: &mut &'a str) -> PResult<IonNumber> {
    let start = *input;
    let _ = opt('-').parse_next(input)?;
    let _ = take_while(1.., |c: char| c.is_ascii_digit() || c == '_').parse_next(input)?;
    let _ = '.'.parse_next(input)?;
    let _ = take_while(0.., |c: char| c.is_ascii_digit() || c == '_').parse_next(input)?;
    let _ = alt(('e', 'E')).parse_next(input)?;
    let _ = opt(alt(('+', '-'))).parse_next(input)?;
    let _ = take_while(1.., |c: char| c.is_ascii_digit()).parse_next(input)?;

    let raw = &start[..start.len() - input.len()];
    let err = || backtrack();
    let f: f64 = if raw.contains('_') {
        raw.replace('_', "").parse().map_err(|_| err())?
    } else {
        raw.parse().map_err(|_| err())?
    };
    Ok(IonNumber::Float(f))
}

/// Parse an Ion decimal: `3.14`, `1.0d2`, `123.`
///
/// Common case (no underscores, no d-exponent): parses from input slice.
// BNF: decimal ::= ['-'] digits '.' digits? [('d'|'D') ['+'/'-'] digits]
pub fn ion_decimal<'a>(input: &mut &'a str) -> PResult<IonNumber> {
    let start = *input;
    let _ = opt('-').parse_next(input)?;
    let _ = take_while(1.., |c: char| c.is_ascii_digit() || c == '_').parse_next(input)?;
    let _ = '.'.parse_next(input)?;
    let frac = take_while(0.., |c: char| c.is_ascii_digit() || c == '_').parse_next(input)?;

    // Must NOT have e/E (that's a float)
    if input.starts_with('e') || input.starts_with('E') {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    }

    // Optional d/D exponent — replace with e for rust_decimal parsing
    let has_d_exp = input.starts_with('d') || input.starts_with('D');
    if has_d_exp {
        let _ = winnow::token::any.parse_next(input)?;
        let _ = opt(alt(('+', '-'))).parse_next(input)?;
        let _ = take_while(1.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    }

    let raw = &start[..start.len() - input.len()];

    // Fast path: no underscores, no d-exponent, non-empty frac
    let needs_transform = raw.contains('_') || has_d_exp || frac.is_empty();
    let s: std::borrow::Cow<'_, str> = if needs_transform {
        let mut s = raw.replace('_', "");
        // Replace d/D exponent with e for rust_decimal
        if let Some(pos) = s.find(|c: char| c == 'd' || c == 'D') {
            s.replace_range(pos..pos + 1, "e");
        }
        // Trailing dot without frac: "123." → "123.0"
        if s.ends_with('.') {
            s.push('0');
        }
        std::borrow::Cow::Owned(s)
    } else {
        std::borrow::Cow::Borrowed(raw)
    };
    let d: rust_decimal::Decimal = s
        .parse()
        .map_err(|_| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?;
    Ok(IonNumber::Decimal(d))
}

fn backtrack() -> winnow::error::ErrMode<winnow::error::ContextError> {
    winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())
}

/// Parse any Ion number: float first (greedy e/E), then decimal (greedy dot), then integer.
pub fn ion_number<'a>(input: &mut &'a str) -> PResult<IonNumber> {
    alt((ion_float, ion_decimal, ion_integer)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integers
    #[test]
    fn test_int_decimal() {
        let mut i = "42 rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Integer(42));
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_int_negative() {
        let mut i = "-1 rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Integer(-1));
    }

    #[test]
    fn test_int_zero() {
        let mut i = "0 rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Integer(0));
    }

    #[test]
    fn test_int_underscore() {
        let mut i = "1_000_000 rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Integer(1_000_000));
    }

    #[test]
    fn test_int_hex() {
        let mut i = "0xFACE rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Integer(0xFACE));
    }

    #[test]
    fn test_int_binary() {
        let mut i = "0b1010 rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Integer(0b1010));
    }

    // Decimals
    #[test]
    fn test_decimal_simple() {
        let mut i = "3.14 rest";
        if let IonNumber::Decimal(d) = ion_number(&mut i).unwrap() {
            assert_eq!(d.to_string(), "3.14");
        } else {
            panic!("Expected Decimal");
        }
    }

    #[test]
    fn test_decimal_trailing_dot() {
        let mut i = "123. rest";
        if let IonNumber::Decimal(d) = ion_number(&mut i).unwrap() {
            assert_eq!(d.to_string(), "123.0");
        } else {
            panic!("Expected Decimal");
        }
    }

    // Floats
    #[test]
    fn test_float_simple() {
        let mut i = "1.0e0 rest";
        if let IonNumber::Float(f) = ion_number(&mut i).unwrap() {
            assert!((f - 1.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_float_scientific() {
        let mut i = "6.022e23 rest";
        if let IonNumber::Float(f) = ion_number(&mut i).unwrap() {
            assert!((f - 6.022e23).abs() < 1e18);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_float_nan() {
        let mut i = "nan rest";
        if let IonNumber::Float(f) = ion_number(&mut i).unwrap() {
            assert!(f.is_nan());
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_float_pos_inf() {
        let mut i = "+inf rest";
        assert_eq!(ion_number(&mut i).unwrap(), IonNumber::Float(f64::INFINITY));
    }

    #[test]
    fn test_float_neg_inf() {
        let mut i = "-inf rest";
        assert_eq!(
            ion_number(&mut i).unwrap(),
            IonNumber::Float(f64::NEG_INFINITY)
        );
    }
}

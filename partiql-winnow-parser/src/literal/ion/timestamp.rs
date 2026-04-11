//! Ion timestamp parsing — ISO 8601 bare timestamps.
//!
//! Ion timestamps are bare (not quoted), with varying precision:
//! ```text
//! timestamp ::= year 'T'
//!             | year '-' month 'T'
//!             | year '-' month '-' day ('T')?
//!             | year '-' month '-' day 'T' hour ':' minute offset
//!             | year '-' month '-' day 'T' hour ':' minute ':' second ('.' frac)? offset
//! offset    ::= 'Z' | ('+' | '-') hour ':' minute
//! year      ::= [0-9]{4}
//! month     ::= [0-9]{2}
//! day       ::= [0-9]{2}
//! hour      ::= [0-9]{2}
//! minute    ::= [0-9]{2}
//! second    ::= [0-9]{2}
//! frac      ::= [0-9]+
//! ```
//!
//! Returns a zero-allocation `&str` slice from the input.

// Wired into PrimaryStrategy for bare Ion timestamp expressions like
// `2024-01-01T10:00:00Z` appearing in DML/SELECT.

use winnow::combinator::alt;
use winnow::prelude::*;
use winnow::token::{take, take_while};

/// Parse an Ion timestamp, returning a borrowed slice from the input.
///
/// Zero allocations — the returned `&str` points directly into the input.
// BNF: see module docs
pub fn ion_timestamp<'a>(input: &mut &'a str) -> PResult<&'a str> {
    let start = *input;

    // Year: YYYY
    let year = take(4usize).parse_next(input)?;
    if !year.chars().all(|c| c.is_ascii_digit()) {
        return Err(backtrack());
    }

    // Year-only: YYYY'T'
    if consume_if(input, 'T') && !input.starts_with(|c: char| c.is_ascii_digit()) {
        return Ok(slice_from(start, input));
    }

    // -MM
    if !consume_if(input, '-') {
        return Err(backtrack());
    }
    let _ = digits2(input)?;

    // Month-only: YYYY-MM'T'
    if consume_if(input, 'T') && !input.starts_with(|c: char| c.is_ascii_digit()) {
        return Ok(slice_from(start, input));
    }

    // -DD
    if !consume_if(input, '-') {
        return Err(backtrack());
    }
    let _ = digits2(input)?;

    // Day-only (no T)
    if !consume_if(input, 'T') {
        return Ok(slice_from(start, input));
    }

    // YYYY-MM-DDT with no time
    if !input.starts_with(|c: char| c.is_ascii_digit()) {
        return Ok(slice_from(start, input));
    }

    // HH:MM
    let _ = digits2(input)?;
    if !consume_if(input, ':') {
        return Err(backtrack());
    }
    let _ = digits2(input)?;

    // Optional :SS
    if consume_if(input, ':') {
        let _ = digits2(input)?;
        // Optional .frac
        if consume_if(input, '.') {
            let _ = take_while(1.., |c: char| c.is_ascii_digit()).parse_next(input)?;
        }
    }

    // Offset: Z | +HH:MM | -HH:MM
    parse_offset(input)?;

    Ok(slice_from(start, input))
}

/// Parse two ASCII digits.
fn digits2<'a>(input: &mut &'a str) -> PResult<&'a str> {
    let d = take(2usize).parse_next(input)?;
    if !d.chars().all(|c| c.is_ascii_digit()) {
        return Err(backtrack());
    }
    Ok(d)
}

/// Consume a single char if it matches, return whether it was consumed.
fn consume_if(input: &mut &str, expected: char) -> bool {
    if input.starts_with(expected) {
        *input = &input[expected.len_utf8()..];
        true
    } else {
        false
    }
}

/// Parse timezone offset: `Z`, `+HH:MM`, `-HH:MM`
fn parse_offset<'a>(input: &mut &'a str) -> PResult<()> {
    alt((
        'Z'.void(),
        (alt(('+', '-')), take(2usize), ':', take(2usize)).void(),
    ))
    .parse_next(input)
}

/// Return the slice of `start` that was consumed (up to where `remaining` begins).
fn slice_from<'a>(start: &'a str, remaining: &'a str) -> &'a str {
    let consumed = start.len() - remaining.len();
    &start[..consumed]
}

fn backtrack() -> winnow::error::ErrMode<winnow::error::ContextError> {
    winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_year_only() {
        let mut i = "2024T rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024T");
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_year_month() {
        let mut i = "2024-01T rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01T");
    }

    #[test]
    fn test_date_only() {
        let mut i = "2024-01-15 rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15");
    }

    #[test]
    fn test_date_with_trailing_t() {
        let mut i = "2024-01-15T rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15T");
    }

    #[test]
    fn test_minute_precision_utc() {
        let mut i = "2024-01-15T12:30Z rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15T12:30Z");
    }

    #[test]
    fn test_second_precision_utc() {
        let mut i = "2024-01-15T12:30:45Z rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15T12:30:45Z");
    }

    #[test]
    fn test_fractional_seconds() {
        let mut i = "2024-01-15T12:30:45.123456Z rest";
        assert_eq!(
            ion_timestamp(&mut i).unwrap(),
            "2024-01-15T12:30:45.123456Z"
        );
    }

    #[test]
    fn test_positive_offset() {
        let mut i = "2024-01-15T12:30:00+05:30 rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15T12:30:00+05:30");
    }

    #[test]
    fn test_negative_offset() {
        let mut i = "2024-01-15T08:00:00-05:00 rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15T08:00:00-05:00");
    }

    #[test]
    fn test_unknown_offset() {
        let mut i = "2024-01-15T12:00:00-00:00 rest";
        assert_eq!(ion_timestamp(&mut i).unwrap(), "2024-01-15T12:00:00-00:00");
    }

    #[test]
    fn test_not_a_timestamp() {
        let mut i = "hello rest";
        assert!(ion_timestamp(&mut i).is_err());
    }
}

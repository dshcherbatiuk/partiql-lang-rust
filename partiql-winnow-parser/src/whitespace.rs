//! Whitespace handling for PartiQL parsing.
//!
//! SQL allows arbitrary whitespace (spaces, tabs, newlines) between tokens.
//! Two combinators handle the two common cases:
//!
//! - `ws()` — mandatory whitespace (1+ chars). Used after keywords:
//!   `SELECT ws FROM ws WHERE ws`
//! - `ws0()` — optional whitespace (0+ chars). Used around operators:
//!   `expr ws0 '=' ws0 expr`

use winnow::error::ContextError;
use winnow::prelude::*;

/// Mandatory whitespace — at least one space, tab, or newline.
/// Inline ASCII fast path — no winnow scanner overhead for common case.
#[inline]
pub fn ws<'a>(input: &mut &'a str) -> PResult<()> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_whitespace() {
        return Err(winnow::error::ErrMode::Backtrack(ContextError::new()));
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    *input = &input[i..];
    Ok(())
}

/// Optional whitespace — zero or more spaces, tabs, or newlines.
/// Inline ASCII fast path — no winnow scanner overhead.
#[inline]
pub fn ws0<'a>(input: &mut &'a str) -> PResult<()> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i > 0 {
        *input = &input[i..];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_mandatory() {
        let mut i = "  rest";
        assert!(ws(&mut i).is_ok());
        assert_eq!(i, "rest");

        let mut i = "\t\n rest";
        assert!(ws(&mut i).is_ok());
        assert_eq!(i, "rest");
    }

    #[test]
    fn test_ws_fails_on_no_space() {
        let mut i = "rest";
        assert!(ws(&mut i).is_err());
    }

    #[test]
    fn test_ws0_optional() {
        let mut i = "  rest";
        assert!(ws0(&mut i).is_ok());
        assert_eq!(i, "rest");

        let mut i = "rest";
        assert!(ws0(&mut i).is_ok());
        assert_eq!(i, "rest");
    }
}

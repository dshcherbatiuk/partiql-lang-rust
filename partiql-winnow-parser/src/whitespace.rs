//! Whitespace handling for PartiQL parsing.
//!
//! SQL allows arbitrary whitespace (spaces, tabs, newlines) between tokens.
//! Two combinators handle the two common cases:
//!
//! - `ws()` — mandatory whitespace (1+ chars). Used after keywords:
//!   `SELECT ws FROM ws WHERE ws`
//! - `ws0()` — optional whitespace (0+ chars). Used around operators:
//!   `expr ws0 '=' ws0 expr`

use winnow::ascii::{multispace0, multispace1};
use winnow::prelude::*;

/// Mandatory whitespace — at least one space, tab, or newline.
///
/// Use after keywords that require separation from the next token:
/// `SELECT`, `FROM`, `WHERE`, `INTO`, etc.
pub fn ws<'a>(input: &mut &'a str) -> PResult<()> {
    multispace1.void().parse_next(input)
}

/// Optional whitespace — zero or more spaces, tabs, or newlines.
///
/// Use around operators and punctuation where whitespace is optional:
/// commas, parentheses, `=`, `<`, `>`, etc.
pub fn ws0<'a>(input: &mut &'a str) -> PResult<()> {
    multispace0.void().parse_next(input)
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

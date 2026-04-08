//! Case-insensitive SQL keyword matching for PartiQL.
//!
//! PartiQL keywords are case-insensitive per the SQL standard:
//! `SELECT`, `select`, `SeLeCt` all match the same keyword.
//!
//! This module provides a `kw()` combinator that matches a keyword
//! without consuming trailing whitespace. The caller is responsible
//! for whitespace handling — this keeps keyword matching composable
//! with different whitespace rules (mandatory ws after FROM,
//! optional ws before comma, etc.).
//!
//! ## Word boundary
//!
//! `kw("SELECT")` will match the prefix of `"SELECTING"`. Callers
//! must ensure word boundaries by requiring whitespace or punctuation
//! after the keyword. Typical usage:
//!
//! ```text
//! (kw("SELECT"), ws)          // SELECT followed by mandatory whitespace
//! (kw("FROM"), ws)            // FROM followed by mandatory whitespace
//! (ws0, kw("WHERE"), ws)      // optional ws before, mandatory after
//! ```
//!
//! ## Reserved keywords
//!
//! PartiQL reserves the following keywords (subset relevant to FDE):
//!
//! | Category | Keywords |
//! |----------|----------|
//! | DQL | SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER, LIMIT, OFFSET, AS, AT, JOIN, ON, CROSS, INNER, LEFT, RIGHT, FULL, OUTER, VALUE, DISTINCT, ALL |
//! | DML | INSERT, INTO, DELETE, REPLACE, UPSERT, SET, ON, CONFLICT, DO, NOTHING, EXCLUDED, UPDATE |
//! | Operators | AND, OR, NOT, IN, LIKE, BETWEEN, IS, NULL, MISSING, TRUE, FALSE, ASC, DESC, CASE, WHEN, THEN, ELSE, END, CAST, COALESCE, NULLIF |
//! | Functions | COUNT, SUM, AVG, MIN, MAX, CURRENT_TIMESTAMP, CURRENT_TIME, TO_STRING |

use winnow::error::ContextError;
use winnow::prelude::*;

/// Match a SQL keyword case-insensitively with word boundary enforcement.
///
/// After matching the keyword, verifies the next character is NOT alphanumeric
/// or underscore — preventing `OR` from matching the prefix of `ORDER`,
/// `IN` from matching `INSERT`, etc.
///
/// Does NOT consume trailing whitespace — caller controls whitespace rules.
///
/// # BNF
/// ```text
/// keyword ::= SELECT | FROM | WHERE | INSERT | INTO | DELETE | ...
/// ```
pub fn kw<'a>(keyword: &'static str) -> impl Parser<&'a str, &'a str, ContextError> {
    move |input: &mut &'a str| {
        let checkpoint = *input;
        let matched = winnow::ascii::Caseless(keyword).parse_next(input)?;
        // Check word boundary — next char must not be alphanumeric or underscore.
        // Prevents `OR` from matching prefix of `ORDER`, `IN` from `INSERT`, etc.
        if let Some(next) = input.chars().next() {
            if next.is_ascii_alphanumeric() || next == '_' {
                *input = checkpoint;
                return Err(winnow::error::ErrMode::Backtrack(ContextError::new()));
            }
        }
        Ok(matched)
    }
}

/// Match a single character with explicit error type.
/// Avoids winnow type inference issues with bare char literals.
#[inline]
pub fn ch<'a>(c: char) -> impl Parser<&'a str, char, ContextError> {
    winnow::token::one_of(c)
}

/// Match an exact string with explicit error type.
#[inline]
pub fn lit<'a>(s: &'static str) -> impl Parser<&'a str, &'a str, ContextError> {
    winnow::token::literal(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_insensitive() {
        let mut i = "SELECT rest";
        assert!(kw("SELECT").parse_next(&mut i).is_ok());
        assert_eq!(i, " rest");

        let mut i = "select rest";
        assert!(kw("SELECT").parse_next(&mut i).is_ok());

        let mut i = "SeLeCt rest";
        assert!(kw("SELECT").parse_next(&mut i).is_ok());
    }

    #[test]
    fn test_no_match() {
        let mut i = "INSERT rest";
        assert!(kw("SELECT").parse_next(&mut i).is_err());
    }

    #[test]
    fn test_word_boundary_enforced() {
        // kw does NOT match prefix — word boundary prevents `SELECT` from matching `SELECTING`
        let mut i = "SELECTING rest";
        assert!(kw("SELECT").parse_next(&mut i).is_err());
        assert_eq!(i, "SELECTING rest"); // input unchanged
    }

    #[test]
    fn test_word_boundary_allows_non_alpha() {
        // kw matches when followed by whitespace, punctuation, or EOF
        let mut i = "SELECT rest";
        assert!(kw("SELECT").parse_next(&mut i).is_ok());

        let mut i = "SELECT(";
        assert!(kw("SELECT").parse_next(&mut i).is_ok());

        let mut i = "SELECT";
        assert!(kw("SELECT").parse_next(&mut i).is_ok());
    }

    #[test]
    fn test_or_not_matching_order() {
        let mut i = "ORDER BY";
        assert!(kw("OR").parse_next(&mut i).is_err());
        assert_eq!(i, "ORDER BY"); // input unchanged
    }
}

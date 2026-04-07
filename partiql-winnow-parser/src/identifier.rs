//! PartiQL identifier parsing — table names, column names, aliases.
//!
//! PartiQL supports two identifier styles:
//!
//! - **Unquoted**: `users`, `email`, `_private` — alphanumeric + underscore,
//!   must start with letter or underscore. Case-insensitive for resolution.
//! - **Quoted**: `"fde.users"`, `"my table"` — any characters between double
//!   quotes. Preserves case and allows special characters (dots, spaces).
//!
//! Dotted paths like `u.email` or `"schema"."table"` are sequences of
//! identifiers separated by dots.
//!
//! # BNF
//! ```text
//! identifier       ::= unquoted_ident | quoted_ident
//! unquoted_ident   ::= [a-zA-Z_] [a-zA-Z0-9_]*
//! quoted_ident     ::= '"' [^"]+ '"'
//! dotted_path      ::= identifier ('.' identifier)*
//! ```

use winnow::combinator::{alt, delimited};
use winnow::error::ContextError;
use winnow::prelude::*;
use winnow::token::take_while;

/// Quoted identifier: `"some.table"` → `some.table`
///
/// Allows any characters between double quotes except double quote itself.
pub fn quoted_identifier<'a>(input: &mut &'a str) -> PResult<String> {
    delimited('"', take_while(1.., |c: char| c != '"'), '"')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

/// Unquoted identifier: `tableName`, `_field1`, `email`
///
/// Must start with ASCII letter or underscore. Continues with alphanumeric
/// or underscore. Does NOT match keywords — caller must check separately.
pub fn unquoted_identifier<'a>(input: &mut &'a str) -> PResult<String> {
    (
        take_while(1, |c: char| c.is_ascii_alphabetic() || c == '_'),
        take_while(0.., |c: char| c.is_ascii_alphanumeric() || c == '_'),
    )
        .map(|(first, rest): (&str, &str)| format!("{first}{rest}"))
        .parse_next(input)
}

/// Identifier — quoted or unquoted.
///
/// Tries quoted first (starts with `"`), then unquoted.
pub fn identifier<'a>(input: &mut &'a str) -> PResult<String> {
    alt((quoted_identifier, unquoted_identifier)).parse_next(input)
}

/// Dotted path: `a.b.c` or `"schema"."table".column`
///
/// Parses a sequence of identifiers separated by dots. Stops when no
/// more dots follow.
pub fn dotted_path<'a>(input: &mut &'a str) -> PResult<String> {
    let first = identifier.parse_next(input)?;
    let mut path = first;
    while winnow::token::one_of::<_, _, ContextError>('.')
        .parse_next(input)
        .is_ok()
    {
        let next = identifier.parse_next(input)?;
        path.push('.');
        path.push_str(&next);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quoted_identifier() {
        let mut i = r#""my.table" rest"#;
        assert_eq!(quoted_identifier(&mut i).unwrap(), "my.table");
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_quoted_with_spaces() {
        let mut i = r#""my table" rest"#;
        assert_eq!(quoted_identifier(&mut i).unwrap(), "my table");
    }

    #[test]
    fn test_unquoted_identifier() {
        let mut i = "users rest";
        assert_eq!(unquoted_identifier(&mut i).unwrap(), "users");
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_unquoted_with_underscore() {
        let mut i = "_private_field rest";
        assert_eq!(unquoted_identifier(&mut i).unwrap(), "_private_field");
    }

    #[test]
    fn test_unquoted_with_numbers() {
        let mut i = "field123 rest";
        assert_eq!(unquoted_identifier(&mut i).unwrap(), "field123");
    }

    #[test]
    fn test_unquoted_rejects_leading_number() {
        let mut i = "123field rest";
        assert!(unquoted_identifier(&mut i).is_err());
    }

    #[test]
    fn test_identifier_prefers_quoted() {
        let mut i = r#""users" rest"#;
        assert_eq!(identifier(&mut i).unwrap(), "users");
    }

    #[test]
    fn test_identifier_falls_back_to_unquoted() {
        let mut i = "users rest";
        assert_eq!(identifier(&mut i).unwrap(), "users");
    }

    #[test]
    fn test_dotted_path_simple() {
        let mut i = "a.b.c rest";
        assert_eq!(dotted_path(&mut i).unwrap(), "a.b.c");
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_dotted_path_quoted() {
        let mut i = r#""schema"."table" rest"#;
        assert_eq!(dotted_path(&mut i).unwrap(), "schema.table");
    }

    #[test]
    fn test_dotted_path_single() {
        let mut i = "users rest";
        assert_eq!(dotted_path(&mut i).unwrap(), "users");
        assert_eq!(i, " rest");
    }
}

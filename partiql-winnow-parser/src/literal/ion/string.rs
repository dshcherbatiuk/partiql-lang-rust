//! Ion string and symbol parsing.
//!
//! Ion has two string-like types:
//!
//! ## Strings (double-quoted)
//! ```text
//! short_string ::= '"' (char | escape)* '"'
//! long_string  ::= "'''" (char | escape | newline)* "'''"
//! ```
//!
//! ## SQL Strings (single-quoted, PartiQL extension)
//! ```text
//! sql_string   ::= "'" ([^'] | "''")* "'"
//! ```
//! SQL strings escape single quotes by doubling: `'it''s'` → `it's`.
//! Ion strings use backslash escapes: `"it\'s"` or `"it's"`.
//!
//! ## Symbols
//! ```text
//! symbol       ::= unquoted_symbol | quoted_symbol | symbol_id
//! quoted_symbol ::= "'" (char | escape)* "'"     (same as Ion short string but single-quoted)
//! symbol_id     ::= '$' [0-9]+
//! ```
//!
//! ## Escape sequences (shared by strings and quoted symbols)
//! ```text
//! escape ::= '\\' ('\\' | '"' | '\'' | '/' | 'a' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '0'
//!           | 'x' hex hex
//!           | 'u' hex hex hex hex
//!           | 'U' hex hex hex hex hex hex hex hex)
//! ```

use winnow::prelude::*;
use winnow::token::take_while;

/// Parse an Ion double-quoted string: `"hello"`, `"line\nbreak"`
///
/// Supports full Ion escape sequences: `\\`, `\"`, `\n`, `\t`, `\uXXXX`, etc.
// BNF: short_string ::= '"' (char | escape)* '"'
pub fn ion_string(input: &mut &str) -> PResult<String> {
    let _ = '"'.parse_next(input)?;
    let mut result = String::new();
    loop {
        let chunk = take_while(0.., |c: char| c != '"' && c != '\\').parse_next(input)?;
        result.push_str(chunk);
        if input.starts_with('"') {
            let _ = '"'.parse_next(input)?;
            break;
        } else if input.starts_with('\\') {
            let _ = '\\'.parse_next(input)?;
            let escaped = parse_escape_char(input)?;
            result.push(escaped);
        } else {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    }
    Ok(result)
}

/// Parse a SQL single-quoted string: `'hello'`, `'it''s'`
///
/// SQL strings escape single quotes by doubling: `''` → `'`.
/// No backslash escapes — backslash is a literal character.
// BNF: sql_string ::= "'" ([^'] | "''")* "'"
pub fn sql_string(input: &mut &str) -> PResult<String> {
    let _ = '\''.parse_next(input)?;
    let mut result = String::new();
    loop {
        let chunk = take_while(0.., |c: char| c != '\'').parse_next(input)?;
        result.push_str(chunk);
        let _ = '\''.parse_next(input)?;
        // Escaped quote: ''
        if input.starts_with('\'') {
            let _ = '\''.parse_next(input)?;
            result.push('\'');
        } else {
            break;
        }
    }
    Ok(result)
}

/// Parse a single escape character after `\`.
pub(crate) fn parse_escape_char(input: &mut &str) -> PResult<char> {
    let c = winnow::token::any.parse_next(input)?;
    match c {
        '\\' => Ok('\\'),
        '"' => Ok('"'),
        '\'' => Ok('\''),
        '/' => Ok('/'),
        'a' => Ok('\x07'), // BEL
        'b' => Ok('\x08'), // BS
        'f' => Ok('\x0C'), // FF
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        'v' => Ok('\x0B'), // VT
        '0' => Ok('\0'),
        'x' => parse_hex_escape(input, 2),
        'u' => parse_hex_escape(input, 4),
        'U' => parse_hex_escape(input, 8),
        _ => Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        )),
    }
}

/// Parse N hex digits and convert to a char.
fn parse_hex_escape(input: &mut &str, digits: usize) -> PResult<char> {
    let hex = winnow::token::take(digits).parse_next(input)?;
    let code = u32::from_str_radix(hex, 16)
        .map_err(|_| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?;
    char::from_u32(code)
        .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ion double-quoted strings
    #[test]
    fn test_ion_string_simple() {
        let mut i = r#""hello" rest"#;
        assert_eq!(ion_string(&mut i).unwrap(), "hello");
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_ion_string_escape_newline() {
        let mut i = r#""line\nbreak" rest"#;
        assert_eq!(ion_string(&mut i).unwrap(), "line\nbreak");
    }

    #[test]
    fn test_ion_string_escape_tab() {
        let mut i = r#""col\tcol" rest"#;
        assert_eq!(ion_string(&mut i).unwrap(), "col\tcol");
    }

    #[test]
    fn test_ion_string_escape_backslash() {
        let mut i = r#""path\\file" rest"#;
        assert_eq!(ion_string(&mut i).unwrap(), "path\\file");
    }

    #[test]
    fn test_ion_string_escape_quote() {
        let mut i = r#""say \"hi\"" rest"#;
        assert_eq!(ion_string(&mut i).unwrap(), r#"say "hi""#);
    }

    #[test]
    fn test_ion_string_unicode_escape() {
        let mut i = r#""\u0041" rest"#; // U+0041 = 'A'
        assert_eq!(ion_string(&mut i).unwrap(), "A");
    }

    #[test]
    fn test_ion_string_hex_escape() {
        let mut i = r#""\x41" rest"#; // 0x41 = 'A'
        assert_eq!(ion_string(&mut i).unwrap(), "A");
    }

    #[test]
    fn test_ion_string_empty() {
        let mut i = r#""" rest"#;
        assert_eq!(ion_string(&mut i).unwrap(), "");
    }

    // SQL single-quoted strings
    #[test]
    fn test_sql_string_simple() {
        let mut i = "'hello' rest";
        assert_eq!(sql_string(&mut i).unwrap(), "hello");
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_sql_string_escaped_quote() {
        let mut i = "'it''s' rest";
        assert_eq!(sql_string(&mut i).unwrap(), "it's");
    }

    #[test]
    fn test_sql_string_empty() {
        let mut i = "'' rest";
        assert_eq!(sql_string(&mut i).unwrap(), "");
    }

    #[test]
    fn test_sql_string_backslash_literal() {
        // SQL strings don't interpret backslash escapes
        let mut i = r"'path\file' rest";
        assert_eq!(sql_string(&mut i).unwrap(), r"path\file");
    }

    #[test]
    fn test_sql_string_with_double_quotes() {
        let mut i = r#"'say "hi"' rest"#;
        assert_eq!(sql_string(&mut i).unwrap(), r#"say "hi""#);
    }
}

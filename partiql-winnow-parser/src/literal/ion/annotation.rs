//! Ion annotation parsing.
//!
//! Any Ion value can be prefixed with one or more annotations:
//! ```text
//! annotated_value ::= (symbol '::')* value
//! annotation      ::= unquoted_symbol '::' | quoted_symbol '::' | symbol_id '::'
//! symbol_id       ::= '$' [0-9]+
//! ```
//!
//! Annotations are symbols — unquoted identifiers, single-quoted `'...'`,
//! or symbol IDs `$0`, `$10`. The `::` separator connects annotation to value.
//!
//! Examples: `dollars::100`, `a::b::42`, `'custom type'::null`

use super::string::parse_escape_char;
use crate::identifier::unquoted_identifier;
use crate::keyword::lit;
use smallvec::SmallVec;
use winnow::prelude::*;
use winnow::token::take_while;

/// A single Ion annotation — the symbol before `::`.
#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// Unquoted identifier: `dollars`, `myType`
    Identifier(String),
    /// Single-quoted symbol: `'custom type'`, `'null'`
    QuotedSymbol(String),
    /// Symbol ID: `$0`, `$10`
    SymbolId(u32),
}

/// Parse a single-quoted Ion symbol: `'hello world'`, `'null'`
///
/// Uses Ion escape rules (backslash escapes), NOT SQL doubling.
fn quoted_symbol(input: &mut &str) -> ModalResult<String> {
    // Ion quoted symbols use single quotes with backslash escapes
    // (same escape rules as Ion double-quoted strings)
    let _ = '\''.parse_next(input)?;
    let mut result = String::new();
    loop {
        let chunk = take_while(0.., |c: char| c != '\'' && c != '\\').parse_next(input)?;
        result.push_str(chunk);
        if input.starts_with('\'') {
            let _ = '\''.parse_next(input)?;
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

/// Parse a symbol ID: `$0`, `$10`, `$256`
fn symbol_id(input: &mut &str) -> PResult<u32> {
    let _ = '$'.parse_next(input)?;
    let digits = take_while(1.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    digits
        .parse()
        .map_err(|_| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))
}

/// Parse a single annotation (the symbol before `::`) without consuming `::`.
fn annotation_symbol(input: &mut &str) -> PResult<Annotation> {
    if input.starts_with('$') && input[1..].starts_with(|c: char| c.is_ascii_digit()) {
        return symbol_id(input).map(Annotation::SymbolId);
    }
    if input.starts_with('\'') {
        return quoted_symbol(input).map(Annotation::QuotedSymbol);
    }
    unquoted_identifier(input).map(Annotation::Identifier)
}

/// Parse zero or more annotations: `a::b::` prefix before a value.
///
/// Returns the list of annotations consumed. Caller then parses the value.
/// Returns empty vec if no annotations found (not an error).
// BNF: annotations ::= (symbol '::')*
pub fn annotations(input: &mut &str) -> SmallVec<[Annotation; 8]> {
    let mut result = SmallVec::new();
    loop {
        let checkpoint = *input;
        match annotation_symbol(input) {
            Ok(ann) => {
                if lit("::").parse_next(input).is_ok() {
                    result.push(ann);
                } else {
                    // Symbol without :: — not an annotation, restore
                    *input = checkpoint;
                    break;
                }
            }
            Err(_) => {
                *input = checkpoint;
                break;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    #[test]
    fn test_no_annotations() {
        let mut i = "42 rest";
        let anns = annotations(&mut i);
        assert!(anns.is_empty());
        assert_eq!(i, "42 rest"); // input unchanged
    }

    #[test]
    fn test_single_identifier_annotation() {
        let mut i = "dollars::100";
        let anns = annotations(&mut i);
        let vec: SmallVec<[Annotation; 1]> = smallvec![Annotation::Identifier("dollars".into())];
        assert_eq!(anns, vec);
        assert_eq!(i, "100");
    }

    #[test]
    fn test_multiple_annotations() {
        let mut i = "a::b::42";
        let anns = annotations(&mut i);
        let vec: SmallVec<[Annotation; 2]> = smallvec![
            Annotation::Identifier("a".into()),
            Annotation::Identifier("b".into()),
        ];
        assert_eq!(anns, vec);
        assert_eq!(i, "42");
    }

    #[test]
    fn test_quoted_symbol_annotation() {
        let mut i = "'custom type'::null";
        let anns = annotations(&mut i);
        let vec: SmallVec<[Annotation; 1]> =
            smallvec![Annotation::QuotedSymbol("custom type".into())];
        assert_eq!(anns, vec);
        assert_eq!(i, "null");
    }

    #[test]
    fn test_symbol_id_annotation() {
        let mut i = "$10::value";
        let anns = annotations(&mut i);
        let vec: SmallVec<[Annotation; 1]> = smallvec![Annotation::SymbolId(10)];
        assert_eq!(anns, vec);
        assert_eq!(i, "value");
    }

    #[test]
    fn test_mixed_annotations() {
        let mut i = "a::'type'::$0::42";
        let anns = annotations(&mut i);
        let vec: SmallVec<[Annotation; 3]> = smallvec![
            Annotation::Identifier("a".into()),
            Annotation::QuotedSymbol("type".into()),
            Annotation::SymbolId(0),
        ];
        assert_eq!(anns, vec);
        assert_eq!(i, "42");
    }

    #[test]
    fn test_identifier_without_colons_not_annotation() {
        let mut i = "hello world";
        let anns = annotations(&mut i);
        assert!(anns.is_empty());
        assert_eq!(i, "hello world");
    }

    #[test]
    fn test_symbol_id_zero() {
        let mut i = "$0::value";
        let anns = annotations(&mut i);
        let vec: SmallVec<[Annotation; 1]> = smallvec![Annotation::SymbolId(0)];
        assert_eq!(anns, vec);
    }
}

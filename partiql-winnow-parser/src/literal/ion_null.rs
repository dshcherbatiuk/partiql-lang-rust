//! Ion null parsing — generic null and typed nulls.
//!
//! Ion supports a generic `null` and typed nulls for every Ion type:
//! ```text
//! null_value  ::= 'null' ('.' type_name)?
//! type_name   ::= 'null' | 'bool' | 'int' | 'float' | 'decimal'
//!               | 'timestamp' | 'string' | 'symbol' | 'blob' | 'clob'
//!               | 'list' | 'sexp' | 'struct'
//! ```
//!
//! PartiQL adds `MISSING` as a distinct absent-value type.

use crate::keyword::kw;
use winnow::combinator::alt;
use winnow::error::ContextError;
use winnow::prelude::*;

/// Ion typed null category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedNull {
    Null,
    Bool,
    Int,
    Float,
    Decimal,
    Timestamp,
    String,
    Symbol,
    Blob,
    Clob,
    List,
    Sexp,
    Struct,
}

/// Ion null value — generic or typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IonNull {
    /// Generic null: `null`
    Generic,
    /// Typed null: `null.int`, `null.string`, etc.
    Typed(TypedNull),
}

/// Parse an Ion null value.
// BNF: null_value ::= 'null' ('.' type_name)?
pub fn ion_null<'a>(input: &mut &'a str) -> PResult<IonNull> {
    let _ = kw("null").parse_next(input)?;

    // Try typed null: null.type
    if winnow::token::one_of::<_, _, ContextError>('.')
        .parse_next(input)
        .is_ok()
    {
        let type_name = alt((
            kw("null").map(|_| TypedNull::Null),
            kw("bool").map(|_| TypedNull::Bool),
            kw("int").map(|_| TypedNull::Int),
            kw("float").map(|_| TypedNull::Float),
            kw("decimal").map(|_| TypedNull::Decimal),
            kw("timestamp").map(|_| TypedNull::Timestamp),
            kw("string").map(|_| TypedNull::String),
            kw("symbol").map(|_| TypedNull::Symbol),
            kw("blob").map(|_| TypedNull::Blob),
            kw("clob").map(|_| TypedNull::Clob),
            kw("list").map(|_| TypedNull::List),
            kw("sexp").map(|_| TypedNull::Sexp),
            kw("struct").map(|_| TypedNull::Struct),
        ))
        .parse_next(input)?;
        Ok(IonNull::Typed(type_name))
    } else {
        Ok(IonNull::Generic)
    }
}

/// Parse PartiQL MISSING literal.
pub fn missing<'a>(input: &mut &'a str) -> PResult<()> {
    kw("MISSING").void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_null() {
        let mut i = "null rest";
        assert_eq!(ion_null(&mut i).unwrap(), IonNull::Generic);
        assert_eq!(i, " rest");
    }

    #[test]
    fn test_typed_null_int() {
        let mut i = "null.int rest";
        assert_eq!(ion_null(&mut i).unwrap(), IonNull::Typed(TypedNull::Int));
    }

    #[test]
    fn test_typed_null_string() {
        let mut i = "null.string rest";
        assert_eq!(ion_null(&mut i).unwrap(), IonNull::Typed(TypedNull::String));
    }

    #[test]
    fn test_typed_null_struct() {
        let mut i = "null.struct rest";
        assert_eq!(ion_null(&mut i).unwrap(), IonNull::Typed(TypedNull::Struct));
    }

    #[test]
    fn test_typed_null_all_types() {
        for (input, expected) in [
            ("null.null", TypedNull::Null),
            ("null.bool", TypedNull::Bool),
            ("null.int", TypedNull::Int),
            ("null.float", TypedNull::Float),
            ("null.decimal", TypedNull::Decimal),
            ("null.timestamp", TypedNull::Timestamp),
            ("null.string", TypedNull::String),
            ("null.symbol", TypedNull::Symbol),
            ("null.blob", TypedNull::Blob),
            ("null.clob", TypedNull::Clob),
            ("null.list", TypedNull::List),
            ("null.sexp", TypedNull::Sexp),
            ("null.struct", TypedNull::Struct),
        ] {
            let mut i = input;
            assert_eq!(
                ion_null(&mut i).unwrap(),
                IonNull::Typed(expected),
                "Failed for: {input}"
            );
        }
    }

    #[test]
    fn test_missing() {
        let mut i = "MISSING rest";
        assert!(missing(&mut i).is_ok());
        assert_eq!(i, " rest");

        let mut i = "missing rest";
        assert!(missing(&mut i).is_ok());
    }
}

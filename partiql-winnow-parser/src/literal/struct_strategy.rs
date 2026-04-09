//! Struct constructor — `{key: value, key: value, ...}`.
//!
//! ```text
//! struct  ::= '{' pair (',' pair)* '}'
//!           | '{' '}'
//! pair    ::= expr ':' expr
//! ```
//!
//! Keys are typically string literals (`'name'`) or identifiers.

use partiql_ast::ast;
use partiql_ast::ast::ExprPair;
use winnow::prelude::*;

use super::LiteralStrategy;
use crate::expr::StrategyContext;
use crate::identifier;
use crate::keyword::ch;
use crate::whitespace::ws0;

pub struct StructConstructorStrategy;

impl LiteralStrategy for StructConstructorStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        ch('{').parse_next(input)?;
        let _ = ws0(input);

        // Empty struct: {}
        if ch('}').parse_next(input).is_ok() {
            return Ok(ast::Expr::Struct(ctx.node(ast::Struct {
                fields: Vec::new(),
            })));
        }

        let mut fields = Vec::new();
        loop {
            let _ = ws0(input);
            // Struct keys are always string literals — 'key', "key", or unquoted key
            let key = parse_struct_key(input, ctx)?;
            let _ = ws0(input);
            ch(':').parse_next(input)?;
            let _ = ws0(input);
            // In struct context, "double-quoted" values are string literals (Ion compat)
            let value = parse_struct_value(input, ctx)?;

            fields.push(ExprPair {
                first: Box::new(key),
                second: Box::new(value),
            });

            let _ = ws0(input);
            if ch(',').parse_next(input).is_err() {
                break;
            }
        }
        let _ = ws0(input);
        ch('}').parse_next(input)?;

        Ok(ast::Expr::Struct(ctx.node(ast::Struct { fields })))
    }
}

/// Parse struct key as string literal — supports 'key', "key", and unquoted key.
/// In Ion/PartiQL struct context, all keys are string literals regardless of quoting.
fn parse_struct_key<'a>(
    input: &mut &'a str,
    ctx: &StrategyContext<'_>,
) -> PResult<ast::Expr> {
    // Single-quoted: 'key'
    if input.starts_with('\'') {
        let s = crate::literal::ion::string::sql_string.parse_next(input)?;
        return Ok(ast::Expr::Lit(ctx.node(ast::Lit::CharStringLit(s))));
    }
    // Double-quoted: "key" — treat as string literal in struct context (not identifier)
    if input.starts_with('"') {
        let s = identifier::quoted_identifier(input)?;
        return Ok(ast::Expr::Lit(ctx.node(ast::Lit::CharStringLit(
            s.to_string(),
        ))));
    }
    // Unquoted: key — treat as string literal
    let s = identifier::unquoted_identifier(input)?;
    Ok(ast::Expr::Lit(ctx.node(ast::Lit::CharStringLit(
        s.to_string(),
    ))))
}

/// Parse struct value — in struct context, "double-quoted" is a string literal (Ion compat).
/// Falls through to normal expression parsing for everything else.
fn parse_struct_value<'a>(
    input: &mut &'a str,
    ctx: &StrategyContext<'_>,
) -> PResult<ast::Expr> {
    // "double-quoted" → string literal in struct context
    if input.starts_with('"') {
        let s = identifier::quoted_identifier(input)?;
        return Ok(ast::Expr::Lit(ctx.node(ast::Lit::CharStringLit(
            s.to_string(),
        ))));
    }
    // Everything else: normal expression (handles 'single', numbers, bools, nested structs, lists, etc.)
    ctx.parse_expr(input)
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use crate::parse_context::ParseContext;
    use partiql_ast::ast;
    use partiql_ast::ast::Lit;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_empty_struct() {
        let expr = parse("{}");
        match &expr {
            ast::Expr::Struct(n) => assert_eq!(n.node.fields.len(), 0),
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_single_field() {
        let expr = parse("{'name': 'Alice'}");
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 1);
                assert!(matches!(
                    &*n.node.fields[0].first,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "name")
                ));
                assert!(matches!(
                    &*n.node.fields[0].second,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "Alice")
                ));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_fields() {
        let expr = parse("{'email': 'a@co', 'age': 30, 'active': true}");
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 3);
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_nested_struct() {
        let expr = parse("{'user': {'name': 'Bob'}}");
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 1);
                assert!(matches!(&*n.node.fields[0].second, ast::Expr::Struct(_)));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_struct_with_list() {
        let expr = parse("{'tags': ['a', 'b'], 'count': 2}");
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 2);
                assert!(matches!(&*n.node.fields[0].second, ast::Expr::List(_)));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_double_quoted_key_is_string_literal() {
        // "key" in struct context is a string literal, not an identifier
        // Values use single quotes (PartiQL string literals)
        let expr = parse(r#"{"name": 'Alice'}"#);
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 1);
                // Key: "name" → CharStringLit("name")
                assert!(matches!(
                    &*n.node.fields[0].first,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "name")
                ));
                // Value: 'Alice' → CharStringLit("Alice")
                assert!(matches!(
                    &*n.node.fields[0].second,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "Alice")
                ));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_double_quoted_value_is_string_literal() {
        // "Alice" as a VALUE in struct context is a string literal (Ion compat)
        let expr = parse(r#"{'name': "Alice"}"#);
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 1);
                assert!(matches!(
                    &*n.node.fields[0].second,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "Alice")
                ));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_unquoted_key_is_string_literal() {
        // Unquoted key in struct context is also a string literal
        let expr = parse("{name: 'Alice'}");
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 1);
                assert!(matches!(
                    &*n.node.fields[0].first,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "name")
                ));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_mixed_key_styles() {
        // Mix of 'single', "double", and unquoted keys
        let expr = parse(r#"{'a': 1, "b": 2, c: 3}"#);
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 3);
                // All keys should be string literals
                for (i, expected) in ["a", "b", "c"].iter().enumerate() {
                    assert!(matches!(
                        &*n.node.fields[i].first,
                        ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == *expected)
                    ), "key {i} should be CharStringLit({expected})");
                }
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_ion_string_key_with_dots() {
        // "fde.users" as a struct key
        let expr = parse(r#"{"fde.users": 'value'}"#);
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 1);
                assert!(matches!(
                    &*n.node.fields[0].first,
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "fde.users")
                ));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn test_fde_insert_pattern() {
        // Real FDE INSERT data pattern
        let expr = parse("{'email': 'user@co', 'platformData': [{'id': 'abc', 'platform': 'MsTeams'}]}");
        match &expr {
            ast::Expr::Struct(n) => {
                assert_eq!(n.node.fields.len(), 2);
                // platformData value is a list containing a struct
                match &*n.node.fields[1].second {
                    ast::Expr::List(list) => {
                        assert_eq!(list.node.values.len(), 1);
                        assert!(matches!(&*list.node.values[0], ast::Expr::Struct(_)));
                    }
                    other => panic!("expected List, got {:?}", other),
                }
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }
}

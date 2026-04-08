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
            let key = ctx.parse_expr(input)?;
            let _ = ws0(input);
            ch(':').parse_next(input)?;
            let _ = ws0(input);
            let value = ctx.parse_expr(input)?;

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

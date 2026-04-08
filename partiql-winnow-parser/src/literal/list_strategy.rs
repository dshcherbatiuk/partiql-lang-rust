//! List constructor — `[expr, expr, ...]`.
//!
//! ```text
//! list ::= '[' expr (',' expr)* ']'
//!        | '[' ']'
//! ```

use partiql_ast::ast;
use winnow::prelude::*;

use super::LiteralStrategy;
use crate::expr::StrategyContext;
use crate::keyword::ch;
use crate::whitespace::ws0;

pub struct ListConstructorStrategy;

impl LiteralStrategy for ListConstructorStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        ch('[').parse_next(input)?;
        let _ = ws0(input);

        // Empty list: []
        if ch(']').parse_next(input).is_ok() {
            return Ok(ast::Expr::List(ctx.node(ast::List {
                values: Vec::new(),
            })));
        }

        let mut values = Vec::new();
        loop {
            let _ = ws0(input);
            let expr = ctx.parse_expr(input)?;
            values.push(Box::new(expr));
            let _ = ws0(input);
            if ch(',').parse_next(input).is_err() {
                break;
            }
        }
        let _ = ws0(input);
        ch(']').parse_next(input)?;

        Ok(ast::Expr::List(ctx.node(ast::List { values })))
    }

    fn name(&self) -> &str {
        "list"
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
    fn test_empty_list() {
        let expr = parse("[]");
        match &expr {
            ast::Expr::List(n) => assert_eq!(n.node.values.len(), 0),
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_list_integers() {
        let expr = parse("[1, 2, 3]");
        match &expr {
            ast::Expr::List(n) => {
                assert_eq!(n.node.values.len(), 3);
                assert!(matches!(
                    &*n.node.values[0],
                    ast::Expr::Lit(l) if matches!(l.node, Lit::Int64Lit(1))
                ));
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_list_strings() {
        let expr = parse("['a', 'b']");
        match &expr {
            ast::Expr::List(n) => {
                assert_eq!(n.node.values.len(), 2);
                assert!(matches!(
                    &*n.node.values[0],
                    ast::Expr::Lit(l) if matches!(&l.node, Lit::CharStringLit(s) if s == "a")
                ));
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_nested_list() {
        let expr = parse("[[1, 2], [3]]");
        match &expr {
            ast::Expr::List(n) => {
                assert_eq!(n.node.values.len(), 2);
                assert!(matches!(&*n.node.values[0], ast::Expr::List(_)));
            }
            other => panic!("expected List, got {:?}", other),
        }
    }
}

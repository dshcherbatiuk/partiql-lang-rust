//! OrStrategy — lowest precedence binary operator.
//!
//! ```text
//! or_expr ::= and_expr (OR and_expr)*
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::kw;
use crate::whitespace::{ws, ws0};

pub struct OrStrategy;

impl ExprStrategy for OrStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let mut left = ctx.parse_next_level(input)?;
        while (ws0, kw("OR"), ws).parse_next(input).is_ok() {
            let right = ctx.parse_next_level(input)?;
            left = ast::Expr::BinOp(ctx.node(BinOp {
                kind: BinOpKind::Or,
                lhs: Box::new(left),
                rhs: Box::new(right),
            }));
        }
        Ok(left)
    }

    fn name(&self) -> &str {
        "Or"
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::BinOpKind;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_simple_or() {
        let expr = parse("1 OR 2");
        assert!(matches!(&expr, ast::Expr::BinOp(n) if n.node.kind == BinOpKind::Or));
    }

    #[test]
    fn test_multiple_ors() {
        // 1 OR 2 OR 3 => left-associative: (1 OR 2) OR 3
        let expr = parse("1 OR 2 OR 3");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Or);
                // lhs should also be an OR
                assert!(
                    matches!(&*n.node.lhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::Or)
                );
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_or_with_and_precedence() {
        // 1 AND 2 OR 3 => (1 AND 2) OR 3
        let expr = parse("1 AND 2 OR 3");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Or);
                assert!(
                    matches!(&*n.node.lhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::And)
                );
            }
            _ => panic!("expected BinOp"),
        }
    }
}

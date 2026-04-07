//! AndStrategy — logical AND.
//!
//! ```text
//! and_expr ::= not_expr (AND not_expr)*
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::kw;
use crate::whitespace::{ws, ws0};

pub struct AndStrategy;

impl ExprStrategy for AndStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let mut left = ctx.parse_next_level(input)?;
        while (ws0, kw("AND"), ws).parse_next(input).is_ok() {
            let right = ctx.parse_next_level(input)?;
            left = ast::Expr::BinOp(ctx.node(BinOp {
                kind: BinOpKind::And,
                lhs: Box::new(left),
                rhs: Box::new(right),
            }));
        }
        Ok(left)
    }

    fn name(&self) -> &str {
        "And"
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::BinOpKind;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let mut i = input;
        chain.parse_expr(&mut i).expect("parse failed")
    }

    #[test]
    fn test_simple_and() {
        let expr = parse("1 AND 2");
        assert!(matches!(&expr, ast::Expr::BinOp(n) if n.node.kind == BinOpKind::And));
    }

    #[test]
    fn test_multiple_ands() {
        // 1 AND 2 AND 3 => left-associative: (1 AND 2) AND 3
        let expr = parse("1 AND 2 AND 3");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::And);
                assert!(
                    matches!(&*n.node.lhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::And)
                );
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_and_with_comparison() {
        // a = 1 AND b = 2
        let expr = parse("a = 1 AND b = 2");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::And);
                assert!(
                    matches!(&*n.node.lhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::Eq)
                );
                assert!(
                    matches!(&*n.node.rhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::Eq)
                );
            }
            _ => panic!("expected BinOp"),
        }
    }
}

//! MulDivStrategy — multiplication, division, modulo.
//!
//! ```text
//! multiply ::= unary (('*' | '/' | '%') unary)*
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::ch;
use crate::whitespace::ws0;

pub struct MulDivStrategy;

impl ExprStrategy for MulDivStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let mut left = ctx.parse_next_level(input)?;
        loop {
            let _ = ws0(input);
            let kind = if ch('*').parse_next(input).is_ok() {
                BinOpKind::Mul
            } else if ch('/').parse_next(input).is_ok() {
                BinOpKind::Div
            } else if ch('%').parse_next(input).is_ok() {
                BinOpKind::Mod
            } else {
                break;
            };
            let _ = ws0(input);
            let right = ctx.parse_next_level(input)?;
            left = ast::Expr::BinOp(ctx.node(BinOp {
                kind,
                lhs: Box::new(left),
                rhs: Box::new(right),
            }));
        }
        Ok(left)
    }

    fn name(&self) -> &str {
        "MulDiv"
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
    fn test_mul() {
        let expr = parse("2 * 3");
        assert!(matches!(&expr, ast::Expr::BinOp(n) if n.node.kind == BinOpKind::Mul));
    }

    #[test]
    fn test_div() {
        let expr = parse("6 / 2");
        assert!(matches!(&expr, ast::Expr::BinOp(n) if n.node.kind == BinOpKind::Div));
    }

    #[test]
    fn test_mod() {
        let expr = parse("7 % 3");
        assert!(matches!(&expr, ast::Expr::BinOp(n) if n.node.kind == BinOpKind::Mod));
    }

    #[test]
    fn test_chained_multiplications() {
        // 2 * 3 * 4 => left-associative: (2 * 3) * 4
        let expr = parse("2 * 3 * 4");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Mul);
                assert!(
                    matches!(&*n.node.lhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::Mul)
                );
            }
            _ => panic!("expected BinOp"),
        }
    }
}

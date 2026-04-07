//! UnaryStrategy — unary minus and plus.
//!
//! ```text
//! unary ::= ['-' | '+'] postfix
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{UniOp, UniOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::ch;
use crate::whitespace::ws0;

pub struct UnaryStrategy;

impl ExprStrategy for UnaryStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        if ch('-').parse_next(input).is_ok() {
            let _ = ws0(input);
            let operand = ctx.parse_next_level(input)?;
            Ok(ast::Expr::UniOp(ctx.node(UniOp {
                kind: UniOpKind::Neg,
                expr: Box::new(operand),
            })))
        } else if ch('+').parse_next(input).is_ok() {
            let _ = ws0(input);
            let operand = ctx.parse_next_level(input)?;
            Ok(ast::Expr::UniOp(ctx.node(UniOp {
                kind: UniOpKind::Pos,
                expr: Box::new(operand),
            })))
        } else {
            ctx.parse_next_level(input)
        }
    }

    fn name(&self) -> &str {
        "Unary"
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::UniOpKind;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let mut i = input;
        chain.parse_expr(&mut i).expect("parse failed")
    }

    #[test]
    fn test_unary_neg() {
        let expr = parse("-5");
        assert!(matches!(&expr, ast::Expr::UniOp(n) if n.node.kind == UniOpKind::Neg));
    }

    #[test]
    fn test_unary_pos() {
        let expr = parse("+5");
        assert!(matches!(&expr, ast::Expr::UniOp(n) if n.node.kind == UniOpKind::Pos));
    }
}

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

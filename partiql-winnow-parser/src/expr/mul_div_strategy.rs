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

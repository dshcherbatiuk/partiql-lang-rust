//! AddSubStrategy — addition, subtraction, string concatenation.
//!
//! ```text
//! addition ::= multiply (('+' | '-' | '||') multiply)*
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::{ch, lit};
use crate::whitespace::ws0;

pub struct AddSubStrategy;

impl ExprStrategy for AddSubStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let mut left = ctx.parse_next_level(input)?;
        loop {
            let _ = ws0(input);
            let kind = if lit("||").parse_next(input).is_ok() {
                BinOpKind::Concat
            } else if ch('+').parse_next(input).is_ok() {
                BinOpKind::Add
            } else if ch('-').parse_next(input).is_ok() {
                BinOpKind::Sub
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
        "AddSub"
    }
}

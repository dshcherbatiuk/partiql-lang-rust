//! NotStrategy — unary NOT prefix.
//!
//! ```text
//! not_expr ::= [NOT] comparison
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{UniOp, UniOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::kw;
use crate::whitespace::ws;

pub struct NotStrategy;

impl ExprStrategy for NotStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        if (kw("NOT"), ws).parse_next(input).is_ok() {
            let operand = ctx.parse_next_level(input)?;
            Ok(ast::Expr::UniOp(ctx.node(UniOp {
                kind: UniOpKind::Not,
                expr: Box::new(operand),
            })))
        } else {
            ctx.parse_next_level(input)
        }
    }

    fn name(&self) -> &str {
        "Not"
    }
}

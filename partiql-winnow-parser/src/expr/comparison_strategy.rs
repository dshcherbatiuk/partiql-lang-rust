//! ComparisonStrategy — comparison operators and special forms.
//!
//! ```text
//! comparison ::= addition (comp_op addition)?
//!             | addition IS [NOT] NULL
//!             | addition [NOT] IN collection
//!             | addition [NOT] LIKE pattern
//!             | addition [NOT] BETWEEN low AND high
//! comp_op    ::= '=' | '!=' | '<>' | '<' | '>' | '<=' | '>='
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::lit;
use crate::whitespace::ws0;

pub struct ComparisonStrategy;

impl ExprStrategy for ComparisonStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let left = ctx.parse_next_level(input)?;
        let _ = ws0(input);

        // TODO: IS [NOT] NULL, [NOT] IN, [NOT] LIKE, [NOT] BETWEEN

        // Comparison operators: = != <> < > <= >=
        if let Ok(kind) = alt((
            lit("!=").map(|_| BinOpKind::Ne),
            lit("<>").map(|_| BinOpKind::Ne),
            lit("<=").map(|_| BinOpKind::Lte),
            lit(">=").map(|_| BinOpKind::Gte),
            lit("=").map(|_| BinOpKind::Eq),
            lit("<").map(|_| BinOpKind::Lt),
            lit(">").map(|_| BinOpKind::Gt),
        ))
        .parse_next(input)
        {
            let _ = ws0(input);
            let right = ctx.parse_next_level(input)?;
            return Ok(ast::Expr::BinOp(ctx.node(BinOp {
                kind,
                lhs: Box::new(left),
                rhs: Box::new(right),
            })));
        }

        Ok(left)
    }

    fn name(&self) -> &str {
        "Comparison"
    }
}

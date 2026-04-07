//! BooleanLiteralStrategy — `true`, `false` (case-insensitive).

use super::LiteralStrategy;
use crate::expr::StrategyContext;
use crate::keyword::kw;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::combinator::alt;
use winnow::prelude::*;

pub struct BooleanLiteralStrategy;

impl LiteralStrategy for BooleanLiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let b = alt((kw("true").map(|_| true), kw("false").map(|_| false))).parse_next(input)?;

        // Word boundary: reject `trueish`, `falsehood`
        if input
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        Ok(ast::Expr::Lit(ctx.node(Lit::BoolLit(b))))
    }

    fn name(&self) -> &str {
        "BooleanLiteral"
    }
}

//! NullMissingStrategy — `null`, `null.int`, `MISSING`.

use super::ion::null;
use super::LiteralStrategy;
use crate::expr::StrategyContext;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::prelude::*;

pub struct NullMissingStrategy;

impl LiteralStrategy for NullMissingStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        if null::missing(input).is_ok() {
            return Ok(ast::Expr::Lit(ctx.node(Lit::Missing)));
        }
        let _ = null::ion_null(input)?;
        Ok(ast::Expr::Lit(ctx.node(Lit::Null)))
    }

    fn name(&self) -> &str {
        "NullMissing"
    }
}

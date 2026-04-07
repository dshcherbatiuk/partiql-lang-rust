//! StringLiteralStrategy — SQL `'hello'` or Ion `"hello"`.

use super::ion_string;
use super::LiteralStrategy;
use crate::expr::StrategyContext;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::combinator::alt;
use winnow::prelude::*;

pub struct StringLiteralStrategy;

impl LiteralStrategy for StringLiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let s = alt((ion_string::sql_string, ion_string::ion_string)).parse_next(input)?;
        Ok(ast::Expr::Lit(ctx.node(Lit::CharStringLit(s))))
    }

    fn name(&self) -> &str {
        "StringLiteral"
    }
}

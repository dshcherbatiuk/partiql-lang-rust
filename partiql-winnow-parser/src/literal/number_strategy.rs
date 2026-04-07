//! NumericLiteralStrategy — integer, decimal, float.

use super::ion_number;
use super::LiteralStrategy;
use crate::expr::StrategyContext;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::prelude::*;

pub struct NumericLiteralStrategy;

impl LiteralStrategy for NumericLiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let num = ion_number::ion_number(input)?;
        Ok(match num {
            ion_number::IonNumber::Integer(n) => ast::Expr::Lit(ctx.node(Lit::Int64Lit(n))),
            ion_number::IonNumber::Decimal(d) => ast::Expr::Lit(ctx.node(Lit::DecimalLit(d))),
            ion_number::IonNumber::Float(f) => ast::Expr::Lit(ctx.node(Lit::DoubleLit(f))),
        })
    }

    fn name(&self) -> &str {
        "NumericLiteral"
    }
}

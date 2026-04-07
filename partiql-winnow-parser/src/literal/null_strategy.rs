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

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::Lit;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn null_generic() {
        let expr = parse("null");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Null)));
    }

    #[test]
    fn null_typed_int() {
        let expr = parse("null.int");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Null)));
    }

    #[test]
    fn null_typed_string() {
        let expr = parse("null.string");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Null)));
    }

    #[test]
    fn missing_keyword() {
        let expr = parse("MISSING");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Missing)));
    }
}

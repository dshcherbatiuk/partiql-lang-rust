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

    fn try_parse(input: &str) -> Result<ast::Expr, ()> {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).map_err(|_| ())
    }

    #[test]
    fn true_lowercase() {
        let expr = parse("true");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::BoolLit(true))));
    }

    #[test]
    fn false_lowercase() {
        let expr = parse("false");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::BoolLit(false))));
    }

    #[test]
    fn true_uppercase() {
        let expr = parse("TRUE");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::BoolLit(true))));
    }

    #[test]
    fn false_uppercase() {
        let expr = parse("FALSE");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::BoolLit(false))));
    }

    #[test]
    fn trueish_rejected() {
        // "trueish" should NOT parse as boolean true — word boundary enforcement
        let result = try_parse("trueish");
        // It may parse as an identifier instead, or fail entirely.
        // The key assertion: it must NOT be BoolLit(true).
        match result {
            Ok(ast::Expr::Lit(n)) => {
                assert!(
                    !matches!(n.node, Lit::BoolLit(true)),
                    "trueish should not parse as true"
                );
            }
            _ => {} // identifier or error — both acceptable
        }
    }
}

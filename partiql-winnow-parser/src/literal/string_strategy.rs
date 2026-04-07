//! StringLiteralStrategy — SQL `'hello'` or Ion `"hello"`.

use super::ion::string;
use super::LiteralStrategy;
use crate::expr::StrategyContext;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::combinator::alt;
use winnow::prelude::*;

pub struct StringLiteralStrategy;

impl LiteralStrategy for StringLiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let s = alt((string::sql_string, string::ion_string)).parse_next(input)?;
        Ok(ast::Expr::Lit(ctx.node(Lit::CharStringLit(s))))
    }

    fn name(&self) -> &str {
        "StringLiteral"
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
    fn sql_string_simple() {
        let expr = parse("'hello'");
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == "hello"))
        );
    }

    #[test]
    fn ion_string_simple() {
        let expr = parse(r#""hello""#);
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == "hello"))
        );
    }

    #[test]
    fn sql_string_escaped_quote() {
        let expr = parse("'it''s'");
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == "it's"))
        );
    }

    #[test]
    fn ion_string_escaped_quote() {
        let expr = parse(r#""say \"hi\"""#);
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == r#"say "hi""#))
        );
    }

    #[test]
    fn sql_string_empty() {
        let expr = parse("''");
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s.is_empty()))
        );
    }

    #[test]
    fn ion_string_empty() {
        let expr = parse(r#""""#);
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s.is_empty()))
        );
    }
}

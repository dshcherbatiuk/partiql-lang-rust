//! StringLiteralStrategy — SQL single-quoted strings only: `'hello'`.
//!
//! Double-quoted strings (`"fde.users"`) are **case-sensitive identifiers**
//! in PartiQL, not string literals. They are handled by the identifier
//! parser in `PrimaryStrategy`, producing `VarRef` with `CaseSensitive`.

use super::ion::string;
use super::LiteralStrategy;
use crate::expr::StrategyContext;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::prelude::*;

pub struct StringLiteralStrategy;

impl LiteralStrategy for StringLiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let s = string::sql_string.parse_next(input)?;
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
    fn double_quoted_is_case_sensitive_varref() {
        // Double-quoted strings are case-sensitive identifiers in PartiQL, not string literals
        let expr = parse(r#""hello""#);
        assert!(matches!(
            &expr,
            ast::Expr::VarRef(n) if n.node.name.value == "hello"
                && n.node.name.case == ast::CaseSensitivity::CaseSensitive
        ));
    }

    #[test]
    fn double_quoted_dotted_table_name() {
        // "fde.users" is a case-sensitive identifier (the dot is part of the name)
        let expr = parse(r#""fde.users""#);
        assert!(matches!(
            &expr,
            ast::Expr::VarRef(n) if n.node.name.value == "fde.users"
                && n.node.name.case == ast::CaseSensitivity::CaseSensitive
        ));
    }

    #[test]
    fn sql_string_escaped_quote() {
        let expr = parse("'it''s'");
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == "it's"))
        );
    }

    #[test]
    fn sql_string_empty() {
        let expr = parse("''");
        assert!(
            matches!(expr, ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s.is_empty()))
        );
    }

}

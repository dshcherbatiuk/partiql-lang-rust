//! NumericLiteralStrategy — integer, decimal, float.

use super::ion::number;
use super::LiteralStrategy;
use crate::expr::StrategyContext;
use partiql_ast::ast;
use partiql_ast::ast::Lit;
use winnow::prelude::*;

pub struct NumericLiteralStrategy;

impl LiteralStrategy for NumericLiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let num = number::ion_number(input)?;
        Ok(match num {
            number::IonNumber::Integer(n) => ast::Expr::Lit(ctx.node(Lit::Int64Lit(n))),
            number::IonNumber::Decimal(d) => ast::Expr::Lit(ctx.node(Lit::DecimalLit(d))),
            number::IonNumber::Float(f) => ast::Expr::Lit(ctx.node(Lit::DoubleLit(f))),
        })
    }

    fn name(&self) -> &str {
        "NumericLiteral"
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
    fn integer() {
        let expr = parse("42");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Int64Lit(42))));
    }

    #[test]
    fn decimal() {
        let expr = parse("3.14");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::DecimalLit(_))));
    }

    #[test]
    fn float() {
        let expr = parse("1.0e0");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::DoubleLit(_))));
    }

    #[test]
    fn hex_integer() {
        let expr = parse("0xFACE");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Int64Lit(0xFACE))));
    }

    #[test]
    fn binary_integer() {
        let expr = parse("0b1010");
        assert!(matches!(expr, ast::Expr::Lit(n) if matches!(n.node, Lit::Int64Lit(0b1010))));
    }

    #[test]
    fn negative_integer() {
        // Negative numbers go through unary minus, so the literal is positive
        // and wrapped in a unary negation. We verify the expression parses.
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = "-42";
        let expr = chain.parse_expr(&mut i, &pctx).expect("parse failed");
        // -42 may parse as unary minus on 42, or as a negative literal
        match expr {
            ast::Expr::Lit(n) => assert!(matches!(n.node, Lit::Int64Lit(-42))),
            ast::Expr::UniOp(ref op) => assert!(matches!(op.node.kind, ast::UniOpKind::Neg)),
            _ => panic!("Expected Lit or UniOp, got {:?}", expr),
        }
    }
}

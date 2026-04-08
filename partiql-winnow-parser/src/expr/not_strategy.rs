//! NotStrategy — unary NOT prefix.
//!
//! ```text
//! not_expr ::= [NOT] comparison
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{UniOp, UniOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::kw;
use crate::whitespace::ws;

pub struct NotStrategy;

impl ExprStrategy for NotStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        if (kw("NOT"), ws).parse_next(input).is_ok() {
            let operand = ctx.parse_next_level(input)?;
            Ok(ast::Expr::UniOp(ctx.node(UniOp {
                kind: UniOpKind::Not,
                expr: Box::new(operand),
            })))
        } else {
            ctx.parse_next_level(input)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::UniOpKind;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_not() {
        let expr = parse("NOT true");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, UniOpKind::Not);
                assert!(matches!(
                    &*n.node.expr,
                    ast::Expr::Lit(lit) if matches!(lit.node, ast::Lit::BoolLit(true))
                ));
            }
            _ => panic!("expected UniOp"),
        }
    }

    #[test]
    fn test_not_parenthesized_not() {
        // NOT (NOT true) => NOT (NOT true) via parenthesized sub-expression
        let expr = parse("NOT (NOT true)");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, UniOpKind::Not);
                assert!(
                    matches!(&*n.node.expr, ast::Expr::UniOp(inner) if inner.node.kind == UniOpKind::Not)
                );
            }
            _ => panic!("expected UniOp"),
        }
    }

    #[test]
    fn test_not_with_comparison() {
        // NOT a = 1 => NOT (a = 1)
        let expr = parse("NOT a = 1");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, UniOpKind::Not);
                assert!(matches!(
                    &*n.node.expr,
                    ast::Expr::BinOp(inner) if inner.node.kind == ast::BinOpKind::Eq
                ));
            }
            _ => panic!("expected UniOp"),
        }
    }
}

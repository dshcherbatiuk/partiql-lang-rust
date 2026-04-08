//! AddSubStrategy — addition, subtraction, string concatenation.
//!
//! ```text
//! addition ::= multiply (('+' | '-' | '||') multiply)*
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::keyword::{ch, lit};
use crate::whitespace::ws0;

pub struct AddSubStrategy;

impl ExprStrategy for AddSubStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let mut left = ctx.parse_next_level(input)?;
        loop {
            let _ = ws0(input);
            let kind = if lit("||").parse_next(input).is_ok() {
                BinOpKind::Concat
            } else if ch('+').parse_next(input).is_ok() {
                BinOpKind::Add
            } else if ch('-').parse_next(input).is_ok() {
                BinOpKind::Sub
            } else {
                break;
            };
            let _ = ws0(input);
            let right = ctx.parse_next_level(input)?;
            left = ast::Expr::BinOp(ctx.node(BinOp {
                kind,
                lhs: Box::new(left),
                rhs: Box::new(right),
            }));
        }
        Ok(left)
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::{BinOpKind, Lit};

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_add() {
        let expr = parse("1 + 2");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Add);
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(1))
                ));
                assert!(matches!(
                    &*n.node.rhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(2))
                ));
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_sub() {
        let expr = parse("3 - 1");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Sub);
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(3))
                ));
                assert!(matches!(
                    &*n.node.rhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(1))
                ));
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_concat() {
        let expr = parse("'a' || 'b'");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Concat);
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::Lit(lit) if matches!(&lit.node, Lit::CharStringLit(s) if s == "a")
                ));
                assert!(matches!(
                    &*n.node.rhs,
                    ast::Expr::Lit(lit) if matches!(&lit.node, Lit::CharStringLit(s) if s == "b")
                ));
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_chained_additions() {
        // 1 + 2 + 3 => left-associative: (1 + 2) + 3
        let expr = parse("1 + 2 + 3");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Add);
                assert!(
                    matches!(&*n.node.lhs, ast::Expr::BinOp(inner) if inner.node.kind == BinOpKind::Add)
                );
            }
            _ => panic!("expected BinOp"),
        }
    }
}

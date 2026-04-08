//! IS [NOT] NULL / IS [NOT] MISSING parser.
//!
//! ```text
//! is_expr ::= expr IS [NOT] NULL
//!           | expr IS [NOT] MISSING
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind, Lit, UniOp, UniOpKind};
use winnow::prelude::*;

use super::ComparisonParser;
use crate::expr::StrategyContext;
use crate::keyword::kw;
use crate::whitespace::ws;

pub struct IsParser;

impl ComparisonParser for IsParser {
    fn parse<'a>(
        &self,
        input: &mut &'a str,
        ctx: &StrategyContext<'_>,
        left: &ast::Expr,
    ) -> PResult<ast::Expr> {
        let checkpoint = *input;

        if (kw("IS"), ws).parse_next(input).is_err() {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        let negated = (kw("NOT"), ws).parse_next(input).is_ok();

        let rhs = if kw("NULL").parse_next(input).is_ok() {
            ast::Expr::Lit(ctx.node(Lit::Null))
        } else if kw("MISSING").parse_next(input).is_ok() {
            ast::Expr::Lit(ctx.node(Lit::Missing))
        } else {
            *input = checkpoint;
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        };

        let expr = ast::Expr::BinOp(ctx.node(BinOp {
            kind: BinOpKind::Is,
            lhs: Box::new(left.clone()),
            rhs: Box::new(rhs),
        }));

        if negated {
            Ok(ast::Expr::UniOp(ctx.node(UniOp {
                kind: UniOpKind::Not,
                expr: Box::new(expr),
            })))
        } else {
            Ok(expr)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use crate::parse_context::ParseContext;
    use partiql_ast::ast;
    use partiql_ast::ast::BinOpKind;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_is_null() {
        let expr = parse("x IS NULL");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Is);
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::VarRef(v) if v.node.name.value == "x"
                ));
                assert!(matches!(
                    &*n.node.rhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, ast::Lit::Null)
                ));
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_is_not_null() {
        let expr = parse("x IS NOT NULL");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, ast::UniOpKind::Not);
                match &*n.node.expr {
                    ast::Expr::BinOp(b) => {
                        assert_eq!(b.node.kind, BinOpKind::Is);
                        assert!(matches!(
                            &*b.node.lhs,
                            ast::Expr::VarRef(v) if v.node.name.value == "x"
                        ));
                        assert!(matches!(
                            &*b.node.rhs,
                            ast::Expr::Lit(lit) if matches!(lit.node, ast::Lit::Null)
                        ));
                    }
                    _ => panic!("expected BinOp inside UniOp"),
                }
            }
            _ => panic!("expected UniOp"),
        }
    }

    #[test]
    fn test_is_missing() {
        let expr = parse("x IS MISSING");
        match &expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Is);
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::VarRef(v) if v.node.name.value == "x"
                ));
                assert!(matches!(
                    &*n.node.rhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, ast::Lit::Missing)
                ));
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_is_not_missing() {
        let expr = parse("x IS NOT MISSING");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, ast::UniOpKind::Not);
                match &*n.node.expr {
                    ast::Expr::BinOp(b) => {
                        assert_eq!(b.node.kind, BinOpKind::Is);
                        assert!(matches!(
                            &*b.node.lhs,
                            ast::Expr::VarRef(v) if v.node.name.value == "x"
                        ));
                        assert!(matches!(
                            &*b.node.rhs,
                            ast::Expr::Lit(lit) if matches!(lit.node, ast::Lit::Missing)
                        ));
                    }
                    _ => panic!("expected BinOp inside UniOp"),
                }
            }
            _ => panic!("expected UniOp"),
        }
    }
}

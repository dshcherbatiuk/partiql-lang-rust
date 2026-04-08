//! [NOT] LIKE parser.
//!
//! ```text
//! like_expr ::= expr [NOT] LIKE pattern [ESCAPE escape_char]
//! ```

use partiql_ast::ast;
use partiql_ast::ast::Like;
use winnow::prelude::*;

use super::ComparisonParser;
use crate::expr::StrategyContext;
use crate::keyword::kw;
use crate::whitespace::{ws, ws0};

pub struct LikeParser;

impl ComparisonParser for LikeParser {
    fn parse<'a>(
        &self,
        input: &mut &'a str,
        ctx: &StrategyContext<'_>,
        left: &ast::Expr,
    ) -> PResult<ast::Expr> {
        let checkpoint = *input;

        if (kw("LIKE"), ws).parse_next(input).is_err() {
            *input = checkpoint;
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        let pattern = ctx.parse_next_level(input)?;
        let _ = ws0(input);

        let escape = if (kw("ESCAPE"), ws).parse_next(input).is_ok() {
            Some(Box::new(ctx.parse_next_level(input)?))
        } else {
            None
        };

        Ok(ast::Expr::Like(ctx.node(Like {
            value: Box::new(left.clone()),
            pattern: Box::new(pattern),
            escape,
        })))
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use crate::parse_context::ParseContext;
    use partiql_ast::ast;
    use partiql_ast::ast::Lit;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_like() {
        let expr = parse("name LIKE '%foo%'");
        match &expr {
            ast::Expr::Like(n) => {
                assert!(matches!(
                    &*n.node.value,
                    ast::Expr::VarRef(v) if v.node.name.value == "name"
                ));
                assert!(matches!(
                    &*n.node.pattern,
                    ast::Expr::Lit(lit) if matches!(&lit.node, Lit::CharStringLit(s) if s == "%foo%")
                ));
            }
            _ => panic!("expected Like"),
        }
    }

    #[test]
    fn test_like_escape() {
        let expr = parse("name LIKE '%\\%%' ESCAPE '\\'");
        assert!(matches!(&expr, ast::Expr::Like(_)));
        if let ast::Expr::Like(n) = &expr {
            assert!(n.node.escape.is_some());
        }
    }

    #[test]
    fn test_not_like() {
        let expr = parse("name NOT LIKE '%foo'");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, ast::UniOpKind::Not);
                match &*n.node.expr {
                    ast::Expr::Like(like) => {
                        assert!(matches!(
                            &*like.node.value,
                            ast::Expr::VarRef(v) if v.node.name.value == "name"
                        ));
                    }
                    _ => panic!("expected Like inside UniOp"),
                }
            }
            _ => panic!("expected UniOp"),
        }
    }
}

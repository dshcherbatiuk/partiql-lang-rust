//! PostfixStrategy — path access (dot, bracket).
//!
//! ```text
//! postfix ::= primary ('.' identifier | '[' expr ']')*
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{Lit, Path, PathExpr, PathStep};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::identifier;
use crate::keyword::ch;
use crate::whitespace::ws0;

pub struct PostfixStrategy;

impl ExprStrategy for PostfixStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let base = ctx.parse_next_level(input)?;
        let mut steps: Vec<PathStep> = Vec::new();

        loop {
            if ch('.').parse_next(input).is_ok() {
                let field = identifier::identifier(input)?;
                steps.push(PathStep::PathProject(PathExpr {
                    index: Box::new(ast::Expr::Lit(ctx.node(Lit::CharStringLit(field)))),
                }));
            } else if ch('[').parse_next(input).is_ok() {
                let _ = ws0(input);
                let index_expr = ctx.parse_expr(input)?;
                let _ = ws0(input);
                let _ = ch(']').parse_next(input)?;
                steps.push(PathStep::PathIndex(PathExpr {
                    index: Box::new(index_expr),
                }));
            } else {
                break;
            }
        }

        if steps.is_empty() {
            Ok(base)
        } else {
            Ok(ast::Expr::Path(ctx.node(Path {
                root: Box::new(base),
                steps,
            })))
        }
    }

    fn name(&self) -> &str {
        "Postfix"
    }
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::{Lit, PathStep};

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_dot_access() {
        // a.b => Path with one step
        let expr = parse("a.b");
        match &expr {
            ast::Expr::Path(n) => {
                assert!(matches!(
                    &*n.node.root,
                    ast::Expr::VarRef(v) if v.node.name.value == "a"
                ));
                assert_eq!(n.node.steps.len(), 1);
                match &n.node.steps[0] {
                    PathStep::PathProject(pe) => {
                        assert!(matches!(
                            &*pe.index,
                            ast::Expr::Lit(lit) if matches!(&lit.node, Lit::CharStringLit(s) if s == "b")
                        ));
                    }
                    _ => panic!("expected PathProject"),
                }
            }
            _ => panic!("expected Path"),
        }
    }

    #[test]
    fn test_chained_dot_access() {
        // a.b.c => Path with two steps
        let expr = parse("a.b.c");
        match &expr {
            ast::Expr::Path(n) => {
                assert_eq!(n.node.steps.len(), 2);
            }
            _ => panic!("expected Path"),
        }
    }

    #[test]
    fn test_bracket_access() {
        // a[0] => Path with one index step
        let expr = parse("a[0]");
        match &expr {
            ast::Expr::Path(n) => {
                assert!(matches!(
                    &*n.node.root,
                    ast::Expr::VarRef(v) if v.node.name.value == "a"
                ));
                assert_eq!(n.node.steps.len(), 1);
                assert!(matches!(&n.node.steps[0], PathStep::PathIndex(_)));
            }
            _ => panic!("expected Path"),
        }
    }

    #[test]
    fn test_mixed_access() {
        // a.b[0].c => Path with three steps
        let expr = parse("a.b[0].c");
        match &expr {
            ast::Expr::Path(n) => {
                assert_eq!(n.node.steps.len(), 3);
            }
            _ => panic!("expected Path"),
        }
    }
}

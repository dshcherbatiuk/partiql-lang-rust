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

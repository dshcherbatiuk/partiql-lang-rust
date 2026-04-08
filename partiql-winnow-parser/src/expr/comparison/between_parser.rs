//! [NOT] BETWEEN parser.
//!
//! ```text
//! between_expr ::= expr [NOT] BETWEEN low AND high
//! ```

use partiql_ast::ast;
use partiql_ast::ast::Between;
use winnow::prelude::*;

use super::ComparisonParser;
use crate::expr::StrategyContext;
use crate::keyword::kw;
use crate::whitespace::{ws, ws0};

pub struct BetweenParser;

impl ComparisonParser for BetweenParser {
    fn parse<'a>(
        &self,
        input: &mut &'a str,
        ctx: &StrategyContext<'_>,
        left: &ast::Expr,
    ) -> PResult<ast::Expr> {
        let checkpoint = *input;

        if (kw("BETWEEN"), ws).parse_next(input).is_err() {
            *input = checkpoint;
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        let from = ctx.parse_next_level(input)?;
        let _ = ws0(input);
        (kw("AND"), ws).parse_next(input)?;
        let to = ctx.parse_next_level(input)?;

        Ok(ast::Expr::Between(ctx.node(Between {
            value: Box::new(left.clone()),
            from: Box::new(from),
            to: Box::new(to),
        })))
    }

    fn name(&self) -> &str {
        "BETWEEN"
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
    fn test_between() {
        let expr = parse("x BETWEEN 1 AND 10");
        match &expr {
            ast::Expr::Between(n) => {
                assert!(matches!(
                    &*n.node.value,
                    ast::Expr::VarRef(v) if v.node.name.value == "x"
                ));
                assert!(matches!(
                    &*n.node.from,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(1))
                ));
                assert!(matches!(
                    &*n.node.to,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(10))
                ));
            }
            _ => panic!("expected Between"),
        }
    }

    #[test]
    fn test_between_expressions() {
        let expr = parse("age BETWEEN 18 AND 65");
        match &expr {
            ast::Expr::Between(n) => {
                assert!(matches!(
                    &*n.node.value,
                    ast::Expr::VarRef(v) if v.node.name.value == "age"
                ));
                assert!(matches!(
                    &*n.node.from,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(18))
                ));
                assert!(matches!(
                    &*n.node.to,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(65))
                ));
            }
            _ => panic!("expected Between"),
        }
    }

    #[test]
    fn test_not_between() {
        let expr = parse("x NOT BETWEEN 1 AND 10");
        assert!(matches!(&expr, ast::Expr::UniOp(_)));
        if let ast::Expr::UniOp(n) = &expr {
            assert!(matches!(&*n.node.expr, ast::Expr::Between(_)));
        }
    }
}

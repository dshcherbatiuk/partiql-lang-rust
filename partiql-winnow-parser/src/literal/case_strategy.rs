//! CASE expression parser.
//!
//! ```text
//! case_expr     ::= simple_case | searched_case
//! simple_case   ::= CASE expr (WHEN expr THEN expr)+ [ELSE expr] END
//! searched_case ::= CASE (WHEN expr THEN expr)+ [ELSE expr] END
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{Case, ExprPair, SearchedCase, SimpleCase};
use winnow::prelude::*;

use super::LiteralStrategy;
use crate::expr::StrategyContext;
use crate::keyword::kw;
use crate::whitespace::{ws, ws0};

pub struct CaseExprStrategy;

impl LiteralStrategy for CaseExprStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        (kw("CASE"), ws).parse_next(input)?;

        // Try searched case first: CASE WHEN ...
        // If the next token is WHEN, it's a searched case.
        // Otherwise it's a simple case: CASE expr WHEN ...
        let checkpoint = *input;
        if (kw("WHEN"), ws).parse_next(input).is_ok() {
            *input = checkpoint;
            parse_searched_case(input, ctx)
        } else {
            *input = checkpoint;
            parse_simple_case(input, ctx)
        }
    }
}

/// CASE expr WHEN expr THEN expr [WHEN ...] [ELSE expr] END
fn parse_simple_case<'a>(
    input: &mut &'a str,
    ctx: &StrategyContext<'_>,
) -> PResult<ast::Expr> {
    let expr = ctx.parse_expr(input)?;
    let _ = ws0(input);

    let mut cases = Vec::new();
    while (kw("WHEN"), ws).parse_next(input).is_ok() {
        let when_expr = ctx.parse_expr(input)?;
        let _ = ws0(input);
        (kw("THEN"), ws).parse_next(input)?;
        let then_expr = ctx.parse_expr(input)?;
        let _ = ws0(input);
        cases.push(ExprPair {
            first: Box::new(when_expr),
            second: Box::new(then_expr),
        });
    }

    let default = if (kw("ELSE"), ws).parse_next(input).is_ok() {
        let else_expr = ctx.parse_expr(input)?;
        let _ = ws0(input);
        Some(Box::new(else_expr))
    } else {
        None
    };

    kw("END").parse_next(input)?;

    Ok(ast::Expr::Case(ctx.node(Case::SimpleCase(SimpleCase {
        expr: Box::new(expr),
        cases,
        default,
    }))))
}

/// CASE WHEN expr THEN expr [WHEN ...] [ELSE expr] END
fn parse_searched_case<'a>(
    input: &mut &'a str,
    ctx: &StrategyContext<'_>,
) -> PResult<ast::Expr> {
    let mut cases = Vec::new();
    while (kw("WHEN"), ws).parse_next(input).is_ok() {
        let when_expr = ctx.parse_expr(input)?;
        let _ = ws0(input);
        (kw("THEN"), ws).parse_next(input)?;
        let then_expr = ctx.parse_expr(input)?;
        let _ = ws0(input);
        cases.push(ExprPair {
            first: Box::new(when_expr),
            second: Box::new(then_expr),
        });
    }

    let default = if (kw("ELSE"), ws).parse_next(input).is_ok() {
        let else_expr = ctx.parse_expr(input)?;
        let _ = ws0(input);
        Some(Box::new(else_expr))
    } else {
        None
    };

    kw("END").parse_next(input)?;

    Ok(ast::Expr::Case(ctx.node(Case::SearchedCase(
        SearchedCase { cases, default },
    ))))
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use crate::parse_context::ParseContext;
    use partiql_ast::ast;
    use partiql_ast::ast::Case;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_searched_case() {
        let expr = parse("CASE WHEN x = 1 THEN 'one' WHEN x = 2 THEN 'two' END");
        match &expr {
            ast::Expr::Case(n) => match &n.node {
                Case::SearchedCase(sc) => {
                    assert_eq!(sc.cases.len(), 2);
                    assert!(sc.default.is_none());
                }
                other => panic!("expected SearchedCase, got {:?}", other),
            },
            other => panic!("expected Case, got {:?}", other),
        }
    }

    #[test]
    fn test_searched_case_with_else() {
        let expr = parse("CASE WHEN status = 'active' THEN 'yes' ELSE 'no' END");
        match &expr {
            ast::Expr::Case(n) => match &n.node {
                Case::SearchedCase(sc) => {
                    assert_eq!(sc.cases.len(), 1);
                    assert!(sc.default.is_some());
                }
                other => panic!("expected SearchedCase, got {:?}", other),
            },
            other => panic!("expected Case, got {:?}", other),
        }
    }

    #[test]
    fn test_simple_case() {
        let expr = parse("CASE status WHEN 'active' THEN 1 WHEN 'inactive' THEN 0 ELSE -1 END");
        match &expr {
            ast::Expr::Case(n) => match &n.node {
                Case::SimpleCase(sc) => {
                    assert_eq!(sc.cases.len(), 2);
                    assert!(sc.default.is_some());
                    assert!(matches!(&*sc.expr, ast::Expr::VarRef(_)));
                }
                other => panic!("expected SimpleCase, got {:?}", other),
            },
            other => panic!("expected Case, got {:?}", other),
        }
    }
}

//! Bag constructor — `<<expr, expr, ...>>`.
//!
//! ```text
//! bag ::= '<<' expr (',' expr)* '>>'
//!       | '<<' '>>'
//! ```

use partiql_ast::ast;
use winnow::prelude::*;

use super::LiteralStrategy;
use crate::expr::StrategyContext;
use crate::keyword::lit;
use crate::whitespace::ws0;

pub struct BagConstructorStrategy;

impl LiteralStrategy for BagConstructorStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        lit("<<").parse_next(input)?;
        let _ = ws0(input);

        // Empty bag: << >>
        if lit(">>").parse_next(input).is_ok() {
            return Ok(ast::Expr::Bag(ctx.node(ast::Bag {
                values: Vec::new(),
            })));
        }

        let mut values = Vec::new();
        loop {
            let _ = ws0(input);
            let expr = ctx.parse_expr(input)?;
            values.push(Box::new(expr));
            let _ = ws0(input);
            if winnow::token::one_of::<_, _, winnow::error::ContextError>(',')
                .parse_next(input)
                .is_err()
            {
                break;
            }
        }
        let _ = ws0(input);
        lit(">>").parse_next(input)?;

        Ok(ast::Expr::Bag(ctx.node(ast::Bag { values })))
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
    fn test_empty_bag() {
        let expr = parse("<< >>");
        match &expr {
            ast::Expr::Bag(n) => assert_eq!(n.node.values.len(), 0),
            other => panic!("expected Bag, got {:?}", other),
        }
    }

    #[test]
    fn test_bag_integers() {
        let expr = parse("<<1, 2, 3>>");
        match &expr {
            ast::Expr::Bag(n) => {
                assert_eq!(n.node.values.len(), 3);
                assert!(matches!(
                    &*n.node.values[0],
                    ast::Expr::Lit(l) if matches!(l.node, Lit::Int64Lit(1))
                ));
            }
            other => panic!("expected Bag, got {:?}", other),
        }
    }

    #[test]
    fn test_bag_with_struct() {
        // FDE INSERT pattern: << {'email': 'a@co'} >>
        let expr = parse("<<{'email': 'a@co'}>>");
        match &expr {
            ast::Expr::Bag(n) => {
                assert_eq!(n.node.values.len(), 1);
                assert!(matches!(&*n.node.values[0], ast::Expr::Struct(_)));
            }
            other => panic!("expected Bag, got {:?}", other),
        }
    }
}

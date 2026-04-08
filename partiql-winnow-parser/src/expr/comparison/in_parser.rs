//! [NOT] IN parser.
//!
//! ```text
//! in_expr ::= expr [NOT] IN '(' expr (',' expr)* ')'
//! ```

use partiql_ast::ast;
use partiql_ast::ast::In;
use winnow::prelude::*;

use super::ComparisonParser;
use crate::expr::StrategyContext;
use crate::keyword::{ch, kw};
use crate::whitespace::ws0;

pub struct InParser;

impl ComparisonParser for InParser {
    fn parse<'a>(
        &self,
        input: &mut &'a str,
        ctx: &StrategyContext<'_>,
        left: &ast::Expr,
    ) -> PResult<ast::Expr> {
        let checkpoint = *input;

        if (kw("IN"), ws0).parse_next(input).is_err() {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        if ch('(').parse_next(input).is_err() {
            *input = checkpoint;
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        let mut items = Vec::new();
        loop {
            let _ = ws0(input);
            let expr = ctx.parse_expr(input)?;
            items.push(expr);
            let _ = ws0(input);
            if ch(',').parse_next(input).is_err() {
                break;
            }
        }
        let _ = ws0(input);
        ch(')').parse_next(input)?;

        let rhs = ast::Expr::List(ctx.node(ast::List {
            values: items.into_iter().map(|e| Box::new(e)).collect(),
        }));

        Ok(ast::Expr::In(ctx.node(In {
            lhs: Box::new(left.clone()),
            rhs: Box::new(rhs),
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
    fn test_in_list() {
        let expr = parse("x IN (1, 2, 3)");
        match &expr {
            ast::Expr::In(n) => {
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::VarRef(v) if v.node.name.value == "x"
                ));
                match &*n.node.rhs {
                    ast::Expr::List(list) => assert_eq!(list.node.values.len(), 3),
                    _ => panic!("expected List rhs"),
                }
            }
            _ => panic!("expected In"),
        }
    }

    #[test]
    fn test_in_single() {
        let expr = parse("x IN (1)");
        match &expr {
            ast::Expr::In(n) => {
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::VarRef(v) if v.node.name.value == "x"
                ));
                match &*n.node.rhs {
                    ast::Expr::List(list) => assert_eq!(list.node.values.len(), 1),
                    _ => panic!("expected List rhs"),
                }
            }
            _ => panic!("expected In"),
        }
    }

    #[test]
    fn test_in_strings() {
        let expr = parse("name IN ('Alice', 'Bob')");
        match &expr {
            ast::Expr::In(n) => {
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::VarRef(v) if v.node.name.value == "name"
                ));
                match &*n.node.rhs {
                    ast::Expr::List(list) => {
                        assert_eq!(list.node.values.len(), 2);
                        assert!(matches!(
                            &*list.node.values[0],
                            ast::Expr::Lit(lit) if matches!(&lit.node, Lit::CharStringLit(s) if s == "Alice")
                        ));
                        assert!(matches!(
                            &*list.node.values[1],
                            ast::Expr::Lit(lit) if matches!(&lit.node, Lit::CharStringLit(s) if s == "Bob")
                        ));
                    }
                    _ => panic!("expected List rhs"),
                }
            }
            _ => panic!("expected In"),
        }
    }

    #[test]
    fn test_not_in() {
        let expr = parse("x NOT IN (1, 2)");
        match &expr {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, ast::UniOpKind::Not);
                match &*n.node.expr {
                    ast::Expr::In(inner) => {
                        assert!(matches!(
                            &*inner.node.lhs,
                            ast::Expr::VarRef(v) if v.node.name.value == "x"
                        ));
                        match &*inner.node.rhs {
                            ast::Expr::List(list) => assert_eq!(list.node.values.len(), 2),
                            _ => panic!("expected List rhs"),
                        }
                    }
                    _ => panic!("expected In inside UniOp"),
                }
            }
            _ => panic!("expected UniOp"),
        }
    }
}

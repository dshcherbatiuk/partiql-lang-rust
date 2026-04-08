//! WhereClause — WHERE expression parsing.
//!
//! ```text
//! where_clause ::= WHERE expr
//! ```

use partiql_ast::ast::{AstNode, WhereClause};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::parse_context::ParseContext;

use super::ClauseParser;

pub struct WhereClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> WhereClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> ClauseParser for WhereClauseParser<'p> {
    type Output = Box<AstNode<WhereClause>>;

    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Box<AstNode<WhereClause>>> {
        let expr = self.chain.parse_expr(input, pctx)?;
        Ok(Box::new(pctx.node(WhereClause {
            expr: Box::new(expr),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SelectParser;
    use partiql_ast::ast::{BinOpKind, Expr};

    // Helper: create a parser and context
    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_simple_comparison() {
        let (parser, pctx) = setup();
        let mut input = "x = 1";
        let result = WhereClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &*result.node.expr {
            Expr::BinOp(bin_op) => {
                assert_eq!(bin_op.node.kind, BinOpKind::Eq);
            }
            other => panic!("expected BinOp, got {:?}", other),
        }
    }

    #[test]
    fn test_and_condition() {
        let (parser, pctx) = setup();
        let mut input = "x = 1 AND y = 2";
        let result = WhereClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &*result.node.expr {
            Expr::BinOp(bin_op) => {
                assert_eq!(bin_op.node.kind, BinOpKind::And);
            }
            other => panic!("expected BinOp(And), got {:?}", other),
        }
    }

    #[test]
    fn test_string_comparison() {
        let (parser, pctx) = setup();
        let mut input = "name = 'Alice'";
        let result = WhereClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &*result.node.expr {
            Expr::BinOp(bin_op) => {
                assert_eq!(bin_op.node.kind, BinOpKind::Eq);
            }
            other => panic!("expected BinOp(Eq), got {:?}", other),
        }
    }
}

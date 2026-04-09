//! HAVING clause parsing.
//!
//! ```text
//! having_clause ::= HAVING expr
//! ```

use partiql_ast::ast::{AstNode, HavingClause};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::parse_context::ParseContext;

use super::ClauseParser;

pub struct HavingClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> HavingClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> ClauseParser for HavingClauseParser<'p> {
    type Output = Box<AstNode<HavingClause>>;

    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Box<AstNode<HavingClause>>> {
        let expr = self.chain.parse_expr(input, pctx)?;
        Ok(Box::new(pctx.node(HavingClause {
            expr: Box::new(expr),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dql::SelectParser;
    use partiql_ast::ast::{BinOpKind, Expr};

    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_simple_having() {
        let (parser, pctx) = setup();
        let mut input = "COUNT(*) > 5";
        let result = HavingClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &*result.node.expr {
            Expr::BinOp(n) => assert_eq!(n.node.kind, BinOpKind::Gt),
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_having_with_and() {
        let (parser, pctx) = setup();
        let mut input = "COUNT(*) > 5 AND SUM(amount) < 1000";
        let result = HavingClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &*result.node.expr {
            Expr::BinOp(n) => assert_eq!(n.node.kind, BinOpKind::And),
            _ => panic!("expected BinOp"),
        }
    }
}

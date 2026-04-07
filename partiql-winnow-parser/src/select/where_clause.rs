//! WhereClause — WHERE expression parsing.
//!
//! ```text
//! where_clause ::= WHERE expr
//! ```

use partiql_ast::ast::{AstNode, WhereClause};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::parse_context::ParseContext;

pub struct WhereClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> WhereClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }

    pub fn parse(
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

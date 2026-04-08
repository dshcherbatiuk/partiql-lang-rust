//! DELETE FROM strategy.
//!
//! ```text
//! delete ::= DELETE FROM expr [WHERE expr]
//! ```

use partiql_ast::ast::{self, Delete, Dml, DmlOp, FromClause};
use winnow::prelude::*;

use super::DmlStrategy;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::select::from_clause::parse_source;
use crate::whitespace::{ws, ws0};

pub struct DeleteStrategy<'p> {
    chain: &'p ExprChain,
}

impl<'p> DeleteStrategy<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> DmlStrategy for DeleteStrategy<'p> {
    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ast::Dml> {
        let _ = ws0(input);
        (kw("DELETE"), ws, kw("FROM"), ws).parse_next(input)?;

        let source = parse_source(input, self.chain, pctx)?;
        let _ = ws0(input);

        let where_clause = {
            let checkpoint = *input;
            if (kw("WHERE"), ws).parse_next(input).is_ok() {
                Some(Box::new(self.chain.parse_expr(input, pctx)?))
            } else {
                *input = checkpoint;
                None
            }
        };

        Ok(Dml {
            op: DmlOp::Delete(Delete {}),
            from_clause: Some(FromClause { source }),
            where_clause,
            returning: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::dml::DmlParser;
    use crate::expr::ExprChain;
    use crate::parse_context::ParseContext;
    use partiql_ast::ast::{self, DmlOp};

    fn parse_dml(input: &str) -> ast::Dml {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        let parser = DmlParser::new(&chain);
        parser
            .try_parse(&mut i, &pctx)
            .expect("not DML")
            .expect("parse failed")
    }

    #[test]
    fn test_delete_all() {
        let dml = parse_dml("DELETE FROM users");
        assert!(matches!(&dml.op, DmlOp::Delete(_)));
        assert!(dml.from_clause.is_some());
        assert!(dml.where_clause.is_none());
    }

    #[test]
    fn test_delete_with_where() {
        let dml = parse_dml("DELETE FROM users WHERE email = 'a@co'");
        assert!(matches!(&dml.op, DmlOp::Delete(_)));
        assert!(dml.from_clause.is_some());
        assert!(dml.where_clause.is_some());
    }

    #[test]
    fn test_delete_quoted_table() {
        let dml = parse_dml("DELETE FROM \"fde.users\" WHERE email = 'test@co'");
        assert!(matches!(&dml.op, DmlOp::Delete(_)));
        assert!(dml.where_clause.is_some());
    }
}

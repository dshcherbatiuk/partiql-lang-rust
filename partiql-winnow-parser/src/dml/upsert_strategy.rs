//! UPSERT INTO strategy.
//!
//! ```text
//! upsert ::= UPSERT INTO expr expr
//! ```

use partiql_ast::ast::{self, Dml, DmlOp, Insert};
use winnow::prelude::*;

use super::DmlStrategy;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

pub struct UpsertStrategy<'p> {
    chain: &'p ExprChain,
}

impl<'p> UpsertStrategy<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> DmlStrategy for UpsertStrategy<'p> {
    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ast::Dml> {
        let _ = ws0(input);
        (kw("UPSERT"), ws, kw("INTO"), ws).parse_next(input)?;

        let target = self.chain.parse_expr(input, pctx)?;
        let _ = ws0(input);
        let values = self.chain.parse_expr(input, pctx)?;

        Ok(Dml {
            op: DmlOp::Insert(Insert {
                target: Box::new(target),
                values: Box::new(values),
            }),
            from_clause: None,
            where_clause: None,
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
    fn test_upsert() {
        let dml = parse_dml("UPSERT INTO users <<{'email': 'a@co'}>>");
        match &dml.op {
            DmlOp::Insert(ins) => {
                assert!(matches!(&*ins.values, ast::Expr::Bag(_)));
            }
            other => panic!("expected Insert (via UPSERT), got {:?}", other),
        }
    }
}

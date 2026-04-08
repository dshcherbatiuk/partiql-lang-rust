//! REPLACE INTO strategy.
//!
//! ```text
//! replace ::= REPLACE INTO expr expr
//! ```
//!
//! FDE uses REPLACE for upsert-like semantics on existing rows.

use partiql_ast::ast::{self, Dml, DmlOp, Insert};
use winnow::prelude::*;

use super::DmlStrategy;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

pub struct ReplaceStrategy<'p> {
    chain: &'p ExprChain,
}

impl<'p> ReplaceStrategy<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> DmlStrategy for ReplaceStrategy<'p> {
    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ast::Dml> {
        let _ = ws0(input);
        (kw("REPLACE"), ws, kw("INTO"), ws).parse_next(input)?;

        let target = self.chain.parse_expr(input, pctx)?;
        let _ = ws0(input);
        let values = self.chain.parse_expr(input, pctx)?;

        // REPLACE reuses Insert AST — the DML handler distinguishes by keyword
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
    fn test_replace() {
        let dml = parse_dml("REPLACE INTO users <<{'email': 'a@co', 'name': 'Updated'}>>");
        match &dml.op {
            DmlOp::Insert(ins) => {
                assert!(matches!(&*ins.target, ast::Expr::VarRef(_)));
                assert!(matches!(&*ins.values, ast::Expr::Bag(_)));
            }
            other => panic!("expected Insert (via REPLACE), got {:?}", other),
        }
    }
}

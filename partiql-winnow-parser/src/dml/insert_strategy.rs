//! INSERT INTO strategy.
//!
//! ```text
//! insert ::= INSERT INTO expr expr
//! ```

use partiql_ast::ast::{self, Dml, DmlOp, Insert};
use winnow::prelude::*;

use super::DmlStrategy;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

pub struct InsertStrategy<'p> {
    chain: &'p ExprChain,
}

impl<'p> InsertStrategy<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> DmlStrategy for InsertStrategy<'p> {
    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ast::Dml> {
        let _ = ws0(input);
        (kw("INSERT"), ws, kw("INTO"), ws).parse_next(input)?;

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
    fn test_insert_bag() {
        let dml = parse_dml("INSERT INTO users <<{'email': 'a@co'}>>");
        match &dml.op {
            DmlOp::Insert(ins) => {
                assert!(matches!(&*ins.target, ast::Expr::VarRef(_)));
                assert!(matches!(&*ins.values, ast::Expr::Bag(_)));
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_insert_quoted_table() {
        let dml = parse_dml(
            "INSERT INTO \"fde.users\" <<{'email': 'user@co', 'name': 'Alice'}>>",
        );
        match &dml.op {
            DmlOp::Insert(ins) => {
                assert!(matches!(&*ins.values, ast::Expr::Bag(_)));
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_insert_multiple_rows() {
        let dml = parse_dml("INSERT INTO users <<{'email': 'a@co'}, {'email': 'b@co'}>>");
        match &dml.op {
            DmlOp::Insert(ins) => match &*ins.values {
                ast::Expr::Bag(bag) => assert_eq!(bag.node.values.len(), 2),
                other => panic!("expected Bag, got {:?}", other),
            },
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_insert_fde_pattern() {
        let dml = parse_dml(
            "INSERT INTO \"fde.users\" <<{'email': 'user@co', 'platformData': [{'id': 'abc', 'platform': 'MsTeams'}]}>>",
        );
        match &dml.op {
            DmlOp::Insert(ins) => match &*ins.values {
                ast::Expr::Bag(bag) => {
                    assert_eq!(bag.node.values.len(), 1);
                    assert!(matches!(&*bag.node.values[0], ast::Expr::Struct(_)));
                }
                other => panic!("expected Bag, got {:?}", other),
            },
            other => panic!("expected Insert, got {:?}", other),
        }
    }
}

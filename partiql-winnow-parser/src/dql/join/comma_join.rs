//! Comma join — `FROM t1, t2` → CROSS JOIN.
//!
//! ```text
//! comma_join ::= ',' from_source
//! ```

use partiql_ast::ast::{FromSource, Join, JoinKind};
use winnow::prelude::*;

use super::JoinParser;
use crate::expr::ExprChain;
use crate::keyword::ch;
use crate::parse_context::ParseContext;
use crate::dql::from_clause::parse_source;
use crate::whitespace::ws0;

pub struct CommaJoinParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> CommaJoinParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> JoinParser for CommaJoinParser<'p> {
    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
        left: &FromSource,
    ) -> PResult<FromSource> {
        ch(',').parse_next(input)?;
        let _ = ws0(input);
        let right = parse_source(input, self.chain, pctx)?;

        Ok(FromSource::Join(pctx.node(Join {
            kind: JoinKind::Cross,
            left: Box::new(left.clone()),
            right: Box::new(right),
            predicate: None,
        })))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_context::ParseContext;
    use crate::dql::from_clause::FromClauseParser;
    use crate::dql::ClauseParser;
    use crate::dql::SelectParser;
    use partiql_ast::ast::{FromSource, JoinKind};

    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_comma_join_two_tables() {
        let (parser, pctx) = setup();
        let mut input = "users, orders WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::Join(join) => {
                assert_eq!(join.node.kind, JoinKind::Cross);
                assert!(join.node.predicate.is_none());
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }

    #[test]
    fn test_comma_join_three_tables() {
        let (parser, pctx) = setup();
        let mut input = "a, b, c WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        // Left-associative: ((a , b) , c)
        match &result.node.source {
            FromSource::Join(outer) => {
                assert_eq!(outer.node.kind, JoinKind::Cross);
                assert!(matches!(&*outer.node.left, FromSource::Join(_)));
            }
            other => panic!("expected nested Join, got {:?}", other),
        }
    }

    #[test]
    fn test_unnest_pattern() {
        // FROM "fde.users" u, u.platformData p
        let (parser, pctx) = setup();
        let mut input = "\"fde.users\" u, u.platformData p WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::Join(join) => {
                assert_eq!(join.node.kind, JoinKind::Cross);
                // Left is "fde.users" u
                match &*join.node.left {
                    FromSource::FromLet(fl) => {
                        assert_eq!(fl.node.as_alias.as_ref().unwrap().value, "u");
                    }
                    other => panic!("expected FromLet left, got {:?}", other),
                }
                // Right is u.platformData p (path expression with implicit alias)
                match &*join.node.right {
                    FromSource::FromLet(fl) => {
                        assert_eq!(fl.node.as_alias.as_ref().unwrap().value, "p");
                    }
                    other => panic!("expected FromLet right, got {:?}", other),
                }
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }
}

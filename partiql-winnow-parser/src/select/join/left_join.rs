//! LEFT [OUTER] JOIN parser.
//!
//! ```text
//! left_join ::= LEFT [OUTER] JOIN from_source ON expr
//! ```

use partiql_ast::ast::{FromSource, Join, JoinKind, JoinSpec};
use winnow::prelude::*;

use super::JoinParser;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::select::from_clause::parse_source;
use crate::whitespace::{ws, ws0};

pub struct LeftJoinParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> LeftJoinParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> JoinParser for LeftJoinParser<'p> {
    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
        left: FromSource,
    ) -> PResult<FromSource> {
        (kw("LEFT"), ws).parse_next(input)?;
        let _ = (kw("OUTER"), ws).parse_next(input); // optional OUTER
        (kw("JOIN"), ws).parse_next(input)?;

        let right = parse_source(input, self.chain, pctx)?;
        let _ = ws0(input);

        let predicate = if (kw("ON"), ws).parse_next(input).is_ok() {
            let on_expr = self.chain.parse_expr(input, pctx)?;
            Some(pctx.node(JoinSpec::On(Box::new(on_expr))))
        } else {
            None
        };

        Ok(FromSource::Join(pctx.node(Join {
            kind: JoinKind::Left,
            left: Box::new(left),
            right: Box::new(right),
            predicate,
        })))
    }

    fn name(&self) -> &str {
        "left"
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_context::ParseContext;
    use crate::select::from_clause::FromClauseParser;
    use crate::select::ClauseParser;
    use crate::select::SelectParser;
    use partiql_ast::ast::{FromSource, JoinKind};

    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_left_join() {
        let (parser, pctx) = setup();
        let mut input = "users LEFT JOIN orders ON users.id = orders.user_id WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::Join(join) => {
                assert_eq!(join.node.kind, JoinKind::Left);
                assert!(join.node.predicate.is_some());
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }

    #[test]
    fn test_left_outer_join() {
        let (parser, pctx) = setup();
        let mut input = "users LEFT OUTER JOIN orders ON users.id = orders.user_id WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::Join(join) => {
                assert_eq!(join.node.kind, JoinKind::Left);
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }
}

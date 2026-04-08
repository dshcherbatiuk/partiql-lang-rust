//! CROSS JOIN parser.
//!
//! ```text
//! cross_join ::= CROSS JOIN from_source
//! ```

use partiql_ast::ast::{FromSource, Join, JoinKind};
use winnow::prelude::*;

use super::JoinParser;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::select::from_clause::parse_source;
use crate::whitespace::ws;

pub struct CrossJoinParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> CrossJoinParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> JoinParser for CrossJoinParser<'p> {
    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
        left: &FromSource,
    ) -> PResult<FromSource> {
        (kw("CROSS"), ws, kw("JOIN"), ws).parse_next(input)?;
        let right = parse_source(input, self.chain, pctx)?;

        Ok(FromSource::Join(pctx.node(Join {
            kind: JoinKind::Cross,
            left: Box::new(left.clone()),
            right: Box::new(right),
            predicate: None,
        })))
    }

    fn name(&self) -> &str {
        "cross"
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
    fn test_cross_join() {
        let (parser, pctx) = setup();
        let mut input = "users CROSS JOIN orders WHERE";
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
}

//! [INNER] JOIN parser.
//!
//! ```text
//! inner_join ::= [INNER] JOIN from_source ON expr
//! ```

use partiql_ast::ast::{FromSource, Join, JoinKind, JoinSpec};
use winnow::prelude::*;

use super::JoinParser;
use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::dql::from_clause::parse_source;
use crate::whitespace::{ws, ws0};

pub struct InnerJoinParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> InnerJoinParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> JoinParser for InnerJoinParser<'p> {
    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
        left: &FromSource,
    ) -> PResult<FromSource> {
        // Optional INNER keyword
        let _ = (kw("INNER"), ws).parse_next(input);
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
            kind: JoinKind::Inner,
            left: Box::new(left.clone()),
            right: Box::new(right),
            predicate,
        })))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_context::ParseContext;
    use crate::dql::from_clause::FromClauseParser;
    use crate::dql::ClauseParser;
    use crate::dql::SelectParser;
    use partiql_ast::ast::{FromSource, JoinKind, JoinSpec};

    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_inner_join_explicit() {
        let (parser, pctx) = setup();
        let mut input = "users INNER JOIN orders ON users.id = orders.user_id WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::Join(join) => {
                assert_eq!(join.node.kind, JoinKind::Inner);
                assert!(matches!(
                    &join.node.predicate.as_ref().unwrap().node,
                    JoinSpec::On(_)
                ));
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }

    #[test]
    fn test_join_implicit_inner() {
        let (parser, pctx) = setup();
        let mut input = "users JOIN orders ON users.id = orders.user_id WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::Join(join) => {
                assert_eq!(join.node.kind, JoinKind::Inner);
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }
}

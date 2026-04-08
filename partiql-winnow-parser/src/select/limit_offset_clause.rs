//! LIMIT / OFFSET clause parsing.
//!
//! ```text
//! limit_offset ::= LIMIT expr [OFFSET expr]
//!                | OFFSET expr [LIMIT expr]
//! ```

use partiql_ast::ast::{AstNode, LimitOffsetClause};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::ClauseParser;

pub struct LimitOffsetClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> LimitOffsetClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> ClauseParser for LimitOffsetClauseParser<'p> {
    type Output = Box<AstNode<LimitOffsetClause>>;

    fn name(&self) -> &str {
        "limit_offset"
    }

    /// Parses LIMIT/OFFSET in either order.
    /// Called after detecting LIMIT or OFFSET keyword in `select_parser`.
    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Box<AstNode<LimitOffsetClause>>> {
        let mut limit = None;
        let mut offset = None;

        // LIMIT expr
        if kw("LIMIT").parse_next(input).is_ok() {
            let _ = ws(input);
            limit = Some(Box::new(self.chain.parse_expr(input, pctx)?));
            let _ = ws0(input);

            // Optional OFFSET after LIMIT
            if (kw("OFFSET"), ws).parse_next(input).is_ok() {
                offset = Some(Box::new(self.chain.parse_expr(input, pctx)?));
            }
        }
        // OFFSET expr
        else if kw("OFFSET").parse_next(input).is_ok() {
            let _ = ws(input);
            offset = Some(Box::new(self.chain.parse_expr(input, pctx)?));
            let _ = ws0(input);

            // Optional LIMIT after OFFSET
            if (kw("LIMIT"), ws).parse_next(input).is_ok() {
                limit = Some(Box::new(self.chain.parse_expr(input, pctx)?));
            }
        } else {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }

        Ok(Box::new(pctx.node(LimitOffsetClause { limit, offset })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SelectParser;

    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_limit_only() {
        let (parser, pctx) = setup();
        let mut input = "LIMIT 10";
        let result = LimitOffsetClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(result.node.limit.is_some());
        assert!(result.node.offset.is_none());
    }

    #[test]
    fn test_offset_only() {
        let (parser, pctx) = setup();
        let mut input = "OFFSET 5";
        let result = LimitOffsetClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(result.node.limit.is_none());
        assert!(result.node.offset.is_some());
    }

    #[test]
    fn test_limit_offset() {
        let (parser, pctx) = setup();
        let mut input = "LIMIT 10 OFFSET 5";
        let result = LimitOffsetClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(result.node.limit.is_some());
        assert!(result.node.offset.is_some());
    }

    #[test]
    fn test_offset_limit() {
        let (parser, pctx) = setup();
        let mut input = "OFFSET 5 LIMIT 10";
        let result = LimitOffsetClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(result.node.limit.is_some());
        assert!(result.node.offset.is_some());
    }
}

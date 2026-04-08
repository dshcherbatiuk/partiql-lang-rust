//! ORDER BY clause parsing.
//!
//! ```text
//! order_by_clause ::= ORDER BY sort_spec (',' sort_spec)*
//! sort_spec       ::= expr [ASC | DESC] [NULLS FIRST | NULLS LAST]
//! ```

use partiql_ast::ast::{
    AstNode, NullOrderingSpec, OrderByExpr, OrderingSpec, SortSpec,
};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::ClauseParser;

pub struct OrderByClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> OrderByClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> ClauseParser for OrderByClauseParser<'p> {
    type Output = Box<AstNode<OrderByExpr>>;

    fn name(&self) -> &str {
        "order_by"
    }

    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Box<AstNode<OrderByExpr>>> {
        let mut sort_specs = Vec::new();

        loop {
            let _ = ws0(input);
            let expr = self.chain.parse_expr(input, pctx)?;
            let _ = ws0(input);

            let ordering_spec = if kw("ASC").parse_next(input).is_ok() {
                Some(OrderingSpec::Asc)
            } else if kw("DESC").parse_next(input).is_ok() {
                Some(OrderingSpec::Desc)
            } else {
                None
            };

            let _ = ws0(input);
            let null_ordering_spec =
                if (kw("NULLS"), ws).parse_next(input).is_ok() {
                    if kw("FIRST").parse_next(input).is_ok() {
                        Some(NullOrderingSpec::First)
                    } else if kw("LAST").parse_next(input).is_ok() {
                        Some(NullOrderingSpec::Last)
                    } else {
                        None
                    }
                } else {
                    None
                };

            sort_specs.push(pctx.node(SortSpec {
                expr: Box::new(expr),
                ordering_spec,
                null_ordering_spec,
            }));

            let _ = ws0(input);
            if crate::keyword::ch(',').parse_next(input).is_err() {
                break;
            }
        }

        Ok(Box::new(pctx.node(OrderByExpr { sort_specs })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SelectParser;
    use partiql_ast::ast::{NullOrderingSpec, OrderingSpec};

    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_single_column() {
        let (parser, pctx) = setup();
        let mut input = "name";
        let result = OrderByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(result.node.sort_specs.len(), 1);
        assert!(result.node.sort_specs[0].node.ordering_spec.is_none());
    }

    #[test]
    fn test_asc() {
        let (parser, pctx) = setup();
        let mut input = "name ASC";
        let result = OrderByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(
            result.node.sort_specs[0].node.ordering_spec,
            Some(OrderingSpec::Asc)
        );
    }

    #[test]
    fn test_desc() {
        let (parser, pctx) = setup();
        let mut input = "age DESC";
        let result = OrderByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(
            result.node.sort_specs[0].node.ordering_spec,
            Some(OrderingSpec::Desc)
        );
    }

    #[test]
    fn test_multiple_specs() {
        let (parser, pctx) = setup();
        let mut input = "name ASC, age DESC";
        let result = OrderByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(result.node.sort_specs.len(), 2);
        assert_eq!(
            result.node.sort_specs[0].node.ordering_spec,
            Some(OrderingSpec::Asc)
        );
        assert_eq!(
            result.node.sort_specs[1].node.ordering_spec,
            Some(OrderingSpec::Desc)
        );
    }

    #[test]
    fn test_nulls_first() {
        let (parser, pctx) = setup();
        let mut input = "name ASC NULLS FIRST";
        let result = OrderByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(
            result.node.sort_specs[0].node.null_ordering_spec,
            Some(NullOrderingSpec::First)
        );
    }

    #[test]
    fn test_nulls_last() {
        let (parser, pctx) = setup();
        let mut input = "name DESC NULLS LAST";
        let result = OrderByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(
            result.node.sort_specs[0].node.null_ordering_spec,
            Some(NullOrderingSpec::Last)
        );
    }
}

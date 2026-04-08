//! GROUP BY clause parsing.
//!
//! ```text
//! group_by_clause ::= GROUP [PARTIAL] BY group_key (',' group_key)* [GROUP AS alias]
//! group_key       ::= expr [AS alias]
//! ```

use partiql_ast::ast::{
    AstNode, CaseSensitivity, GroupByExpr, GroupKey, GroupingStrategy, SymbolPrimitive,
};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::identifier;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::ClauseParser;

pub struct GroupByClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> GroupByClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> ClauseParser for GroupByClauseParser<'p> {
    type Output = Box<AstNode<GroupByExpr>>;

    fn name(&self) -> &str {
        "group_by"
    }

    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Box<AstNode<GroupByExpr>>> {
        let strategy = if (kw("PARTIAL"), ws).parse_next(input).is_ok() {
            Some(GroupingStrategy::GroupPartial)
        } else {
            None
        };

        let mut keys = Vec::new();
        loop {
            let _ = ws0(input);
            let expr = self.chain.parse_expr(input, pctx)?;
            let _ = ws0(input);

            let as_alias = if (kw("AS"), ws).parse_next(input).is_ok() {
                let alias = identifier::identifier(input)?;
                Some(SymbolPrimitive {
                    value: alias.to_string(),
                    case: CaseSensitivity::CaseInsensitive,
                })
            } else {
                None
            };

            keys.push(pctx.node(GroupKey {
                expr: Box::new(expr),
                as_alias,
            }));

            let _ = ws0(input);
            if crate::keyword::ch(',').parse_next(input).is_err() {
                break;
            }
        }

        let _ = ws0(input);
        let group_as_alias = if (kw("GROUP"), ws, kw("AS"), ws).parse_next(input).is_ok() {
            let alias = identifier::identifier(input)?;
            Some(SymbolPrimitive {
                value: alias.to_string(),
                case: CaseSensitivity::CaseInsensitive,
            })
        } else {
            None
        };

        Ok(Box::new(pctx.node(GroupByExpr {
            strategy,
            keys,
            group_as_alias,
        })))
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
    fn test_single_key() {
        let (parser, pctx) = setup();
        let mut input = "category";
        let result = GroupByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(result.node.keys.len(), 1);
        assert!(result.node.strategy.is_none());
    }

    #[test]
    fn test_multiple_keys() {
        let (parser, pctx) = setup();
        let mut input = "category, region";
        let result = GroupByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(result.node.keys.len(), 2);
    }

    #[test]
    fn test_key_with_alias() {
        let (parser, pctx) = setup();
        let mut input = "category AS cat";
        let result = GroupByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(result.node.keys.len(), 1);
        assert_eq!(
            result.node.keys[0].node.as_alias.as_ref().unwrap().value,
            "cat"
        );
    }

    #[test]
    fn test_partial_strategy() {
        let (parser, pctx) = setup();
        let mut input = "PARTIAL category";
        let result = GroupByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(matches!(
            result.node.strategy,
            Some(GroupingStrategy::GroupPartial)
        ));
    }

    #[test]
    fn test_group_as_alias() {
        let (parser, pctx) = setup();
        let mut input = "category GROUP AS g";
        let result = GroupByClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert_eq!(
            result.node.group_as_alias.as_ref().unwrap().value,
            "g"
        );
    }
}

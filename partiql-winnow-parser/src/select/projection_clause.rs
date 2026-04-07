//! ProjectionClause — SELECT projection parsing.
//!
//! ```text
//! projection ::= '*'
//!              | VALUE expr
//!              | [ALL | DISTINCT] expr [AS alias] (',' expr [AS alias])*
//! ```

use partiql_ast::ast::{
    CaseSensitivity, ProjectExpr, ProjectItem, Projection, ProjectionKind, SetQuantifier,
    SymbolPrimitive,
};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::identifier;
use crate::keyword::{ch, kw};
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::ClauseParser;

pub struct ProjectionClause<'p> {
    chain: &'p ExprChain,
}

impl<'p> ProjectionClause<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> ClauseParser for ProjectionClause<'p> {
    type Output = Projection;

    fn name(&self) -> &str {
        "projection"
    }

    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<Projection> {
        if (kw("VALUE"), ws).parse_next(input).is_ok() {
            let expr = self.chain.parse_expr(input, pctx)?;
            return Ok(Projection {
                kind: ProjectionKind::ProjectValue(Box::new(expr)),
                setq: None,
            });
        }

        let setq = if (kw("ALL"), ws).parse_next(input).is_ok() {
            Some(SetQuantifier::All)
        } else if (kw("DISTINCT"), ws).parse_next(input).is_ok() {
            Some(SetQuantifier::Distinct)
        } else {
            None
        };

        if ch('*').parse_next(input).is_ok() {
            return Ok(Projection {
                kind: ProjectionKind::ProjectStar,
                setq,
            });
        }

        let mut items = Vec::new();
        loop {
            let _ = ws0(input);
            let expr = self.chain.parse_expr(input, pctx)?;
            let _ = ws0(input);

            let as_alias = if (kw("AS"), ws).parse_next(input).is_ok() {
                let alias = identifier::identifier(input)?;
                Some(SymbolPrimitive {
                    value: alias,
                    case: CaseSensitivity::CaseInsensitive,
                })
            } else {
                None
            };

            items.push(pctx.node(ProjectItem::ProjectExpr(ProjectExpr {
                expr: Box::new(expr),
                as_alias,
            })));

            let _ = ws0(input);
            if ch(',').parse_next(input).is_err() {
                break;
            }
        }

        Ok(Projection {
            kind: ProjectionKind::ProjectList(items),
            setq,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SelectParser;
    use partiql_ast::ast::{ProjectItem, SetQuantifier};

    // Helper: create a parser and context
    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_star() {
        let (parser, pctx) = setup();
        let mut input = "* FROM";
        let result = ProjectionClause::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(matches!(result.kind, ProjectionKind::ProjectStar));
        assert!(result.setq.is_none());
    }

    #[test]
    fn test_value_expr() {
        let (parser, pctx) = setup();
        let mut input = "VALUE x FROM";
        let result = ProjectionClause::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(matches!(result.kind, ProjectionKind::ProjectValue(_)));
    }

    #[test]
    fn test_single_field() {
        let (parser, pctx) = setup();
        let mut input = "a FROM";
        let result = ProjectionClause::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.kind {
            ProjectionKind::ProjectList(items) => assert_eq!(items.len(), 1),
            other => panic!("expected ProjectList, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_fields() {
        let (parser, pctx) = setup();
        let mut input = "a, b, c FROM";
        let result = ProjectionClause::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.kind {
            ProjectionKind::ProjectList(items) => assert_eq!(items.len(), 3),
            other => panic!("expected ProjectList, got {:?}", other),
        }
    }

    #[test]
    fn test_field_with_alias() {
        let (parser, pctx) = setup();
        let mut input = "a AS col1 FROM";
        let result = ProjectionClause::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.kind {
            ProjectionKind::ProjectList(items) => {
                assert_eq!(items.len(), 1);
                match &items[0].node {
                    ProjectItem::ProjectExpr(pe) => {
                        assert!(pe.as_alias.is_some());
                        assert_eq!(pe.as_alias.as_ref().unwrap().value, "col1");
                    }
                    other => panic!("expected ProjectExpr, got {:?}", other),
                }
            }
            other => panic!("expected ProjectList, got {:?}", other),
        }
    }

    #[test]
    fn test_distinct() {
        let (parser, pctx) = setup();
        let mut input = "DISTINCT a FROM";
        let result = ProjectionClause::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        assert!(matches!(result.setq, Some(SetQuantifier::Distinct)));
        assert!(matches!(result.kind, ProjectionKind::ProjectList(_)));
    }
}

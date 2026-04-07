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

pub struct ProjectionClause<'p> {
    chain: &'p ExprChain,
}

impl<'p> ProjectionClause<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }

    pub fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<Projection> {
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

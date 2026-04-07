//! FromClause — FROM source parsing.
//!
//! ```text
//! from_clause ::= from_source (',' from_source)*
//! from_source ::= expr [AS alias] [AT alias]
//! ```
//! TODO: JOIN, UNNEST

use partiql_ast::ast::{
    AstNode, CaseSensitivity, FromClause, FromLet, FromLetKind, FromSource, SymbolPrimitive,
};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::identifier;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

pub struct FromClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> FromClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }

    pub fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<AstNode<FromClause>> {
        let source = self.parse_source(input, pctx)?;
        // TODO: comma-separated sources, JOINs
        Ok(pctx.node(FromClause { source }))
    }

    fn parse_source(&self, input: &mut &str, pctx: &ParseContext) -> PResult<FromSource> {
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

        let _ = ws0(input);
        let at_alias = if (kw("AT"), ws).parse_next(input).is_ok() {
            let alias = identifier::identifier(input)?;
            Some(SymbolPrimitive {
                value: alias,
                case: CaseSensitivity::CaseInsensitive,
            })
        } else {
            None
        };

        Ok(FromSource::FromLet(pctx.node(FromLet {
            expr: Box::new(expr),
            kind: FromLetKind::Scan,
            as_alias,
            at_alias,
            by_alias: None,
        })))
    }
}

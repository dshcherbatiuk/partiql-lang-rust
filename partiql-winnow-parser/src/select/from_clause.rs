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

use super::ClauseParser;

pub struct FromClauseParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> FromClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
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

impl<'p> ClauseParser for FromClauseParser<'p> {
    type Output = AstNode<FromClause>;

    fn name(&self) -> &str {
        "from"
    }

    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<AstNode<FromClause>> {
        let source = self.parse_source(input, pctx)?;
        // TODO: comma-separated sources, JOINs
        Ok(pctx.node(FromClause { source }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SelectParser;
    use partiql_ast::ast::Expr;

    // Helper: create a parser and context
    fn setup() -> (SelectParser, ParseContext) {
        (SelectParser::new(), ParseContext::new())
    }

    #[test]
    fn test_simple_table() {
        let (parser, pctx) = setup();
        let mut input = "users";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::FromLet(from_let) => {
                assert!(matches!(*from_let.node.expr, Expr::VarRef(_)));
            }
            other => panic!("expected FromLet, got {:?}", other),
        }
    }

    #[test]
    fn test_table_with_alias() {
        let (parser, pctx) = setup();
        let mut input = "users AS u";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::FromLet(from_let) => {
                assert!(from_let.node.as_alias.is_some());
                assert_eq!(from_let.node.as_alias.as_ref().unwrap().value, "u");
            }
            other => panic!("expected FromLet, got {:?}", other),
        }
    }

    #[test]
    fn test_table_with_at_alias() {
        let (parser, pctx) = setup();
        let mut input = "users AS u AT idx";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::FromLet(from_let) => {
                assert_eq!(from_let.node.as_alias.as_ref().unwrap().value, "u");
                assert!(from_let.node.at_alias.is_some());
                assert_eq!(from_let.node.at_alias.as_ref().unwrap().value, "idx");
            }
            other => panic!("expected FromLet, got {:?}", other),
        }
    }

    #[test]
    fn test_quoted_table() {
        // Double-quoted identifiers are parsed as Ion string literals by the
        // expression chain (StringLiteralStrategy takes priority over IdentifierStrategy).
        let (parser, pctx) = setup();
        let mut input = "\"fde.users\"";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::FromLet(from_let) => {
                assert!(matches!(*from_let.node.expr, Expr::Lit(_)));
            }
            other => panic!("expected FromLet, got {:?}", other),
        }
    }
}

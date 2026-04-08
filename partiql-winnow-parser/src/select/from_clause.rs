//! FromClause — FROM source parsing.
//!
//! ```text
//! from_clause  ::= from_source (',' from_source)*
//!               | from_source join_clause+
//! from_source  ::= expr [AS alias] [AT alias]
//! join_clause  ::= [INNER | LEFT | RIGHT | FULL | CROSS] JOIN from_source [ON expr]
//! ```
//!
//! Comma-separated sources are folded into left-associative CROSS JOINs,
//! matching the PartiQL spec and the existing LALRPOP parser behavior.
//! UNNEST patterns like `FROM users u, u.platformData p` naturally produce
//! a CROSS JOIN where the right side is a Path expression.

use partiql_ast::ast::{
    AstNode, CaseSensitivity, FromClause, FromLet, FromLetKind, FromSource, Join, JoinKind,
    JoinSpec, SymbolPrimitive,
};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::identifier;
use crate::keyword::{ch, kw};
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::join::comma_join::CommaJoinParser;
use super::join::cross_join::CrossJoinParser;
use super::join::full_join::FullJoinParser;
use super::join::inner_join::InnerJoinParser;
use super::join::left_join::LeftJoinParser;
use super::join::right_join::RightJoinParser;
use super::join::JoinParser;
use super::ClauseParser;

pub struct FromClauseParser<'p> {
    chain: &'p ExprChain,
    join_parsers: Vec<Box<dyn JoinParser + 'p>>,
}

impl<'p> FromClauseParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self {
            chain,
            join_parsers: vec![
                Box::new(CommaJoinParser::new(chain)),
                Box::new(CrossJoinParser::new(chain)),
                Box::new(InnerJoinParser::new(chain)),
                Box::new(LeftJoinParser::new(chain)),
                Box::new(RightJoinParser::new(chain)),
                Box::new(FullJoinParser::new(chain)),
            ],
        }
    }
}

/// Parse a single FROM source: `expr [AS alias] [AT alias]`.
/// Shared by `FromClauseParser` and all `JoinParser` implementations.
pub(crate) fn parse_source(
    input: &mut &str,
    chain: &ExprChain,
    pctx: &ParseContext,
) -> PResult<FromSource> {
    let expr = chain.parse_expr(input, pctx)?;
    let _ = ws0(input);

    let as_alias = if (kw("AS"), ws).parse_next(input).is_ok() {
        let alias = identifier::identifier(input)?;
        Some(SymbolPrimitive {
            value: alias,
            case: CaseSensitivity::CaseInsensitive,
        })
    } else {
        try_implicit_alias(input)
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

/// Reserved keywords that cannot be implicit aliases.
/// If the next token is one of these, it's a clause keyword, not an alias.
const CLAUSE_KEYWORDS: &[&str] = &[
    "WHERE", "GROUP", "HAVING", "ORDER", "LIMIT", "OFFSET", "JOIN", "INNER", "LEFT", "RIGHT",
    "FULL", "CROSS", "ON", "SET", "AT", "UNION", "INTERSECT", "EXCEPT",
];

/// Try to parse an implicit alias — an identifier that is not a reserved clause keyword.
fn try_implicit_alias(input: &mut &str) -> Option<SymbolPrimitive> {
    let checkpoint = *input;
    if let Ok(name) = identifier::identifier(input) {
        let upper = name.to_uppercase();
        if CLAUSE_KEYWORDS.iter().any(|kw| *kw == upper) {
            *input = checkpoint;
            None
        } else {
            Some(SymbolPrimitive {
                value: name,
                case: CaseSensitivity::CaseInsensitive,
            })
        }
    } else {
        None
    }
}

impl<'p> FromClauseParser<'p> {
    /// Chain of Responsibility — tries each join parser in order.
    /// Returns `Some(joined)` if a parser matched, `None` if none matched.
    fn try_join(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
        left: FromSource,
    ) -> Result<Option<FromSource>, winnow::error::ErrMode<winnow::error::ContextError>> {
        for parser in &self.join_parsers {
            let checkpoint = *input;
            match parser.parse(input, pctx, left.clone()) {
                Ok(joined) => return Ok(Some(joined)),
                Err(winnow::error::ErrMode::Backtrack(_)) => {
                    *input = checkpoint;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }
}

impl<'p> ClauseParser for FromClauseParser<'p> {
    type Output = AstNode<FromClause>;

    fn name(&self) -> &str {
        "from"
    }

    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<AstNode<FromClause>> {
        let mut source = parse_source(input, self.chain, pctx)?;

        loop {
            let _ = ws0(input);
            match self.try_join(input, pctx, source.clone())? {
                Some(joined) => source = joined,
                None => break,
            }
        }

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
    fn test_implicit_alias() {
        let (parser, pctx) = setup();
        let mut input = "users u WHERE";
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
        assert_eq!(input.trim(), "WHERE");
    }

    #[test]
    fn test_implicit_alias_not_keyword() {
        // WHERE should NOT be consumed as an implicit alias
        let (parser, pctx) = setup();
        let mut input = "users WHERE";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::FromLet(from_let) => {
                assert!(from_let.node.as_alias.is_none());
            }
            other => panic!("expected FromLet, got {:?}", other),
        }
    }

    #[test]
    fn test_quoted_table() {
        // Double-quoted identifiers are case-sensitive VarRef in PartiQL
        let (parser, pctx) = setup();
        let mut input = "\"fde.users\"";
        let result = FromClauseParser::new(parser.chain())
            .parse(&mut input, &pctx)
            .expect("parse failed");
        match &result.node.source {
            FromSource::FromLet(from_let) => {
                match &*from_let.node.expr {
                    Expr::VarRef(v) => {
                        assert_eq!(v.node.name.value, "fde.users");
                        assert_eq!(
                            v.node.name.case,
                            partiql_ast::ast::CaseSensitivity::CaseSensitive
                        );
                    }
                    other => panic!("expected VarRef, got {:?}", other),
                }
            }
            other => panic!("expected FromLet, got {:?}", other),
        }
    }
}

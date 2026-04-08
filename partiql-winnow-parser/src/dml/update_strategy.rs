//! UPDATE strategy.
//!
//! ```text
//! update ::= UPDATE expr SET path '=' expr (',' path '=' expr)* [WHERE expr]
//! ```

use partiql_ast::ast::{self, Assignment, Dml, DmlOp, Set};
use winnow::prelude::*;

use super::DmlStrategy;
use crate::expr::ExprChain;
use crate::identifier;
use crate::keyword::{ch, kw};
use crate::parse_context::ParseContext;
use crate::select::from_clause::parse_source;
use crate::whitespace::{ws, ws0};

/// Parse assignment target as a simple identifier or dotted path.
/// Does NOT use the full expression parser to avoid `=` being consumed as comparison.
fn parse_assignment_target(
    input: &mut &str,
    pctx: &ParseContext,
) -> PResult<ast::Expr> {
    let name = identifier::dotted_path(input)?;
    if name.contains('.') {
        // Build Path expression for dotted paths like u.name
        let parts: Vec<&str> = name.splitn(2, '.').collect();
        let root = ast::Expr::VarRef(pctx.node(ast::VarRef {
            name: ast::SymbolPrimitive {
                value: parts[0].to_string(),
                case: ast::CaseSensitivity::CaseInsensitive,
            },
            qualifier: ast::ScopeQualifier::Unqualified,
        }));
        let step = ast::PathStep::PathProject(ast::PathExpr {
            index: Box::new(ast::Expr::VarRef(pctx.node(ast::VarRef {
                name: ast::SymbolPrimitive {
                    value: parts[1].to_string(),
                    case: ast::CaseSensitivity::CaseInsensitive,
                },
                qualifier: ast::ScopeQualifier::Unqualified,
            }))),
        });
        Ok(ast::Expr::Path(pctx.node(ast::Path {
            root: Box::new(root),
            steps: vec![step],
        })))
    } else {
        Ok(ast::Expr::VarRef(pctx.node(ast::VarRef {
            name: ast::SymbolPrimitive {
                value: name,
                case: ast::CaseSensitivity::CaseInsensitive,
            },
            qualifier: ast::ScopeQualifier::Unqualified,
        })))
    }
}

pub struct UpdateStrategy<'p> {
    chain: &'p ExprChain,
}

impl<'p> UpdateStrategy<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }
}

impl<'p> DmlStrategy for UpdateStrategy<'p> {
    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ast::Dml> {
        let _ = ws0(input);
        (kw("UPDATE"), ws).parse_next(input)?;

        let source = parse_source(input, self.chain, pctx)?;
        let _ = ws0(input);

        (kw("SET"), ws).parse_next(input)?;

        // First assignment: target = value
        // Parse target as identifier/path (NOT full expr — avoids consuming `=` as comparison)
        let target = parse_assignment_target(input, pctx)?;
        let _ = ws0(input);
        ch('=').parse_next(input)?;
        let _ = ws0(input);
        let value = self.chain.parse_expr(input, pctx)?;

        // TODO: support multiple SET assignments via comma
        let _ = ws0(input);

        let where_clause = {
            let checkpoint = *input;
            if (kw("WHERE"), ws).parse_next(input).is_ok() {
                Some(Box::new(self.chain.parse_expr(input, pctx)?))
            } else {
                *input = checkpoint;
                None
            }
        };

        Ok(Dml {
            op: DmlOp::Set(Set {
                assignment: Assignment {
                    target: Box::new(target),
                    value: Box::new(value),
                },
            }),
            from_clause: Some(ast::FromClause { source }),
            where_clause,
            returning: None,
        })
    }

    fn name(&self) -> &str {
        "update"
    }
}

#[cfg(test)]
mod tests {
    use crate::dml::DmlParser;
    use crate::expr::ExprChain;
    use crate::parse_context::ParseContext;
    use partiql_ast::ast::{self, DmlOp};

    fn parse_dml(input: &str) -> ast::Dml {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        let parser = DmlParser::new(&chain);
        parser
            .try_parse(&mut i, &pctx)
            .expect("not DML")
            .expect("parse failed")
    }

    #[test]
    fn test_update_set() {
        let dml = parse_dml("UPDATE users SET name = 'Bob' WHERE email = 'a@co'");
        assert!(matches!(&dml.op, DmlOp::Set(_)));
        assert!(dml.from_clause.is_some());
        assert!(dml.where_clause.is_some());
    }

    #[test]
    fn test_update_without_where() {
        let dml = parse_dml("UPDATE users SET active = true");
        assert!(matches!(&dml.op, DmlOp::Set(_)));
        assert!(dml.where_clause.is_none());
    }
}

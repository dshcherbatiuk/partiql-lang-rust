//! ON CONFLICT clause parsing for INSERT statements.
//!
//! ```text
//! on_conflict  ::= ON CONFLICT [conflict_target] conflict_action
//! conflict_target ::= '(' column (',' column)* ')'
//!                   | ON CONSTRAINT constraint_name
//! conflict_action ::= DO NOTHING
//!                   | DO UPDATE EXCLUDED [WHERE expr]
//!                   | DO UPDATE SET set_clause (',' set_clause)* [WHERE expr]
//!                   | DO REPLACE EXCLUDED [WHERE expr]
//!                   | DO REPLACE VALUE expr [WHERE expr]
//! set_clause   ::= column '=' extended_expr
//! extended_expr ::= function_call | expr
//! function_call ::= identifier '(' expr (',' expr)* ')'
//! ```

use partiql_ast::ast::{
    self, ConflictAction, ConflictTarget, ExtendedExpr, Expr, MergeFunction, OnConflict, SetClause,
};
use winnow::prelude::*;

use crate::expr::pratt::PrattParser;
use crate::identifier;
use crate::keyword::{ch, kw};
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

// ── Parser ──────────────────────────────────────────────────────────────

/// ON CONFLICT parser — holds &PrattParser for expression delegation.
pub struct OnConflictParser<'p> {
    pratt: &'p PrattParser,
}

impl<'p> OnConflictParser<'p> {
    pub fn new(pratt: &'p PrattParser) -> Self {
        Self { pratt }
    }

    /// Parse ON CONFLICT clause after INSERT INTO ... << ... >>.
    pub fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<OnConflict> {
        let _ = ws0(input);
        (kw("ON"), ws, kw("CONFLICT"), ws0).parse_next(input)?;

        let target = self.parse_conflict_target(input)?;
        let _ = ws0(input);
        let action = self.parse_conflict_action(input, pctx)?;

        Ok(OnConflict {
            expr: Box::new(Expr::Lit(pctx.node(ast::Lit::Null))),
            target,
            conflict_action: action,
        })
    }

    fn parse_conflict_target(&self, input: &mut &str) -> PResult<Option<ConflictTarget>> {
        let checkpoint = *input;

        // (column1, column2, ...)
        if ch('(').parse_next(input).is_ok() {
            let mut columns = Vec::new();
            loop {
                let _ = ws0(input);
                let col = identifier::identifier(input)?;
                columns.push(col.to_string());
                let _ = ws0(input);
                if ch(',').parse_next(input).is_err() {
                    break;
                }
            }
            let _ = ws0(input);
            ch(')').parse_next(input)?;
            return Ok(Some(ConflictTarget::Columns(columns)));
        }
        *input = checkpoint;

        // ON CONSTRAINT constraint_name
        let checkpoint2 = *input;
        if (kw("ON"), ws, kw("CONSTRAINT"), ws).parse_next(input).is_ok() {
            let name = identifier::identifier(input)?;
            return Ok(Some(ConflictTarget::Constraint(name.to_string())));
        }
        *input = checkpoint2;

        Ok(None)
    }

    fn parse_conflict_action(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<ConflictAction> {
        (kw("DO"), ws).parse_next(input)?;

        // DO NOTHING
        if kw("NOTHING").parse_next(input).is_ok() {
            return Ok(ConflictAction::DoNothing);
        }

        // DO UPDATE ...
        let checkpoint = *input;
        if (kw("UPDATE"), ws).parse_next(input).is_ok() {
            // DO UPDATE EXCLUDED [WHERE ...]
            if kw("EXCLUDED").parse_next(input).is_ok() {
                let where_clause = self.parse_optional_where(input, pctx)?;
                return Ok(ConflictAction::DoUpdateExcluded { where_clause });
            }

            // DO UPDATE SET col = expr, ... [WHERE ...]
            if (kw("SET"), ws).parse_next(input).is_ok() {
                let set_clauses = self.parse_set_clauses(input, pctx)?;
                let where_clause = self.parse_optional_where(input, pctx)?;
                return Ok(ConflictAction::DoUpdateSet {
                    set_clauses,
                    where_clause,
                });
            }

            *input = checkpoint;
        }

        // DO REPLACE ...
        if (kw("REPLACE"), ws).parse_next(input).is_ok() {
            // DO REPLACE EXCLUDED [WHERE ...]
            if kw("EXCLUDED").parse_next(input).is_ok() {
                let where_clause = self.parse_optional_where(input, pctx)?;
                return Ok(ConflictAction::DoReplaceExcluded { where_clause });
            }

            // DO REPLACE VALUE expr [WHERE ...]
            if (kw("VALUE"), ws).parse_next(input).is_ok() {
                let value = self.pratt.parse_expr(input, pctx)?;
                let where_clause = self.parse_optional_where(input, pctx)?;
                return Ok(ConflictAction::DoReplaceValue {
                    value: Box::new(value),
                    where_clause,
                });
            }
        }

        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }

    fn parse_optional_where(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Option<Box<Expr>>> {
        let _ = ws0(input);
        let checkpoint = *input;
        if (kw("WHERE"), ws).parse_next(input).is_ok() {
            let expr = self.pratt.parse_expr(input, pctx)?;
            Ok(Some(Box::new(expr)))
        } else {
            *input = checkpoint;
            Ok(None)
        }
    }

    fn parse_set_clauses(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<Vec<SetClause>> {
        let mut clauses = Vec::new();
        loop {
            let _ = ws0(input);
            let column = identifier::identifier(input)?;
            let _ = ws0(input);
            ch('=').parse_next(input)?;
            let _ = ws0(input);

            let expr = self.parse_extended_expr(input, pctx)?;
            clauses.push(SetClause {
                column: column.to_string(),
                expr,
            });

            let _ = ws0(input);
            if ch(',').parse_next(input).is_err() {
                break;
            }
        }
        Ok(clauses)
    }

    fn parse_extended_expr(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
    ) -> PResult<ExtendedExpr> {
        // Try function call first: identifier(args...)
        let checkpoint = *input;
        if let Ok(name) = identifier::identifier(input) {
            let _ = ws0(input);
            if ch('(').parse_next(input).is_ok() {
                let mut args: Vec<Box<Expr>> = Vec::new();
                let _ = ws0(input);
                if ch(')').parse_next(input).is_ok() {
                    return Ok(ExtendedExpr::FunctionCall(MergeFunction {
                        function_name: name.to_string(),
                        arguments: args,
                    }));
                }
                loop {
                    let _ = ws0(input);
                    let arg = self.pratt.parse_expr(input, pctx)?;
                    args.push(Box::new(arg));
                    let _ = ws0(input);
                    if ch(',').parse_next(input).is_err() {
                        break;
                    }
                }
                let _ = ws0(input);
                ch(')').parse_next(input)?;
                return Ok(ExtendedExpr::FunctionCall(MergeFunction {
                    function_name: name.to_string(),
                    arguments: args,
                }));
            }
        }
        *input = checkpoint;

        // Regular expression
        let expr = self.pratt.parse_expr(input, pctx)?;
        Ok(ExtendedExpr::PartiQL(Box::new(expr)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::ExprChain;

    fn parse_oc(input: &str) -> OnConflict {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = input;
        OnConflictParser::new(chain.pratt())
            .parse(&mut i, &pctx)
            .expect("parse failed")
    }

    #[test]
    fn test_do_nothing() {
        let oc = parse_oc("ON CONFLICT DO NOTHING");
        assert!(matches!(oc.conflict_action, ConflictAction::DoNothing));
    }

    #[test]
    fn test_do_update_excluded() {
        let oc = parse_oc("ON CONFLICT DO UPDATE EXCLUDED");
        assert!(matches!(
            oc.conflict_action,
            ConflictAction::DoUpdateExcluded { where_clause: None }
        ));
    }

    #[test]
    fn test_do_update_excluded_where() {
        let oc = parse_oc("ON CONFLICT DO UPDATE EXCLUDED WHERE email = 'a@co'");
        match &oc.conflict_action {
            ConflictAction::DoUpdateExcluded { where_clause } => {
                assert!(where_clause.is_some());
            }
            other => panic!("expected DoUpdateExcluded, got {:?}", other),
        }
    }

    #[test]
    fn test_do_update_set() {
        let oc = parse_oc("ON CONFLICT DO UPDATE SET name = 'Bob', age = 30");
        match &oc.conflict_action {
            ConflictAction::DoUpdateSet {
                set_clauses,
                where_clause,
            } => {
                assert_eq!(set_clauses.len(), 2);
                assert_eq!(set_clauses[0].column, "name");
                assert_eq!(set_clauses[1].column, "age");
                assert!(where_clause.is_none());
            }
            other => panic!("expected DoUpdateSet, got {:?}", other),
        }
    }

    #[test]
    fn test_do_update_set_with_function() {
        let oc = parse_oc(
            "ON CONFLICT DO UPDATE SET tags = array_union(EXCLUDED.tags, tags)",
        );
        match &oc.conflict_action {
            ConflictAction::DoUpdateSet { set_clauses, .. } => {
                assert_eq!(set_clauses.len(), 1);
                assert!(matches!(
                    &set_clauses[0].expr,
                    ExtendedExpr::FunctionCall(f) if f.function_name == "array_union"
                ));
            }
            other => panic!("expected DoUpdateSet, got {:?}", other),
        }
    }

    #[test]
    fn test_conflict_target_columns() {
        let oc = parse_oc("ON CONFLICT (email, platform) DO NOTHING");
        match &oc.target {
            Some(ConflictTarget::Columns(cols)) => {
                assert_eq!(cols.as_slice(), &["email", "platform"]);
            }
            other => panic!("expected Columns target, got {:?}", other),
        }
    }

    #[test]
    fn test_do_replace_excluded() {
        let oc = parse_oc("ON CONFLICT DO REPLACE EXCLUDED");
        assert!(matches!(
            oc.conflict_action,
            ConflictAction::DoReplaceExcluded { where_clause: None }
        ));
    }

    #[test]
    fn test_full_insert_on_conflict_pattern() {
        let oc = parse_oc(
            "ON CONFLICT DO UPDATE SET \
             email = EXCLUDED.email, \
             name = EXCLUDED.name, \
             platformData = array_union(EXCLUDED.platformData, platformData)",
        );
        match &oc.conflict_action {
            ConflictAction::DoUpdateSet { set_clauses, .. } => {
                assert_eq!(set_clauses.len(), 3);
                assert!(matches!(&set_clauses[0].expr, ExtendedExpr::PartiQL(_)));
                assert!(matches!(&set_clauses[1].expr, ExtendedExpr::PartiQL(_)));
                assert!(matches!(
                    &set_clauses[2].expr,
                    ExtendedExpr::FunctionCall(f) if f.function_name == "array_union"
                ));
            }
            other => panic!("expected DoUpdateSet, got {:?}", other),
        }
    }
}

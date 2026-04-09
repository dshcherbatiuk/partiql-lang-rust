//! ParsedDml — single-pass DML parse with pre-extracted metadata.
//!
//! Returns FDE-ready types: table_name as String, values as Vec<Expr>,
//! optional OnConflict. Replaces fde-dml crate entirely.

use partiql_ast::ast::{self, Expr, OnConflict};
use winnow::prelude::*;

use super::on_conflict::OnConflictParser;
use crate::expr::ExprChain;
use crate::identifier;
use crate::keyword::{kw, lit};
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

/// Parsed DML operation with pre-extracted table_name and values.
#[derive(Debug, Clone)]
pub enum ParsedDml {
    Insert(InsertOp),
    InsertOnConflict(InsertOnConflictOp),
    Replace(ReplaceOp),
    Upsert(UpsertOp),
    Delete(DeleteOp),
}

#[derive(Debug, Clone)]
pub struct InsertOp {
    pub table_name: String,
    pub values: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct InsertOnConflictOp {
    pub table_name: String,
    pub values: Vec<Expr>,
    pub on_conflict: OnConflict,
}

#[derive(Debug, Clone)]
pub struct ReplaceOp {
    pub table_name: String,
    pub values: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct UpsertOp {
    pub table_name: String,
    pub values: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct DeleteOp {
    pub table_name: String,
    pub where_clause: Option<Expr>,
}

/// FDE DML parser — parses once, returns FDE-ready types.
pub struct DmlQueryParser<'p> {
    chain: &'p ExprChain,
}

impl<'p> DmlQueryParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self { chain }
    }

    /// Parse a DML statement. Returns None if input is not DML.
    pub fn parse(&self, input: &mut &str, pctx: &ParseContext) -> Option<PResult<ParsedDml>> {
        let checkpoint = *input;
        let _ = ws0(input);

        if (kw("INSERT"), ws, kw("INTO"), ws).parse_next(input).is_ok() {
            return Some(self.parse_insert(input, pctx));
        }
        *input = checkpoint;
        let _ = ws0(input);

        if (kw("REPLACE"), ws, kw("INTO"), ws).parse_next(input).is_ok() {
            return Some(self.parse_replace(input, pctx));
        }
        *input = checkpoint;
        let _ = ws0(input);

        if (kw("UPSERT"), ws, kw("INTO"), ws).parse_next(input).is_ok() {
            return Some(self.parse_upsert(input, pctx));
        }
        *input = checkpoint;
        let _ = ws0(input);

        if (kw("DELETE"), ws, kw("FROM"), ws).parse_next(input).is_ok() {
            return Some(self.parse_delete(input, pctx));
        }
        *input = checkpoint;

        None
    }

    fn parse_table_name(&self, input: &mut &str) -> PResult<String> {
        let (name, _quoted) = identifier::identifier_with_case(input)?;
        Ok(name.to_string())
    }

    fn parse_bag_values(&self, input: &mut &str, pctx: &ParseContext) -> PResult<Vec<Expr>> {
        let _ = ws0(input);
        lit("<<").parse_next(input)?;
        let _ = ws0(input);

        if lit(">>").parse_next(input).is_ok() {
            return Ok(Vec::new());
        }

        let mut values = Vec::new();
        loop {
            let _ = ws0(input);
            let expr = self.chain.parse_expr(input, pctx)?;
            values.push(expr);
            let _ = ws0(input);
            if winnow::token::one_of::<_, _, winnow::error::ContextError>(',')
                .parse_next(input)
                .is_err()
            {
                break;
            }
        }
        let _ = ws0(input);
        lit(">>").parse_next(input)?;
        Ok(values)
    }

    fn parse_insert(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ParsedDml> {
        let table_name = self.parse_table_name(input)?;
        let values = self.parse_bag_values(input, pctx)?;

        // Check for ON CONFLICT
        let _ = ws0(input);
        let checkpoint = *input;
        let oc_parser = OnConflictParser::new(self.chain.pratt());
        match oc_parser.parse(input, pctx) {
            Ok(on_conflict) => Ok(ParsedDml::InsertOnConflict(InsertOnConflictOp {
                table_name,
                values,
                on_conflict,
            })),
            Err(winnow::error::ErrMode::Backtrack(_)) => {
                *input = checkpoint;
                Ok(ParsedDml::Insert(InsertOp { table_name, values }))
            }
            Err(e) => Err(e),
        }
    }

    fn parse_replace(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ParsedDml> {
        let table_name = self.parse_table_name(input)?;
        let values = self.parse_bag_values(input, pctx)?;
        Ok(ParsedDml::Replace(ReplaceOp { table_name, values }))
    }

    fn parse_upsert(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ParsedDml> {
        let table_name = self.parse_table_name(input)?;
        let values = self.parse_bag_values(input, pctx)?;
        Ok(ParsedDml::Upsert(UpsertOp { table_name, values }))
    }

    fn parse_delete(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ParsedDml> {
        let table_name = self.parse_table_name(input)?;

        // Skip optional alias (not a keyword)
        let _ = ws0(input);
        let checkpoint = *input;
        if let Ok(word) = identifier::identifier(input) {
            if word.eq_ignore_ascii_case("WHERE") {
                *input = checkpoint; // put WHERE back
            }
            // else: alias consumed, continue
        } else {
            *input = checkpoint;
        }

        // Optional WHERE
        let _ = ws0(input);
        let checkpoint = *input;
        let where_clause = if (kw("WHERE"), ws).parse_next(input).is_ok() {
            Some(self.chain.parse_expr(input, pctx)?)
        } else {
            *input = checkpoint;
            None
        };

        Ok(ParsedDml::Delete(DeleteOp {
            table_name,
            where_clause,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(sql: &str) -> ParsedDml {
        let chain = ExprChain::new();
        let pctx = ParseContext::new();
        let mut i = sql;
        let parser = DmlQueryParser::new(&chain);
        parser
            .parse(&mut i, &pctx)
            .expect("not DML")
            .expect("parse failed")
    }

    #[test]
    fn test_insert() {
        match parse("INSERT INTO \"fde.users\" <<{'email': 'a@co'}, {'email': 'b@co'}>>") {
            ParsedDml::Insert(op) => {
                assert_eq!(op.table_name, "fde.users");
                assert_eq!(op.values.len(), 2);
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_insert_on_conflict() {
        match parse(
            "INSERT INTO \"fde.users\" <<{'email': 'a@co'}>> ON CONFLICT DO UPDATE SET email = EXCLUDED.email",
        ) {
            ParsedDml::InsertOnConflict(op) => {
                assert_eq!(op.table_name, "fde.users");
                assert_eq!(op.values.len(), 1);
            }
            other => panic!("expected InsertOnConflict, got {:?}", other),
        }
    }

    #[test]
    fn test_replace() {
        match parse("REPLACE INTO users <<{'email': 'a@co'}>>") {
            ParsedDml::Replace(op) => {
                assert_eq!(op.table_name, "users");
                assert_eq!(op.values.len(), 1);
            }
            other => panic!("expected Replace, got {:?}", other),
        }
    }

    #[test]
    fn test_upsert() {
        match parse("UPSERT INTO \"fde.users\" <<{'email': 'a@co'}>>") {
            ParsedDml::Upsert(op) => {
                assert_eq!(op.table_name, "fde.users");
            }
            other => panic!("expected Upsert, got {:?}", other),
        }
    }

    #[test]
    fn test_delete_where() {
        match parse("DELETE FROM \"fde.users\" WHERE email = 'a@co'") {
            ParsedDml::Delete(op) => {
                assert_eq!(op.table_name, "fde.users");
                assert!(op.where_clause.is_some());
            }
            other => panic!("expected Delete, got {:?}", other),
        }
    }

    #[test]
    fn test_delete_no_where() {
        match parse("DELETE FROM users") {
            ParsedDml::Delete(op) => {
                assert_eq!(op.table_name, "users");
                assert!(op.where_clause.is_none());
            }
            other => panic!("expected Delete, got {:?}", other),
        }
    }

    #[test]
    fn test_delete_with_alias() {
        match parse("DELETE FROM \"fde.users\" u WHERE u.email = 'a@co'") {
            ParsedDml::Delete(op) => {
                assert_eq!(op.table_name, "fde.users");
                assert!(op.where_clause.is_some());
            }
            other => panic!("expected Delete, got {:?}", other),
        }
    }

    #[test]
    #[test]
    fn test_insert_nested_fde_pattern() {
        match parse(
            r#"INSERT INTO "fde.users" <<{'email': 'user@co', 'platformData': [{'id': 'abc', 'platform': 'MsTeams'}]}>>"#,
        ) {
            ParsedDml::Insert(op) => {
                assert_eq!(op.table_name, "fde.users");
                assert_eq!(op.values.len(), 1);
                assert!(matches!(&op.values[0], Expr::Struct(_)));
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }
}

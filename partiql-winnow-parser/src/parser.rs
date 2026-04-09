//! WinnowParser — single entry point for all PartiQL queries.
//!
//! One call to `parse(sql)` determines query type and returns `ParsedQuery`.
//! Uses chain of responsibility: try DML, then DQL (SELECT).

use crate::dml::parsed_dml::{DmlQueryParser, ParsedDml};
use crate::dql::SelectParser;
use crate::expr::ExprChain;
use crate::parse_context::ParseContext;
use crate::parsed_select::ParsedSelect;

/// Result of parsing any PartiQL query.
#[derive(Debug)]
pub enum ParsedQuery {
    Select(ParsedSelect),
    Dml(ParsedDml),
}

/// Single entry point parser — stateless, created once, reused.
pub struct WinnowParser {
    chain: ExprChain,
    select_parser: SelectParser,
}

impl WinnowParser {
    pub fn new() -> Self {
        Self {
            chain: ExprChain::new(),
            select_parser: SelectParser::new(),
        }
    }

    /// Parse any PartiQL query in one call.
    pub fn parse(&self, sql: &str) -> Result<ParsedQuery, String> {
        // Try SELECT first (most frequent operation)
        if let Ok(select) = self.try_select(sql) {
            return Ok(select);
        }

        let pctx = ParseContext::new();
        // Try DML
        if let Some(result) = self.try_dml(sql, &pctx) {
            return result;
        }

        Err(format!("Failed to parse query: {sql}"))
    }

    fn try_dml(&self, sql: &str, pctx: &ParseContext) -> Option<Result<ParsedQuery, String>> {
        let parser = DmlQueryParser::new(&self.chain);
        let mut input = sql;
        match parser.parse(&mut input, pctx) {
            Some(Ok(dml)) => Some(Ok(ParsedQuery::Dml(dml))),
            Some(Err(e)) => Some(Err(format!("DML parse error: {e:?}"))),
            None => None,
        }
    }

    fn try_select(&self, sql: &str) -> Result<ParsedQuery, String> {
        ParsedSelect::parse(&self.select_parser, sql).map(ParsedQuery::Select)
    }
}

impl Default for WinnowParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use partiql_ast::ast::{BinOpKind, ConflictAction, Expr};

    fn parse(sql: &str) -> ParsedQuery {
        WinnowParser::new().parse(sql).expect("parse failed")
    }

    #[test]
    fn test_select_star() {
        match parse("SELECT * FROM users") {
            ParsedQuery::Select(s) => {
                assert_eq!(s.table_names.as_slice(), &["users"]);
                assert!(s.where_clause.is_none());
                assert!(s.unnest_aliases.is_empty());
            }
            other => panic!("expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_select_where() {
        match parse("SELECT a FROM t WHERE a = 1") {
            ParsedQuery::Select(s) => {
                assert_eq!(s.table_names.as_slice(), &["t"]);
                match &s.where_clause {
                    Some(Expr::BinOp(op)) => assert_eq!(op.node.kind, BinOpKind::Eq),
                    other => panic!("expected BinOp(Eq), got {:?}", other),
                }
            }
            other => panic!("expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_select_unnest() {
        match parse(r#"SELECT p.id FROM "fde.users" u, u.platformData p WHERE p.id = 'x'"#) {
            ParsedQuery::Select(s) => {
                assert_eq!(s.table_names.as_slice(), &["fde.users"]);
                assert!(s.where_clause.is_some());
                let alias = s.unnest_aliases.iter().find(|(k, _)| k == "p");
                assert_eq!(alias.map(|(_, v)| v.as_str()), Some("platformData"));
            }
            other => panic!("expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_insert() {
        match parse("INSERT INTO \"fde.users\" <<{'email': 'a@co'}>>") {
            ParsedQuery::Dml(ParsedDml::Insert(op)) => {
                assert_eq!(op.table_name, "fde.users");
                assert_eq!(op.values.len(), 1);
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_replace() {
        match parse("REPLACE INTO users <<{'email': 'a@co'}, {'email': 'b@co'}>>") {
            ParsedQuery::Dml(ParsedDml::Replace(op)) => {
                assert_eq!(op.table_name, "users");
                assert_eq!(op.values.len(), 2);
            }
            other => panic!("expected Replace, got {:?}", other),
        }
    }

    #[test]
    fn test_upsert() {
        match parse("UPSERT INTO \"fde.users\" <<{'email': 'a@co'}>>") {
            ParsedQuery::Dml(ParsedDml::Upsert(op)) => {
                assert_eq!(op.table_name, "fde.users");
                assert_eq!(op.values.len(), 1);
            }
            other => panic!("expected Upsert, got {:?}", other),
        }
    }

    #[test]
    fn test_delete_where() {
        match parse("DELETE FROM users WHERE email = 'a@co'") {
            ParsedQuery::Dml(ParsedDml::Delete(op)) => {
                assert_eq!(op.table_name, "users");
                match &op.where_clause {
                    Some(Expr::BinOp(b)) => assert_eq!(b.node.kind, BinOpKind::Eq),
                    other => panic!("expected BinOp(Eq), got {:?}", other),
                }
            }
            other => panic!("expected Delete, got {:?}", other),
        }
    }

    #[test]
    fn test_delete_no_where() {
        match parse("DELETE FROM users") {
            ParsedQuery::Dml(ParsedDml::Delete(op)) => {
                assert_eq!(op.table_name, "users");
                assert!(op.where_clause.is_none());
            }
            other => panic!("expected Delete, got {:?}", other),
        }
    }

    #[test]
    fn test_insert_on_conflict_do_nothing() {
        match parse("INSERT INTO users <<{'a': 'b'}>> ON CONFLICT DO NOTHING") {
            ParsedQuery::Dml(ParsedDml::InsertOnConflict(op)) => {
                assert_eq!(op.table_name, "users");
                assert_eq!(op.values.len(), 1);
                assert!(matches!(
                    op.on_conflict.conflict_action,
                    ConflictAction::DoNothing
                ));
            }
            other => panic!("expected InsertOnConflict, got {:?}", other),
        }
    }

    #[test]
    fn test_case_insensitive_select() {
        match parse("select a from t where a = 1") {
            ParsedQuery::Select(s) => {
                assert_eq!(s.table_names.as_slice(), &["t"]);
                assert!(s.where_clause.is_some());
            }
            other => panic!("expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_case_insensitive_dml() {
        match parse("insert into t <<{'a': 'b'}>>") {
            ParsedQuery::Dml(ParsedDml::Insert(op)) => {
                assert_eq!(op.table_name, "t");
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }
}

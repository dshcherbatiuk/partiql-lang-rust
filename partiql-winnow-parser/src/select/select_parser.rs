//! SelectParser — stateless, owns ExprChain and clause parsers.

use partiql_ast::ast;
use partiql_ast::ast::{Query, QuerySet, Select};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::from_clause::FromClauseParser;
use super::projection_clause::ProjectionClause;
use super::where_clause::WhereClauseParser;

/// Parses SELECT statements. Stateless — created once per engine, reused.
/// Clause parsers borrow the ExprChain via self-referential lifetime.
pub struct SelectParser {
    chain: ExprChain,
}

impl SelectParser {
    pub fn new() -> Self {
        Self {
            chain: ExprChain::new(),
        }
    }

    #[inline]
    pub fn chain(&self) -> &ExprChain {
        &self.chain
    }

    /// Parse a SELECT statement. `ParseContext` carries per-parse mutable state.
    pub fn parse<'a>(&self, input: &mut &'a str, pctx: &ParseContext) -> PResult<ast::Expr> {
        let projection_clause = ProjectionClause::new(&self.chain);
        let from_clause = FromClauseParser::new(&self.chain);
        let where_clause = WhereClauseParser::new(&self.chain);

        let _ = ws0(input);
        let _ = (kw("SELECT"), ws).parse_next(input)?;

        let projection = projection_clause.parse(input, pctx)?;

        let from = if (ws0, kw("FROM"), ws).parse_next(input).is_ok() {
            Some(from_clause.parse(input, pctx)?)
        } else {
            None
        };

        let where_ = if (ws0, kw("WHERE"), ws).parse_next(input).is_ok() {
            Some(where_clause.parse(input, pctx)?)
        } else {
            None
        };

        // TODO: GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET

        let select = Select {
            project: pctx.node(projection),
            exclude: None,
            from,
            from_let: None,
            where_clause: where_,
            group_by: None,
            having: None,
        };

        let query = Query {
            set: pctx.node(QuerySet::Select(Box::new(pctx.node(select)))),
            order_by: None,
            limit_offset: None,
        };

        Ok(ast::Expr::Query(pctx.node(query)))
    }
}

impl Default for SelectParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ast::Expr {
        let parser = SelectParser::new();
        let pctx = ParseContext::new();
        let mut i = input;
        parser.parse(&mut i, &pctx).expect("parse failed")
    }

    #[test]
    fn test_select_star() {
        assert!(matches!(parse("SELECT * FROM t"), ast::Expr::Query(_)));
    }

    #[test]
    fn test_select_field() {
        assert!(matches!(parse("SELECT a FROM t"), ast::Expr::Query(_)));
    }

    #[test]
    fn test_select_multiple_fields() {
        assert!(matches!(
            parse("SELECT a, b, c FROM t"),
            ast::Expr::Query(_)
        ));
    }

    #[test]
    fn test_select_with_where() {
        assert!(matches!(
            parse("SELECT a FROM t WHERE x = 1"),
            ast::Expr::Query(_)
        ));
    }

    #[test]
    fn test_select_with_alias() {
        assert!(matches!(
            parse("SELECT a FROM t AS u WHERE u.x = 1"),
            ast::Expr::Query(_)
        ));
    }

    #[test]
    fn test_select_value() {
        assert!(matches!(
            parse("SELECT VALUE x FROM t"),
            ast::Expr::Query(_)
        ));
    }

    #[test]
    fn test_select_distinct() {
        assert!(matches!(
            parse("SELECT DISTINCT a FROM t"),
            ast::Expr::Query(_)
        ));
    }

    #[test]
    fn test_select_complex_where() {
        assert!(matches!(
            parse("SELECT u.email FROM users u WHERE u.email = 'test@co.com' AND u.active = true"),
            ast::Expr::Query(_)
        ));
    }
}

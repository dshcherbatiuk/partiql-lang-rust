//! SelectParser — stateless, owns ExprChain and clause parsers.

use partiql_ast::ast;
use partiql_ast::ast::{Query, QuerySet, Select};
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::keyword::kw;
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

use super::from_clause::FromClauseParser;
use super::group_by_clause::GroupByClauseParser;
use super::having_clause::HavingClauseParser;
use super::limit_offset_clause::LimitOffsetClauseParser;
use super::order_by_clause::OrderByClauseParser;
use super::projection_clause::ProjectionClause;
use super::where_clause::WhereClauseParser;
use super::ClauseParser;

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
        let group_by_clause = GroupByClauseParser::new(&self.chain);
        let having_clause = HavingClauseParser::new(&self.chain);
        let order_by_clause = OrderByClauseParser::new(&self.chain);
        let limit_offset_clause = LimitOffsetClauseParser::new(&self.chain);

        let _ = ws0(input);
        let _ = (kw("SELECT"), ws).parse_next(input)?;

        let projection = projection_clause.parse(input, pctx)?;

        let from = {
            let checkpoint = *input;
            if (ws0, kw("FROM"), ws).parse_next(input).is_ok() {
                Some(from_clause.parse(input, pctx)?)
            } else {
                *input = checkpoint;
                None
            }
        };

        let where_ = {
            let checkpoint = *input;
            if (ws0, kw("WHERE"), ws).parse_next(input).is_ok() {
                Some(where_clause.parse(input, pctx)?)
            } else {
                *input = checkpoint;
                None
            }
        };

        let group_by = {
            let checkpoint = *input;
            if (ws0, kw("GROUP"), ws, kw("BY"), ws)
                .parse_next(input)
                .is_ok()
            {
                Some(group_by_clause.parse(input, pctx)?)
            } else {
                *input = checkpoint;
                None
            }
        };

        let having = {
            let checkpoint = *input;
            if (ws0, kw("HAVING"), ws).parse_next(input).is_ok() {
                Some(having_clause.parse(input, pctx)?)
            } else {
                *input = checkpoint;
                None
            }
        };

        let select = Select {
            project: pctx.node(projection),
            exclude: None,
            from,
            from_let: None,
            where_clause: where_,
            group_by,
            having,
        };

        let order_by = {
            let checkpoint = *input;
            if (ws0, kw("ORDER"), ws, kw("BY"), ws)
                .parse_next(input)
                .is_ok()
            {
                Some(order_by_clause.parse(input, pctx)?)
            } else {
                *input = checkpoint;
                None
            }
        };

        let limit_offset = {
            let checkpoint = *input;
            match limit_offset_clause.parse(input, pctx) {
                Ok(lo) => Some(lo),
                Err(winnow::error::ErrMode::Backtrack(_)) => {
                    *input = checkpoint;
                    None
                }
                Err(e) => return Err(e),
            }
        };

        let query = Query {
            set: pctx.node(QuerySet::Select(Box::new(pctx.node(select)))),
            order_by,
            limit_offset,
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
    use partiql_ast::ast::{
        BinOpKind, CallArg, OrderingSpec, ProjectionKind, QuerySet, SetQuantifier,
    };

    fn parse(input: &str) -> ast::Expr {
        let parser = SelectParser::new();
        let pctx = ParseContext::new();
        let mut i = input;
        parser.parse(&mut i, &pctx).expect("parse failed")
    }

    /// Extract (Query, Select) from parsed Expr for deep assertions.
    fn extract_query_select(expr: &ast::Expr) -> (&Query, &Select) {
        match expr {
            ast::Expr::Query(q) => match &q.node.set.node {
                QuerySet::Select(s) => (&q.node, &s.node),
                other => panic!("expected QuerySet::Select, got {:?}", other),
            },
            other => panic!("expected Expr::Query, got {:?}", other),
        }
    }

    #[test]
    fn test_select_star() {
        let expr = parse("SELECT * FROM t");
        let (_, select) = extract_query_select(&expr);
        assert!(matches!(
            select.project.node.kind,
            ProjectionKind::ProjectStar
        ));
        assert!(select.from.is_some());
    }

    #[test]
    fn test_select_field() {
        let expr = parse("SELECT a FROM t");
        let (_, select) = extract_query_select(&expr);
        match &select.project.node.kind {
            ProjectionKind::ProjectList(items) => assert_eq!(items.len(), 1),
            other => panic!("expected ProjectList, got {:?}", other),
        }
    }

    #[test]
    fn test_select_multiple_fields() {
        let expr = parse("SELECT a, b, c FROM t");
        let (_, select) = extract_query_select(&expr);
        match &select.project.node.kind {
            ProjectionKind::ProjectList(items) => assert_eq!(items.len(), 3),
            other => panic!("expected ProjectList, got {:?}", other),
        }
    }

    #[test]
    fn test_select_with_where() {
        let expr = parse("SELECT a FROM t WHERE x = 1");
        let (_, select) = extract_query_select(&expr);
        assert!(select.from.is_some());
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        assert!(matches!(&*where_clause.node.expr, ast::Expr::BinOp(_)));
    }

    #[test]
    fn test_select_with_alias() {
        let expr = parse("SELECT a FROM t AS u WHERE u.x = 1");
        let (_, select) = extract_query_select(&expr);
        assert!(select.from.is_some());
        assert!(select.where_clause.is_some());
    }

    #[test]
    fn test_select_value() {
        let expr = parse("SELECT VALUE x FROM t");
        let (_, select) = extract_query_select(&expr);
        assert!(matches!(
            select.project.node.kind,
            ProjectionKind::ProjectValue(_)
        ));
    }

    #[test]
    fn test_select_distinct() {
        let expr = parse("SELECT DISTINCT a FROM t");
        let (_, select) = extract_query_select(&expr);
        assert_eq!(
            select.project.node.setq,
            Some(SetQuantifier::Distinct)
        );
    }

    #[test]
    fn test_select_complex_where() {
        let expr = parse(
            "SELECT u.email FROM users u WHERE u.email = 'test@co.com' AND u.active = true",
        );
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        match &*where_clause.node.expr {
            ast::Expr::BinOp(n) => assert_eq!(n.node.kind, BinOpKind::And),
            other => panic!("expected BinOp(And), got {:?}", other),
        }
    }

    #[test]
    fn test_select_where_is_null() {
        let expr = parse("SELECT a FROM t WHERE x IS NULL");
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        match &*where_clause.node.expr {
            ast::Expr::BinOp(n) => assert_eq!(n.node.kind, BinOpKind::Is),
            other => panic!("expected BinOp(Is), got {:?}", other),
        }
    }

    #[test]
    fn test_select_where_is_not_null() {
        let expr = parse("SELECT a FROM t WHERE x IS NOT NULL");
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        assert!(matches!(&*where_clause.node.expr, ast::Expr::UniOp(_)));
    }

    #[test]
    fn test_select_where_in() {
        let expr = parse("SELECT a FROM t WHERE x IN (1, 2, 3)");
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        assert!(matches!(&*where_clause.node.expr, ast::Expr::In(_)));
    }

    #[test]
    fn test_select_where_not_in() {
        let expr = parse("SELECT a FROM t WHERE x NOT IN ('a', 'b')");
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        match &*where_clause.node.expr {
            ast::Expr::UniOp(n) => assert!(matches!(&*n.node.expr, ast::Expr::In(_))),
            other => panic!("expected UniOp(Not, In), got {:?}", other),
        }
    }

    #[test]
    fn test_select_where_like() {
        let expr = parse("SELECT a FROM t WHERE name LIKE '%foo%'");
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        assert!(matches!(&*where_clause.node.expr, ast::Expr::Like(_)));
    }

    #[test]
    fn test_select_where_between() {
        let expr = parse("SELECT a FROM t WHERE age BETWEEN 18 AND 65");
        let (_, select) = extract_query_select(&expr);
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        assert!(matches!(&*where_clause.node.expr, ast::Expr::Between(_)));
    }

    #[test]
    fn test_select_count_star() {
        let expr = parse("SELECT COUNT(*) FROM t");
        let (_, select) = extract_query_select(&expr);
        match &select.project.node.kind {
            ProjectionKind::ProjectList(items) => {
                assert_eq!(items.len(), 1);
                match &items[0].node {
                    ast::ProjectItem::ProjectExpr(pe) => match &*pe.expr {
                        ast::Expr::CallAgg(c) => {
                            assert_eq!(c.node.func_name.value, "COUNT");
                            assert_eq!(c.node.args.len(), 1);
                            assert!(matches!(c.node.args[0].node, CallArg::Star()));
                        }
                        other => panic!("expected CallAgg, got {:?}", other),
                    },
                    other => panic!("expected ProjectExpr, got {:?}", other),
                }
            }
            other => panic!("expected ProjectList, got {:?}", other),
        }
    }

    #[test]
    fn test_select_function_call() {
        let expr = parse("SELECT UPPER(name) FROM t WHERE LENGTH(name) > 3");
        let (_, select) = extract_query_select(&expr);
        // Projection has UPPER(name)
        match &select.project.node.kind {
            ProjectionKind::ProjectList(items) => {
                assert_eq!(items.len(), 1);
                match &items[0].node {
                    ast::ProjectItem::ProjectExpr(pe) => {
                        assert!(matches!(&*pe.expr, ast::Expr::Call(_)));
                    }
                    other => panic!("expected ProjectExpr, got {:?}", other),
                }
            }
            other => panic!("expected ProjectList, got {:?}", other),
        }
        // WHERE has LENGTH(name) > 3
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        match &*where_clause.node.expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, BinOpKind::Gt);
                assert!(matches!(&*n.node.lhs, ast::Expr::Call(_)));
            }
            other => panic!("expected BinOp(Gt), got {:?}", other),
        }
    }

    #[test]
    fn test_select_group_by() {
        let expr = parse("SELECT category, COUNT(*) FROM items GROUP BY category");
        let (_, select) = extract_query_select(&expr);
        let group_by = select.group_by.as_ref().expect("missing GROUP BY");
        assert_eq!(group_by.node.keys.len(), 1);
        assert!(group_by.node.strategy.is_none());
    }

    #[test]
    fn test_select_group_by_having() {
        let expr = parse(
            "SELECT category, COUNT(*) FROM items GROUP BY category HAVING COUNT(*) > 5",
        );
        let (_, select) = extract_query_select(&expr);
        assert!(select.group_by.is_some());
        let having = select.having.as_ref().expect("missing HAVING");
        match &*having.node.expr {
            ast::Expr::BinOp(n) => assert_eq!(n.node.kind, BinOpKind::Gt),
            other => panic!("expected BinOp(Gt), got {:?}", other),
        }
    }

    #[test]
    fn test_select_order_by() {
        let expr = parse("SELECT a FROM t ORDER BY a ASC");
        let (query, _) = extract_query_select(&expr);
        let order_by = query.order_by.as_ref().expect("missing ORDER BY");
        assert_eq!(order_by.node.sort_specs.len(), 1);
        assert_eq!(
            order_by.node.sort_specs[0].node.ordering_spec,
            Some(OrderingSpec::Asc)
        );
    }

    #[test]
    fn test_select_order_by_multiple() {
        let expr = parse("SELECT a, b FROM t ORDER BY a ASC, b DESC");
        let (query, _) = extract_query_select(&expr);
        let order_by = query.order_by.as_ref().expect("missing ORDER BY");
        assert_eq!(order_by.node.sort_specs.len(), 2);
        assert_eq!(
            order_by.node.sort_specs[0].node.ordering_spec,
            Some(OrderingSpec::Asc)
        );
        assert_eq!(
            order_by.node.sort_specs[1].node.ordering_spec,
            Some(OrderingSpec::Desc)
        );
    }

    #[test]
    fn test_select_limit() {
        let expr = parse("SELECT a FROM t LIMIT 10");
        let (query, _) = extract_query_select(&expr);
        let lo = query.limit_offset.as_ref().expect("missing LIMIT");
        assert!(lo.node.limit.is_some());
        assert!(lo.node.offset.is_none());
    }

    #[test]
    fn test_select_limit_offset() {
        let expr = parse("SELECT a FROM t LIMIT 10 OFFSET 5");
        let (query, _) = extract_query_select(&expr);
        let lo = query.limit_offset.as_ref().expect("missing LIMIT/OFFSET");
        assert!(lo.node.limit.is_some());
        assert!(lo.node.offset.is_some());
    }

    #[test]
    fn test_select_full_query() {
        let expr = parse(
            "SELECT category, COUNT(*) FROM items WHERE active = true GROUP BY category HAVING COUNT(*) > 1 ORDER BY category ASC LIMIT 10 OFFSET 0",
        );
        let (query, select) = extract_query_select(&expr);

        // Projection: 2 items
        match &select.project.node.kind {
            ProjectionKind::ProjectList(items) => assert_eq!(items.len(), 2),
            other => panic!("expected ProjectList, got {:?}", other),
        }

        assert!(select.from.is_some());
        assert!(select.where_clause.is_some());

        // GROUP BY: 1 key
        let group_by = select.group_by.as_ref().expect("missing GROUP BY");
        assert_eq!(group_by.node.keys.len(), 1);

        // HAVING: COUNT(*) > 1
        let having = select.having.as_ref().expect("missing HAVING");
        assert!(matches!(&*having.node.expr, ast::Expr::BinOp(_)));

        // ORDER BY: 1 spec, ASC
        let order_by = query.order_by.as_ref().expect("missing ORDER BY");
        assert_eq!(order_by.node.sort_specs.len(), 1);
        assert_eq!(
            order_by.node.sort_specs[0].node.ordering_spec,
            Some(OrderingSpec::Asc)
        );

        // LIMIT 10 OFFSET 0
        let lo = query.limit_offset.as_ref().expect("missing LIMIT/OFFSET");
        assert!(lo.node.limit.is_some());
        assert!(lo.node.offset.is_some());
    }

    #[test]
    fn test_select_unnest_fde_query() {
        // Real FDE production query with UNNEST via comma-join
        let expr = parse(
            "SELECT p.id, p.email, u.givenName FROM \"fde.users\" u, u.platformData p \
             WHERE p.id = '5e021ce0' AND p.platform = 'MsTeams'",
        );
        let (_, select) = extract_query_select(&expr);

        // Projection: 3 items
        match &select.project.node.kind {
            ProjectionKind::ProjectList(items) => assert_eq!(items.len(), 3),
            other => panic!("expected ProjectList, got {:?}", other),
        }

        // FROM: CROSS JOIN (comma-join)
        let from = select.from.as_ref().expect("missing FROM");
        match &from.node.source {
            ast::FromSource::Join(join) => {
                assert_eq!(join.node.kind, ast::JoinKind::Cross);
                // Left: "fde.users" u
                match &*join.node.left {
                    ast::FromSource::FromLet(fl) => {
                        assert_eq!(fl.node.as_alias.as_ref().unwrap().value, "u");
                    }
                    other => panic!("expected FromLet left, got {:?}", other),
                }
                // Right: u.platformData p (path with implicit alias)
                match &*join.node.right {
                    ast::FromSource::FromLet(fl) => {
                        assert_eq!(fl.node.as_alias.as_ref().unwrap().value, "p");
                        assert!(matches!(&*fl.node.expr, ast::Expr::Path(_)));
                    }
                    other => panic!("expected FromLet right, got {:?}", other),
                }
            }
            other => panic!("expected Join for comma-separated FROM, got {:?}", other),
        }

        // WHERE: AND with 2 equalities
        let where_clause = select.where_clause.as_ref().expect("missing WHERE");
        match &*where_clause.node.expr {
            ast::Expr::BinOp(n) => assert_eq!(n.node.kind, BinOpKind::And),
            other => panic!("expected BinOp(And), got {:?}", other),
        }
    }

    #[test]
    fn test_select_inner_join_on() {
        let expr = parse(
            "SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id",
        );
        let (_, select) = extract_query_select(&expr);

        let from = select.from.as_ref().expect("missing FROM");
        match &from.node.source {
            ast::FromSource::Join(join) => {
                assert_eq!(join.node.kind, ast::JoinKind::Inner);
                assert!(join.node.predicate.is_some());
            }
            other => panic!("expected Join, got {:?}", other),
        }
    }

    #[test]
    fn test_select_left_join() {
        let expr = parse(
            "SELECT u.name, o.total FROM users u LEFT JOIN orders o ON u.id = o.user_id WHERE u.active = true",
        );
        let (_, select) = extract_query_select(&expr);

        let from = select.from.as_ref().expect("missing FROM");
        match &from.node.source {
            ast::FromSource::Join(join) => {
                assert_eq!(join.node.kind, ast::JoinKind::Left);
            }
            other => panic!("expected Join, got {:?}", other),
        }
        assert!(select.where_clause.is_some());
    }
}

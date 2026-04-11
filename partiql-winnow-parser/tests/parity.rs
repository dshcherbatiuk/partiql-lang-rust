//! Test parity — compares winnow parser output against LALRPOP parser
//! on real FDE production queries.
//!
//! Each test parses the same query through both parsers and compares
//! structural properties of the resulting AST (not exact equality,
//! since node IDs differ).

use partiql_ast::ast::{self, BinOpKind, FromSource, JoinKind, ProjectionKind, QuerySet};
use partiql_parser::Parser;
use partiql_winnow_parser::parse_context::ParseContext;
use partiql_winnow_parser::dql::SelectParser;

// ── Helpers ─────────────────────────────────────────────────────────────

/// Parse with the existing LALRPOP parser, extract the Select node.
fn lalrpop_select(sql: &str) -> ast::Select {
    let parser = Parser::default();
    let parsed = parser.parse(sql).expect("LALRPOP parse failed");
    match &*parsed.ast.query.set {
        QuerySet::Select(select) => select.node.clone(),
        other => panic!("expected Select, got {:?}", other),
    }
}

/// Parse with winnow parser, extract the Select node.
fn winnow_select(sql: &str) -> (ast::Query, ast::Select) {
    let parser = SelectParser::new();
    let pctx = ParseContext::new();
    let mut i = sql;
    let expr = parser.parse(&mut i, &pctx).expect("winnow parse failed");
    match &expr {
        ast::Expr::Query(q) => match &q.node.set.node {
            QuerySet::Select(s) => (q.node.clone(), s.node.clone()),
            other => panic!("expected Select, got {:?}", other),
        },
        other => panic!("expected Query, got {:?}", other),
    }
}

// ── SELECT queries ──────────────────────────────────────────────────────

#[test]
fn parity_select_star() {
    let sql = "SELECT * FROM users";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    assert!(matches!(lalr.project.node.kind, ProjectionKind::ProjectStar));
    assert!(matches!(winn.project.node.kind, ProjectionKind::ProjectStar));
    assert!(lalr.from.is_some());
    assert!(winn.from.is_some());
}

/// `SELECT u.*` — `.*` is a PathUnpivot step on the alias `u`. Both parsers
/// must produce a ProjectList with one item that is `Path(VarRef("u"), [PathUnpivot])`.
#[test]
fn parity_select_path_unpivot() {
    use partiql_ast::ast::{Expr, PathStep, ProjectItem};

    let sql = r#"SELECT DISTINCT u.* FROM "schema.users" u WHERE u.email = 'a@b.c'"#;
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    for (label, kind) in [
        ("lalrpop", &lalr.project.node.kind),
        ("winnow", &winn.project.node.kind),
    ] {
        let items = match kind {
            ProjectionKind::ProjectList(items) => items,
            other => panic!("{label}: expected ProjectList, got {other:?}"),
        };
        assert_eq!(items.len(), 1, "{label}: expected one project item");
        let project_expr = match &items[0].node {
            ProjectItem::ProjectExpr(pe) => pe,
            other => panic!("{label}: expected ProjectExpr, got {other:?}"),
        };
        let path = match &*project_expr.expr {
            Expr::Path(p) => p,
            other => panic!("{label}: expected Path expression, got {other:?}"),
        };
        assert!(
            matches!(*path.node.root, Expr::VarRef(_)),
            "{label}: path root must be a VarRef"
        );
        assert_eq!(path.node.steps.len(), 1, "{label}: expected one path step");
        assert!(
            matches!(path.node.steps[0], PathStep::PathUnpivot),
            "{label}: expected PathUnpivot, got {:?}",
            path.node.steps[0]
        );
    }
}

#[test]
fn parity_select_fields_with_where() {
    let sql = "SELECT a, b FROM users WHERE a = 1";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&lalr.project.node.kind, &winn.project.node.kind) {
        (ProjectionKind::ProjectList(l), ProjectionKind::ProjectList(w)) => {
            assert_eq!(l.len(), w.len());
        }
        other => panic!("expected ProjectList, got {:?}", other),
    }
    assert!(lalr.where_clause.is_some());
    assert!(winn.where_clause.is_some());
}

#[test]
fn parity_select_with_alias() {
    let sql = "SELECT u.email FROM users u WHERE u.email = 'test@co'";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    assert!(lalr.from.is_some());
    assert!(winn.from.is_some());
    assert!(lalr.where_clause.is_some());
    assert!(winn.where_clause.is_some());

    // Both should have implicit alias "u"
    match (&lalr.from.unwrap().node.source, &winn.from.unwrap().node.source) {
        (FromSource::FromLet(l), FromSource::FromLet(w)) => {
            assert_eq!(
                l.node.as_alias.as_ref().unwrap().value,
                w.node.as_alias.as_ref().unwrap().value,
            );
        }
        other => panic!("expected FromLet, got {:?}", other),
    }
}

#[test]
fn parity_select_quoted_table() {
    let sql = r#"SELECT * FROM "fde.users""#;
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    // Both should parse "fde.users" as a VarRef with CaseSensitive
    match (&lalr.from.unwrap().node.source, &winn.from.unwrap().node.source) {
        (FromSource::FromLet(l), FromSource::FromLet(w)) => {
            match (&*l.node.expr, &*w.node.expr) {
                (ast::Expr::VarRef(lv), ast::Expr::VarRef(wv)) => {
                    assert_eq!(lv.node.name.value, wv.node.name.value);
                    assert_eq!(lv.node.name.case, wv.node.name.case);
                }
                other => panic!("expected VarRef for quoted table, got {:?}", other),
            }
        }
        other => panic!("expected FromLet, got {:?}", other),
    }
}

#[test]
fn parity_select_comma_join_unnest() {
    let sql = r#"SELECT p.id FROM "fde.users" u, u.platformData p WHERE p.id = 'abc'"#;
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    // Both should produce a Cross Join
    match (&lalr.from.unwrap().node.source, &winn.from.unwrap().node.source) {
        (FromSource::Join(lj), FromSource::Join(wj)) => {
            assert_eq!(lj.node.kind, wj.node.kind);
            assert_eq!(lj.node.kind, JoinKind::Cross);
        }
        other => panic!("expected Join for comma-separated FROM, got {:?}", other),
    }

    assert!(lalr.where_clause.is_some());
    assert!(winn.where_clause.is_some());
}

#[test]
fn parity_select_value() {
    let sql = "SELECT VALUE x FROM items x";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    assert!(matches!(lalr.project.node.kind, ProjectionKind::ProjectValue(_)));
    assert!(matches!(winn.project.node.kind, ProjectionKind::ProjectValue(_)));
}

#[test]
fn parity_select_distinct() {
    let sql = "SELECT DISTINCT a FROM t";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    assert_eq!(lalr.project.node.setq, winn.project.node.setq);
}

#[test]
fn parity_select_count_star() {
    let sql = "SELECT COUNT(*) FROM users";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&lalr.project.node.kind, &winn.project.node.kind) {
        (ProjectionKind::ProjectList(l), ProjectionKind::ProjectList(w)) => {
            assert_eq!(l.len(), 1);
            assert_eq!(w.len(), 1);
        }
        other => panic!("expected ProjectList, got {:?}", other),
    }
}

#[test]
fn parity_select_where_in() {
    let sql = "SELECT * FROM users WHERE email IN ('a@co', 'b@co')";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&*lalr.where_clause.unwrap().node.expr, &*winn.where_clause.unwrap().node.expr) {
        (ast::Expr::In(_), ast::Expr::In(_)) => {}
        other => panic!("expected In, got {:?}", other),
    }
}

#[test]
fn parity_select_where_is_null() {
    let sql = "SELECT * FROM users WHERE name IS NULL";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&*lalr.where_clause.unwrap().node.expr, &*winn.where_clause.unwrap().node.expr) {
        (ast::Expr::BinOp(l), ast::Expr::BinOp(w)) => {
            assert_eq!(l.node.kind, BinOpKind::Is);
            assert_eq!(w.node.kind, BinOpKind::Is);
        }
        other => panic!("expected BinOp(Is), got {:?}", other),
    }
}

#[test]
fn parity_select_where_like() {
    let sql = "SELECT * FROM users WHERE name LIKE '%foo%'";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&*lalr.where_clause.unwrap().node.expr, &*winn.where_clause.unwrap().node.expr) {
        (ast::Expr::Like(_), ast::Expr::Like(_)) => {}
        other => panic!("expected Like, got {:?}", other),
    }
}

#[test]
fn parity_select_where_between() {
    let sql = "SELECT * FROM users WHERE age BETWEEN 18 AND 65";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&*lalr.where_clause.unwrap().node.expr, &*winn.where_clause.unwrap().node.expr) {
        (ast::Expr::Between(_), ast::Expr::Between(_)) => {}
        other => panic!("expected Between, got {:?}", other),
    }
}

#[test]
fn parity_select_order_by_limit() {
    let sql = "SELECT a FROM t ORDER BY a ASC LIMIT 10";
    let lalr_parser = Parser::default();
    let lalr_parsed = lalr_parser.parse(sql).expect("LALRPOP parse failed");
    let lalr_query = &lalr_parsed.ast.query;

    let (winn_query, _) = winnow_select(sql);

    assert!(lalr_query.order_by.is_some());
    assert!(winn_query.order_by.is_some());
    assert!(lalr_query.limit_offset.is_some());
    assert!(winn_query.limit_offset.is_some());
}

#[test]
fn parity_select_group_by_having() {
    let sql = "SELECT category, COUNT(*) FROM items GROUP BY category HAVING COUNT(*) > 5";
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    assert!(lalr.group_by.is_some());
    assert!(winn.group_by.is_some());
    assert!(lalr.having.is_some());
    assert!(winn.having.is_some());

    assert_eq!(
        lalr.group_by.unwrap().node.keys.len(),
        winn.group_by.unwrap().node.keys.len(),
    );
}

// ── Real FDE production queries ─────────────────────────────────────────

#[test]
fn parity_fde_users_query() {
    let sql = r#"SELECT p.id, p.email, u.givenName, u.familyName, p.platform FROM "fde.users" u, u.platformData p WHERE p.id = '5e021ce0' AND p.platform = 'MsTeams'"#;
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    // Projection: 5 fields
    match (&lalr.project.node.kind, &winn.project.node.kind) {
        (ProjectionKind::ProjectList(l), ProjectionKind::ProjectList(w)) => {
            assert_eq!(l.len(), w.len());
            assert_eq!(l.len(), 5);
        }
        other => panic!("expected ProjectList, got {:?}", other),
    }

    // FROM: Cross Join (comma-join with unnest)
    match (&lalr.from.unwrap().node.source, &winn.from.unwrap().node.source) {
        (FromSource::Join(lj), FromSource::Join(wj)) => {
            assert_eq!(lj.node.kind, JoinKind::Cross);
            assert_eq!(wj.node.kind, JoinKind::Cross);
        }
        other => panic!("expected Cross Join, got {:?}", other),
    }

    // WHERE: AND
    match (&*lalr.where_clause.unwrap().node.expr, &*winn.where_clause.unwrap().node.expr) {
        (ast::Expr::BinOp(l), ast::Expr::BinOp(w)) => {
            assert_eq!(l.node.kind, BinOpKind::And);
            assert_eq!(w.node.kind, BinOpKind::And);
        }
        other => panic!("expected BinOp(And), got {:?}", other),
    }
}

#[test]
fn parity_fde_subscriptions_query() {
    let sql = r#"SELECT us.userEmail, us.chatId FROM "msteams.userSubscriptions" us WHERE us.chatId = 'chat-42'"#;
    let lalr = lalrpop_select(sql);
    let (_, winn) = winnow_select(sql);

    match (&lalr.project.node.kind, &winn.project.node.kind) {
        (ProjectionKind::ProjectList(l), ProjectionKind::ProjectList(w)) => {
            assert_eq!(l.len(), w.len());
            assert_eq!(l.len(), 2);
        }
        other => panic!("expected ProjectList, got {:?}", other),
    }
}

// DML parity skipped — current LALRPOP parser version doesn't support
// INSERT INTO with bag syntax. DML is implemented at the FDE layer.

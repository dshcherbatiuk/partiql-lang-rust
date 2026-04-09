//! ParsedDql — single-pass parse result with pre-extracted metadata.
//!
//! Eliminates the need to re-walk the AST after parsing to extract
//! WHERE clause, table names, and unnest aliases.

use partiql_ast::ast::{self, AstNode, Expr, FromSource, QuerySet, TopLevelQuery};
use smallvec::SmallVec;
/// Stack-allocated alias map — typically 0-2 entries.
pub type AliasMap = SmallVec<[(String, String); 4]>;

use crate::parse_context::ParseContext;
use crate::dql::SelectParser;

/// Parsed SELECT query with pre-extracted metadata.
#[derive(Debug)]
pub struct ParsedDql {
    /// Full AST for LogicalPlanner::lower()
    pub ast: AstNode<TopLevelQuery>,
    /// WHERE clause expression (if present)
    pub where_clause: Option<Expr>,
    /// All table names found in FROM clause
    pub table_names: SmallVec<[String; 4]>,
    /// Unnest aliases: alias → field path (e.g., "p" → "platformData")
    pub unnest_aliases: AliasMap,
}

impl ParsedDql {
    /// Parse a SELECT query and extract metadata in one pass.
    pub fn parse(parser: &SelectParser, sql: &str) -> Result<Self, String> {
        let pctx = ParseContext::new();
        let mut input = sql;

        let expr = parser
            .parse(&mut input, &pctx)
            .map_err(|e| format!("Parse error: {e:?}"))?;

        let query_node = match expr {
            Expr::Query(q) => q,
            other => {
                return Err(format!(
                    "Expected Query, got {:?}",
                    std::mem::discriminant(&other)
                ))
            }
        };

        let (where_clause, table_names, unnest_aliases) =
            extract_select_metadata(&query_node.node);

        let ast = pctx.node(TopLevelQuery {
            with: None,
            query: AstNode {
                id: query_node.id,
                node: query_node.node,
            },
        });

        Ok(Self {
            ast,
            where_clause,
            table_names,
            unnest_aliases,
        })
    }
}

// ── Metadata extraction (single walk) ───────────────────────────────────

fn extract_select_metadata(
    query: &ast::Query,
) -> (
    Option<Expr>,
    SmallVec<[String; 4]>,
    AliasMap,
) {
    match &*query.set {
        QuerySet::Select(select) => {
            let where_clause = select
                .node
                .where_clause
                .as_ref()
                .map(|w| (*w.node.expr).clone());

            let mut table_names = SmallVec::new();
            let mut unnest_aliases = AliasMap::new();

            if let Some(from) = &select.node.from {
                collect_table_names(&from.node.source, &mut table_names);
                collect_unnest_aliases(&from.node.source, &mut unnest_aliases);
            }

            (where_clause, table_names, unnest_aliases)
        }
        _ => (None, SmallVec::new(), AliasMap::new()),
    }
}

fn collect_table_names(source: &FromSource, names: &mut SmallVec<[String; 4]>) {
    match source {
        FromSource::FromLet(from_let) => {
            if let Some(name) = extract_table_name_from_expr(&from_let.node.expr) {
                names.push(name);
            }
        }
        FromSource::Join(join) => {
            collect_table_names(&join.node.left, names);
            collect_table_names(&join.node.right, names);
        }
    }
}

fn extract_table_name_from_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::VarRef(var_ref) => Some(var_ref.node.name.value.clone()),
        _ => None,
    }
}

fn collect_unnest_aliases(source: &FromSource, aliases: &mut AliasMap) {
    match source {
        FromSource::FromLet(_) => {}
        FromSource::Join(join) => {
            if let FromSource::FromLet(right) = &*join.node.right {
                if let Expr::Path(path_expr) = &*right.node.expr {
                    if let Some(alias) = &right.node.as_alias {
                        let field_parts: Vec<String> = path_expr
                            .node
                            .steps
                            .iter()
                            .filter_map(|step| match step {
                                ast::PathStep::PathProject(pe)
                                | ast::PathStep::PathIndex(pe) => match &*pe.index {
                                    Expr::VarRef(var_ref) => {
                                        Some(var_ref.node.name.value.clone())
                                    }
                                    Expr::Lit(lit) => {
                                        if let ast::Lit::CharStringLit(s) = &lit.node {
                                            Some(s.clone())
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                },
                                _ => None,
                            })
                            .collect();

                        if !field_parts.is_empty() {
                            aliases.push((alias.value.clone(), field_parts.join(".")));
                        }
                    }
                }
            }

            collect_unnest_aliases(&join.node.left, aliases);
            collect_unnest_aliases(&join.node.right, aliases);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dql::SelectParser;

    fn parse(sql: &str) -> ParsedDql {
        let parser = SelectParser::new();
        ParsedDql::parse(&parser, sql).expect("parse failed")
    }

    #[test]
    fn test_simple_select() {
        let result = parse("SELECT * FROM users WHERE email = 'test@co'");
        assert_eq!(result.table_names.as_slice(), &["users"]);
        assert!(result.where_clause.is_some());
        assert!(result.unnest_aliases.is_empty());
    }

    #[test]
    fn test_quoted_table() {
        let result = parse(r#"SELECT * FROM "fde.users" WHERE email = 'test@co'"#);
        assert_eq!(result.table_names.as_slice(), &["fde.users"]);
    }

    #[test]
    fn test_unnest_pattern() {
        let result = parse(
            r#"SELECT p.id FROM "fde.users" u, u.platformData p WHERE p.id = 'abc'"#,
        );
        assert_eq!(result.table_names.len(), 1); // "fde.users", path is not a table
        let p_alias = result
            .unnest_aliases
            .iter()
            .find(|(k, _)| k == "p")
            .map(|(_, v)| v.as_str());
        assert_eq!(p_alias, Some("platformData"));
        assert!(result.where_clause.is_some());
    }

    #[test]
    fn test_no_where() {
        let result = parse("SELECT * FROM users");
        assert!(result.where_clause.is_none());
    }

    #[test]
    fn test_ast_available() {
        let result = parse("SELECT a FROM t WHERE a = 1");
        // AST can be passed to LogicalPlanner::lower()
        match &*result.ast.node.query.node.set {
            QuerySet::Select(_) => {}
            other => panic!("expected Select, got {:?}", other),
        }
    }
}

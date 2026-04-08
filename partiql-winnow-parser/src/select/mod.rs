//! SELECT statement parsing.
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ SelectParser (stateless, created once)   │
//! │                                          │
//! │  chain: ExprChain                        │
//! │                                          │
//! │ ClauseParser trait:                      │
//! │  ProjectionClause                        │
//! │  FromClauseParser                        │
//! │  WhereClauseParser                       │
//! │  GroupByClauseParser                     │
//! │  HavingClauseParser                      │
//! │  OrderByClauseParser                     │
//! │  LimitOffsetClauseParser                 │
//! │                                          │
//! │ ParseContext (per-parse mutable state)    │
//! │  — only argument to parse()              │
//! └──────────────────────────────────────────┘
//! ```

pub mod from_clause;
pub mod group_by_clause;
pub mod join;
pub mod having_clause;
pub mod limit_offset_clause;
pub mod order_by_clause;
pub mod projection_clause;
mod select_parser;
pub mod where_clause;

pub use select_parser::SelectParser;

use winnow::prelude::*;

use crate::parse_context::ParseContext;

/// Each SELECT clause implements this trait.
///
/// `Output` is the clause-specific AST node (e.g., `Projection`, `AstNode<FromClause>`).
/// Clause parsers are stateless — they hold `&ExprChain` for expression delegation
/// and receive `&ParseContext` per parse call for mutable state.
pub trait ClauseParser {
    type Output;

    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<Self::Output>;
}

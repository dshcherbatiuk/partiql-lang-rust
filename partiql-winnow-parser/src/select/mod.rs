//! SELECT statement parsing.
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ SelectParser (stateless, created once)   │
//! │                                          │
//! │  chain: ExprChain                        │
//! │  projection: ProjectionClause            │
//! │  from: FromClause                        │
//! │  where_: WhereClause                     │
//! │  // TODO: GroupByClause, HavingClause,   │
//! │  //       OrderByClause, LimitOffset     │
//! │                                          │
//! │ ParseContext (per-parse mutable state)    │
//! │  — only argument to parse()              │
//! └──────────────────────────────────────────┘
//! ```

pub mod from_clause;
pub mod projection_clause;
mod select_parser;
pub mod where_clause;

pub use select_parser::SelectParser;

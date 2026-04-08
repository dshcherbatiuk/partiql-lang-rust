//! Ion-compatible literal value parsing for PartiQL.
//!
//! PartiQL values are Ion values. This module parses the full Ion text format
//! (https://amazon-ion.github.io/ion-docs/) plus SQL extensions:
//!
//! | Ion Type | Syntax | SQL Extension |
//! |----------|--------|---------------|
//! | null | `null`, `null.int`, `null.string` | `MISSING` |
//! | bool | `true`, `false` | |
//! | int | `42`, `0xFACE`, `0b1010`, `1_000` | |
//! | decimal | `3.14`, `1.0d2` | |
//! | float | `1.0e0`, `nan`, `+inf`, `-inf` | |
//! | timestamp | `2024-01-15T12:30:00Z` (bare, not quoted) | |
//! | string | `"hello"` (Ion double-quoted) | `'hello'` (SQL single-quoted) |
//! | symbol | `foo`, `'quoted symbol'`, `$10` | |
//! | blob | `{{ SGVsbG8= }}` | |
//! | clob | `{{ "raw bytes" }}` | |
//! | list | `[1, 2, 3]` | |
//! | sexp | `(+ 1 2)` | |
//! | struct | `{ field: value }` | |
//! | bag | `<< 1, 2, 3 >>` | PartiQL extension |
//! | annotation | `name::value`, `a::b::42` | |
//!
//! ## Annotations
//!
//! Any Ion value can be prefixed with one or more annotations:
//! ```text
//! annotated_value ::= (symbol '::')* value
//! ```
//!
//! ## Timestamps
//!
//! Ion timestamps are bare (not quoted), following ISO 8601:
//! ```text
//! timestamp ::= YYYY 'T'
//!             | YYYY '-' MM 'T'
//!             | YYYY '-' MM '-' DD 'T'?
//!             | YYYY '-' MM '-' DD 'T' HH ':' MM ('+' | '-') HH ':' MM
//!             | YYYY '-' MM '-' DD 'T' HH ':' MM ':' SS ('.' frac)? offset
//! offset    ::= 'Z' | ('+' | '-') HH ':' MM
//! ```

pub mod bag_strategy;
pub mod boolean_strategy;
pub mod ion;
pub mod list_strategy;
pub mod null_strategy;
pub mod number_strategy;
pub mod string_strategy;
pub mod struct_strategy;

use crate::expr::StrategyContext;
use partiql_ast::ast;
use winnow::prelude::*;

/// Strategy trait for literal parsing. Each Ion/SQL literal type
/// implements this. Used by PrimaryStrategy's internal chain.
pub trait LiteralStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr>;
    fn name(&self) -> &str;
}

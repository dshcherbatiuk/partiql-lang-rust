//! Comparison special forms — IS, IN, LIKE, BETWEEN.
//!
//! Each form is a separate parser implementing `ComparisonParser`.
//! `ComparisonStrategy` chains them before falling through to operators.

pub mod between_parser;
pub mod in_parser;
pub mod is_parser;
pub mod like_parser;

use partiql_ast::ast;
use winnow::prelude::*;

use super::StrategyContext;

/// Each comparison special form implements this trait.
///
/// Receives the already-parsed `left` operand and attempts to parse
/// the rest (e.g., `IS NULL`, `IN (...)`, `LIKE '%foo'`, `BETWEEN 1 AND 10`).
/// Returns `Backtrack` if this form doesn't match.
pub trait ComparisonParser {
    fn parse<'a>(
        &self,
        input: &mut &'a str,
        ctx: &StrategyContext<'_>,
        left: &ast::Expr,
    ) -> PResult<ast::Expr>;

    fn name(&self) -> &str;
}

//! JOIN parsers — each join type is a separate strategy.
//!
//! `FromClauseParser` chains these after parsing the first source.
//! Each parser holds `&ExprChain` for expression delegation and
//! receives `&ParseContext` + `left` source per call.

pub mod comma_join;
pub mod cross_join;
pub mod full_join;
pub mod inner_join;
pub mod left_join;
pub mod right_join;

use partiql_ast::ast::FromSource;
use winnow::prelude::*;

use crate::parse_context::ParseContext;

/// Each JOIN type implements this trait.
///
/// Holds `&ExprChain` in the struct. Receives the already-parsed
/// `left` source and attempts to match its join keyword.
/// Returns `Backtrack` if this join doesn't match.
pub trait JoinParser: Send + Sync {
    fn parse(
        &self,
        input: &mut &str,
        pctx: &ParseContext,
        left: &FromSource,
    ) -> PResult<FromSource>;
}

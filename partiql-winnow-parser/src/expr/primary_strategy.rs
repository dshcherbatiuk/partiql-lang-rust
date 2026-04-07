//! PrimaryStrategy — highest precedence, bottom of the expression chain.
//!
//! Handles atomic expressions that don't involve operators:
//! ```text
//! primary ::= literal
//!           | identifier
//!           | '*'
//!           | '(' expr ')'
//!           | function_call
//!           | bag_expr
//!           | list_expr
//!           | struct_expr
//! ```

use partiql_ast::ast;
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};

pub struct PrimaryStrategy;

impl ExprStrategy for PrimaryStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let _ = crate::whitespace::ws0(input);
        // TODO: implement primary expression parsing
        // For now, fail — strategies above will be added incrementally
        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }

    fn name(&self) -> &str {
        "Primary"
    }
}

#[cfg(test)]
mod tests {
    // Tests will compare winnow output with LALRPOP output on same input
}

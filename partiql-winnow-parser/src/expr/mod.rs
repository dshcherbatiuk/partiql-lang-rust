//! Expression parsing — Strategy + Chain of Responsibility pattern.
//!
//! Each precedence level is a strategy. The chain delegates down from
//! lowest to highest precedence. Produces `partiql_ast::ast::Expr` directly.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────┐
//! │ ExprChain                                             │
//! │                                                       │
//! │  [0] OrStrategy          ← entry point (lowest prec) │
//! │  [1] AndStrategy                                      │
//! │  [2] NotStrategy                                      │
//! │  [3] ComparisonStrategy  (= != < > IS IN LIKE)        │
//! │  [4] AddSubStrategy      (+ - ||)                     │
//! │  [5] MulDivStrategy      (* / %)                      │
//! │  [6] UnaryStrategy       (- + NOT)                    │
//! │  [7] PostfixStrategy     (. [] ())                    │
//! │  [8] PrimaryStrategy     (lit ident parens fn)        │
//! └───────────────────────────────────────────────────────┘
//! ```

pub mod primary_strategy;

use partiql_ast::ast;
use winnow::prelude::*;

/// Context passed to each strategy — provides access to the chain
/// and current level. Stack-allocated, 16 bytes, zero heap.
///
/// Extensible: future fields (location tracker, error state) added
/// here without changing the strategy trait signature.
pub struct StrategyContext<'c> {
    chain: &'c ExprChain,
    level: usize,
}

impl<'c> StrategyContext<'c> {
    /// Parse a sub-expression at the next (higher-precedence) level.
    #[inline]
    pub fn parse_next_level<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.chain.parse_at(input, self.level + 1)
    }

    /// Parse a full expression from the lowest precedence level.
    /// Used for sub-expressions in parentheses, function args, etc.
    #[inline]
    pub fn parse_expr<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.chain.parse_expr(input)
    }
}

/// Each precedence level implements this trait.
pub trait ExprStrategy {
    /// Parse at this precedence level.
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr>;

    fn name(&self) -> &str;
}

/// Chain of expression strategies in precedence order (lowest → highest).
pub struct ExprChain {
    strategies: Vec<Box<dyn ExprStrategy>>,
}

impl ExprChain {
    pub fn new() -> Self {
        Self {
            strategies: vec![Box::new(primary_strategy::PrimaryStrategy)],
        }
    }

    /// Parse at the given precedence level.
    pub fn parse_at<'a>(&self, input: &mut &'a str, level: usize) -> PResult<ast::Expr> {
        let ctx = StrategyContext { chain: self, level };
        self.strategies[level].parse(input, &ctx)
    }

    /// Entry point: parse a full expression at the lowest precedence.
    pub fn parse_expr<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.parse_at(input, 0)
    }
}

impl Default for ExprChain {
    fn default() -> Self {
        Self::new()
    }
}

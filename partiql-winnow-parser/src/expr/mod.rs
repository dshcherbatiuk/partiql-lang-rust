//! Expression parsing — Strategy + Chain of Responsibility pattern.
//!
//! Each precedence level is a strategy. The chain delegates down from
//! lowest to highest precedence. Produces `partiql_ast::ast::Expr` directly.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────┐
//! │ ExprChain (stateless — created once, reused)         │
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

pub mod add_sub_strategy;
pub mod and_strategy;
pub mod comparison;
pub mod comparison_strategy;
pub mod mul_div_strategy;
pub mod not_strategy;
pub mod or_strategy;
pub mod postfix_strategy;
pub mod primary_strategy;
pub mod unary_strategy;

use partiql_ast::ast;
use winnow::prelude::*;

use crate::parse_context::ParseContext;

/// Context passed to each strategy — provides access to the chain,
/// current precedence level, and shared parse state.
///
/// Stack-allocated reference. Strategies use this to delegate to
/// next levels and create AST nodes.
pub struct StrategyContext<'c> {
    chain: &'c ExprChain,
    level: usize,
    pub(crate) parse_ctx: &'c ParseContext,
}

impl<'c> StrategyContext<'c> {
    /// Parse a sub-expression at the next (higher-precedence) level.
    #[inline]
    pub fn parse_next_level<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.chain.parse_at(input, self.level + 1, self.parse_ctx)
    }

    /// Parse a full expression from the lowest precedence level.
    #[inline]
    pub fn parse_expr<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.chain.parse_expr(input, self.parse_ctx)
    }

    /// Create an AST node with a fresh ID.
    #[inline]
    pub fn node<T>(&self, value: T) -> ast::AstNode<T> {
        self.parse_ctx.node(value)
    }
}

/// Each precedence level implements this trait.
pub trait ExprStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr>;
    fn name(&self) -> &str;
}

/// Chain of expression strategies — stateless, created once, reused.
pub struct ExprChain {
    strategies: Vec<Box<dyn ExprStrategy>>,
}

impl ExprChain {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(or_strategy::OrStrategy),
                Box::new(and_strategy::AndStrategy),
                Box::new(not_strategy::NotStrategy),
                Box::new(comparison_strategy::ComparisonStrategy::new()),
                Box::new(add_sub_strategy::AddSubStrategy),
                Box::new(mul_div_strategy::MulDivStrategy),
                Box::new(unary_strategy::UnaryStrategy),
                Box::new(postfix_strategy::PostfixStrategy),
                Box::new(primary_strategy::PrimaryStrategy::new()),
            ],
        }
    }

    /// Parse at the given precedence level.
    pub fn parse_at<'a>(
        &self,
        input: &mut &'a str,
        level: usize,
        parse_ctx: &ParseContext,
    ) -> PResult<ast::Expr> {
        let ctx = StrategyContext {
            chain: self,
            level,
            parse_ctx,
        };
        self.strategies[level].parse(input, &ctx)
    }

    /// Parse a full expression at the lowest precedence.
    pub fn parse_expr<'a>(
        &self,
        input: &mut &'a str,
        parse_ctx: &ParseContext,
    ) -> PResult<ast::Expr> {
        self.parse_at(input, 0, parse_ctx)
    }
}

impl Default for ExprChain {
    fn default() -> Self {
        Self::new()
    }
}

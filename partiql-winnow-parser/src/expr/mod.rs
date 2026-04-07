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
use partiql_ast::ast::AstNode;
use partiql_common::node::{AutoNodeIdGenerator, NodeIdGenerator};
use std::cell::RefCell;
use winnow::prelude::*;

/// Context passed to each strategy — provides access to the chain,
/// current precedence level, and a node ID generator for AST construction.
///
/// Stack-allocated reference. Extensible: add location tracker, error
/// state, etc. without changing the strategy trait signature.
pub struct StrategyContext<'c> {
    chain: &'c ExprChain,
    level: usize,
    ids: &'c RefCell<AutoNodeIdGenerator>,
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

    /// Create an AST node with a fresh ID.
    #[inline]
    pub fn node<T>(&self, value: T) -> AstNode<T> {
        AstNode {
            id: self.ids.borrow_mut().next_id(),
            node: value,
        }
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
    ids: RefCell<AutoNodeIdGenerator>,
}

impl ExprChain {
    pub fn new() -> Self {
        Self {
            strategies: vec![Box::new(primary_strategy::PrimaryStrategy::new())],
            ids: RefCell::new(AutoNodeIdGenerator::default()),
        }
    }

    /// Parse at the given precedence level.
    pub fn parse_at<'a>(&self, input: &mut &'a str, level: usize) -> PResult<ast::Expr> {
        let ctx = StrategyContext {
            chain: self,
            level,
            ids: &self.ids,
        };
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

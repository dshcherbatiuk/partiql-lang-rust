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
pub mod pratt;
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
    pratt: &'c pratt::PrattParser,
    pub(crate) parse_ctx: &'c ParseContext,
}

impl<'c> StrategyContext<'c> {
    pub fn new(pratt: &'c pratt::PrattParser, parse_ctx: &'c ParseContext) -> Self {
        Self { pratt, parse_ctx }
    }

    /// Parse a sub-expression at "next level" — in Pratt terms, parse with
    /// min_bp high enough to stop before AND/OR (used by BETWEEN, comparison RHS).
    #[inline]
    pub fn parse_next_level<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        // bp=5 stops before AND(3) and OR(1), but allows arithmetic(7+) and comparison(5+)
        self.pratt.parse_bp(input, self.parse_ctx, 5)
    }

    /// Parse a full expression (delegates to Pratt parser).
    #[inline]
    pub fn parse_expr<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.pratt.parse_expr(input, self.parse_ctx)
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

/// Expression parser — delegates to PrattParser.
/// Keeps the `ExprChain` name for API compatibility with clause/join/DML parsers.
pub struct ExprChain {
    pratt: pratt::PrattParser,
}

impl ExprChain {
    pub fn new() -> Self {
        Self {
            pratt: pratt::PrattParser::new(),
        }
    }

    /// Parse a full expression.
    #[inline]
    pub fn parse_expr<'a>(
        &self,
        input: &mut &'a str,
        parse_ctx: &ParseContext,
    ) -> PResult<ast::Expr> {
        self.pratt.parse_expr(input, parse_ctx)
    }

    /// Access the underlying Pratt parser.
    #[inline]
    pub fn pratt(&self) -> &pratt::PrattParser {
        &self.pratt
    }
}

impl Default for ExprChain {
    fn default() -> Self {
        Self::new()
    }
}

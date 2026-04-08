//! Expression parsing — Pratt parser with binding power.
//!
//! Single-loop expression engine replacing the 9-level recursive chain.
//! For a literal `1`: 1 call. For `a = 1 AND b = 2`: ~7 calls.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────┐
//! │ PrattParser (single loop, binding power table)        │
//! │                                                       │
//! │  Prefix:  NOT, -, +, literals, identifiers, parens   │
//! │  Infix:   OR, AND, =, !=, <, >, +, -, *, /, ||      │
//! │  Special: IS, IN, LIKE, BETWEEN, NOT IN/LIKE/BETWEEN │
//! │  Postfix: . [] (path access)                         │
//! │                                                       │
//! │  PrimaryStrategy: literals, identifiers, functions   │
//! │  ComparisonParsers: IS, IN, LIKE, BETWEEN            │
//! └───────────────────────────────────────────────────────┘
//! ```

pub mod comparison;
pub mod pratt;
pub mod primary_strategy;

use partiql_ast::ast;
use winnow::prelude::*;

use crate::parse_context::ParseContext;

/// Context passed to PrimaryStrategy and ComparisonParsers.
/// Provides expression parsing delegation and AST node creation.
pub struct StrategyContext<'c> {
    pratt: &'c pratt::PrattParser,
    pub(crate) parse_ctx: &'c ParseContext,
}

impl<'c> StrategyContext<'c> {
    pub fn new(pratt: &'c pratt::PrattParser, parse_ctx: &'c ParseContext) -> Self {
        Self { pratt, parse_ctx }
    }

    /// Parse a sub-expression stopping before AND/OR (bp=5).
    /// Used by BETWEEN (to not consume AND), comparison RHS.
    #[inline]
    pub fn parse_next_level<'a>(&self, input: &mut &'a str) -> PResult<ast::Expr> {
        self.pratt.parse_bp(input, self.parse_ctx, 5)
    }

    /// Parse a full expression from lowest precedence.
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

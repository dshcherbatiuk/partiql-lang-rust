//! Per-parse mutable state — node IDs, location tracking, errors.
//!
//! Created once per `parse()` call. Shared by all strategies and clauses.
//! Not `Send` — single-threaded parse context.

use partiql_ast::ast::AstNode;
use partiql_common::node::{AutoNodeIdGenerator, NodeIdGenerator};
use std::cell::RefCell;

/// Per-parse mutable state. Created for each `parse()` invocation,
/// shared by all strategies via reference.
pub struct ParseContext {
    ids: RefCell<AutoNodeIdGenerator>,
    // TODO: location tracker, error collector
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            ids: RefCell::new(AutoNodeIdGenerator::default()),
        }
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

impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

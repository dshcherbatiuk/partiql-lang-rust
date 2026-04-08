//! Per-parse mutable state — node IDs, location tracking, errors.
//!
//! Created once per `parse()` call. Shared by all strategies and clauses.
//! Not `Send` — single-threaded parse context.

use partiql_ast::ast::AstNode;
use partiql_common::node::NodeId;
use std::cell::Cell;

/// Per-parse mutable state. Created for each `parse()` invocation,
/// shared by all strategies via reference.
///
/// Uses `Cell<u32>` for node ID generation — zero overhead compared
/// to `RefCell<AutoNodeIdGenerator>` (no borrow checks).
pub struct ParseContext {
    next_id: Cell<u32>,
    // TODO: location tracker, error collector
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            next_id: Cell::new(1),
        }
    }

    /// Create an AST node with a fresh ID.
    #[inline]
    pub fn node<T>(&self, value: T) -> AstNode<T> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        AstNode {
            id: NodeId(id),
            node: value,
        }
    }
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

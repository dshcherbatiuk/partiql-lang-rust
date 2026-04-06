//! Aggregate pushdown trait for COUNT.
//!
//! Storage engines implement [`CollCount`] to return row count without
//! materializing all rows into a `Bag`. The evaluator checks
//! [`EvalContext::as_coll_count`] before falling back to iteration.

/// Trait for optimized COUNT pushdown.
///
/// Storage engines implement this on their custom `EvalContext` to provide
/// O(1) row counts. The evaluator calls this before iterating the collection.
pub trait CollCount {
    /// Returns the count of rows in the current collection.
    fn coll_count(&self) -> usize;
}

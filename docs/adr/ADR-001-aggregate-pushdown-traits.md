# ADR-001: Aggregate Function Pushdown via Traits

**Status**: Proposed
**Date**: 2026-04-06

## Context

PartiQL evaluates aggregate functions (COUNT, SUM, MIN, MAX, AVG) by:

1. Materializing all rows from the data source into a `Bag` (in-memory `Vec<Value>`)
2. Iterating the Bag to compute the aggregate

For storage engines that keep data in a non-`Value` format (e.g. serialized binary, columnar, on-disk), step 1 is the bottleneck. A `SELECT COUNT(x) FROM table` forces the storage to deserialize every row into a `PartiQLValue` just to count them — O(n) deserialization for an answer the storage already knows.

The storage engine has no way to communicate pre-computed or O(1) answers to the evaluator.

## Decision

Add **one method per aggregate function** to `EvalContext`, each with a default implementation that returns `None` (preserving current behavior). Storage engines provide a custom `EvalContext` that overrides specific methods to return pre-computed results.

### EvalContext Extension

```rust
pub trait EvalContext: Bindings<Value> + SessionContext + Debug {
    fn add_error(&self, error: EvaluationError);
    fn has_errors(&self) -> bool;
    fn errors(&self) -> Vec<EvaluationError>;

    /// Aggregate pushdown: COUNT.
    /// Returns the count of rows for the named binding without materialization.
    /// Default: None — evaluator falls back to standard iteration.
    fn coll_count(&self, binding_name: &str) -> Option<usize> {
        None
    }

    // Future aggregate pushdowns:
    // fn coll_sum(&self, binding_name: &str, field: &str) -> Option<Value> { None }
    // fn coll_min(&self, binding_name: &str, field: &str) -> Option<Value> { None }
    // fn coll_max(&self, binding_name: &str, field: &str) -> Option<Value> { None }
    // fn coll_avg(&self, binding_name: &str, field: &str) -> Option<Value> { None }
}
```

### Evaluation Flow

```text
BEFORE (current):
  Bindings → materialize ALL rows into Bag → COLL_COUNT iterates Bag → count
  Cost: O(n) deserialization + O(n) iteration

AFTER (with pushdown):
  EvalContext.coll_count("binding") → Some(n) → return Value::Integer(n)
  EvalContext.coll_count("binding") → None    → current behavior (no change)
  Cost: O(1) when storage supports it, O(n) otherwise
```

### Where the Check Happens

In `EvalCollFn::Count` binding (`partiql-eval/src/eval/expr/coll.rs`), before creating the iterator-based evaluation:

```rust
EvalCollFn::Count(setq) => {
    // Try pushdown first — storage may answer without iteration
    if setq == SetQuantifier::All {
        if let Some(count) = context.coll_count(source_binding) {
            return Ok(Value::Integer(count as i64));
        }
    }
    // Fallback: standard iteration
    create::<STRICT, _>([any_elems], args, move |it| it.coll_count(setq))
}
```

### Storage Engine Integration

A storage engine provides a custom `EvalContext`:

```rust
struct CustomContext<'u> {
    inner: BasicContext<'u>,
    storage: Arc<dyn StorageBackend>,
}

impl EvalContext for CustomContext<'_> {
    // delegate add_error, has_errors, errors to inner

    fn coll_count(&self, binding_name: &str) -> Option<usize> {
        self.storage.row_count(binding_name)
    }
}
```

`BasicContext` is unchanged — its default `coll_count` returns `None`.

### Scope Limitations

Pushdown applies only when the evaluator can guarantee the aggregate covers the full binding:
- **No WHERE clause** — filtered counts need materialization
- **No DISTINCT** — distinct counts need value comparison
- **No UNNEST** — unnested counts depend on per-row array sizes

The default `None` return ensures safety — unsupported patterns always fall back to standard evaluation.

## Consequences

### Positive
- Aggregates over large tables drop from O(n) deserialization to O(1)
- No breaking changes — `BasicContext` and all existing code unchanged
- Pattern extends naturally to SUM, MIN, MAX, AVG
- Storage engines opt in per aggregate — no all-or-nothing

### Negative
- `EvalContext` grows one method per aggregate (mitigated by default `None` impls)
- Pushdown check adds one branch per COUNT evaluation (~1ns when None)
- Storage engines that want pushdown must provide a custom `EvalContext` instead of using `BasicContext`

### Future Extensions
- `coll_count_filtered(binding, predicate)` — pushdown for COUNT with WHERE
- `coll_sum`, `coll_min`, `coll_max`, `coll_avg` — same pattern
- `coll_count_distinct` — pushdown for COUNT(DISTINCT ...)

## Key Files

| File | Change |
|------|--------|
| `partiql-eval/src/eval/mod.rs` | Add `coll_count` default method to `EvalContext` |
| `partiql-eval/src/eval/expr/coll.rs` | Check pushdown before creating iterator |

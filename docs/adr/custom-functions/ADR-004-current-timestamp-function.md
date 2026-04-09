# ADR-004: CURRENT_TIMESTAMP Function

**Status**: Implemented
**Date**: 2025-10-16

## Context

PartiQL specification references `CURRENT_TIMESTAMP` as a standard SQL function, but the partiql-rust v0.14.0 implementation did not include it. FDE requires current timestamp support for:
- Audit trails (`created_at`, `updated_at` fields)
- Replication conflict resolution (version-based timestamps)
- Query filtering by time ranges

## Decision

Implement `CURRENT_TIMESTAMP()` as a built-in function that returns the current UTC timestamp as an Ion timestamp value.

### Syntax

```sql
CURRENT_TIMESTAMP()
```

### Return Type

Ion timestamp with nanosecond precision and UTC timezone offset.

### Examples

```sql
SELECT CURRENT_TIMESTAMP() as now
-- Returns: 2025-10-16T14:30:00.123456789Z

SELECT * FROM events WHERE created_at > CURRENT_TIMESTAMP()
```

## Key Files

| File | Change |
|------|--------|
| `partiql-eval/src/eval/builtins.rs` | Register CURRENT_TIMESTAMP function |
| `extension/partiql-extension-ion/src/datetime.rs` | Ion timestamp construction |

## Consequences

### Positive
- Standard SQL compatibility for datetime operations
- Enables time-based queries without client-side timestamp injection

### Negative
- Non-deterministic function — same query returns different results at different times

# ADR-007: UNIX_TIMESTAMP Function

**Status**: Implemented
**Date**: 2025-11-27

## Context

FDE needs Unix epoch timestamps (seconds since 1970-01-01T00:00:00Z) for:
- Interop with external systems that use epoch seconds
- Token expiration calculations (`tokenCreatedAt + expiresIn`)
- Efficient numeric timestamp comparisons

## Decision

Implement `UNIX_TIMESTAMP([datetime])` as a built-in function that returns the number of seconds since Unix epoch.

### Syntax

```sql
-- Current time as epoch seconds
UNIX_TIMESTAMP()

-- Convert timestamp to epoch seconds
UNIX_TIMESTAMP(timestamp_expr)
```

### Return Type

Integer (i64) — seconds since 1970-01-01T00:00:00Z.

### Examples

```sql
SELECT UNIX_TIMESTAMP() as epoch_now
-- Returns: 1730000000

SELECT UNIX_TIMESTAMP(created_at) as epoch_created FROM events

-- Token expiration check
SELECT * FROM tokens
WHERE UNIX_TIMESTAMP(tokenCreatedAt) + expiresIn > UNIX_TIMESTAMP()
```

## Key Files

| File | Change |
|------|--------|
| `partiql-eval/src/eval/builtins.rs` | Register UNIX_TIMESTAMP function |
| `extension/partiql-extension-ion/src/datetime.rs` | Epoch conversion |

## Consequences

### Positive
- Numeric timestamp for efficient comparisons
- Standard Unix epoch interop with external systems
- No-arg form provides current time as epoch

### Negative
- Seconds precision only (no milliseconds) — sufficient for FDE use cases
- Integer overflow in year 2038 for 32-bit systems (not applicable — uses i64)

# ADR-005: CURRENT_TIME Function

**Status**: Implemented
**Date**: 2025-10-16

## Context

FDE needs time-only (without date) functionality for scheduling, time-of-day filtering, and display formatting. PartiQL did not implement `CURRENT_TIME`.

## Decision

Implement `CURRENT_TIME()` as a built-in function that returns the current UTC time as an Ion timestamp value (date portion set to epoch).

### Syntax

```sql
CURRENT_TIME()
```

### Return Type

Ion timestamp representing time-of-day with nanosecond precision.

### Examples

```sql
SELECT CURRENT_TIME() as now_time
-- Returns: 1970-01-01T14:30:00.123Z

SELECT TO_STRING(CURRENT_TIME(), 'HH:mm:ss') as formatted
-- Returns: "14:30:00"
```

## Key Files

| File | Change |
|------|--------|
| `partiql-eval/src/eval/builtins.rs` | Register CURRENT_TIME function |
| `extension/partiql-extension-ion/src/datetime.rs` | Time-only timestamp construction |

## Consequences

### Positive
- Time-of-day queries without date component
- Composable with TO_STRING for formatting

### Negative
- Returns timestamp with epoch date — callers must understand the convention

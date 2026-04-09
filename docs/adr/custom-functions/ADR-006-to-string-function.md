# ADR-006: TO_STRING Function

**Status**: Implemented
**Date**: 2025-10-16

## Context

FDE needs to format Ion timestamp values into human-readable strings for API responses, logging, and display. PartiQL did not implement `TO_STRING`. The Java `DateTimeFormatter` pattern syntax is the standard in the FDE ecosystem.

## Decision

Implement `TO_STRING(timestamp, format_pattern)` as a built-in function that formats an Ion timestamp using Java `DateTimeFormatter` pattern syntax.

### Syntax

```sql
TO_STRING(timestamp_expr, format_string)
```

### Parameters

- `timestamp_expr` — Ion timestamp value (from CURRENT_TIMESTAMP, column, etc.)
- `format_string` — Java DateTimeFormatter pattern string

### Supported Patterns

| Pattern | Description | Example |
|---------|-------------|---------|
| `yyyy` | 4-digit year | 2025 |
| `MM` | 2-digit month | 10 |
| `dd` | 2-digit day | 16 |
| `HH` | 24-hour hour | 14 |
| `mm` | Minute | 30 |
| `ss` | Second | 45 |
| `MMMM` | Full month name | October |
| `h` | 12-hour hour | 2 |
| `a` | AM/PM | PM |

### Examples

```sql
SELECT TO_STRING(CURRENT_TIMESTAMP(), 'yyyy-MM-dd HH:mm:ss') as formatted
-- Returns: "2025-10-16 14:30:45"

SELECT TO_STRING(CURRENT_TIMESTAMP(), 'MMMM d, y h:m a') as friendly
-- Returns: "October 16, 2025 2:30 PM"

SELECT TO_STRING(created_at, 'yyyy-MM-dd') as date_only FROM events
```

## Key Files

| File | Change |
|------|--------|
| `partiql-eval/src/eval/builtins.rs` | Register TO_STRING function |
| `extension/partiql-extension-ion/src/datetime.rs` | Timestamp formatting with chrono |

## Consequences

### Positive
- Human-readable timestamp output without client-side formatting
- Java DateTimeFormatter compatibility (same patterns used in Java services)
- Composable with CURRENT_TIMESTAMP and CURRENT_TIME

### Negative
- Pattern syntax tied to Java conventions (not POSIX strftime)
- Locale-dependent patterns (month names) use English

# Winnow Parser Benchmark Report

## Date: 2026-04-08 (v2)

## Environment
- Apple Silicon (aarch64-apple-darwin)
- Rust 1.88.0, release profile
- Criterion 0.5, 100 samples per benchmark

## SELECT Queries — LALRPOP vs Winnow

| Query | LALRPOP | Winnow | Speedup |
|-------|---------|--------|---------|
| `SELECT * FROM users` | 864ns | 364ns | **2.4x** |
| `SELECT a, b, c FROM users WHERE a = 1` | 2.53µs | 1.21µs | **2.1x** |
| `SELECT u.email FROM users u WHERE u.email = 'test@co'` | 2.30µs | 1.01µs | **2.3x** |
| `SELECT * FROM "fde.users" WHERE email = 'test@co'` | 1.56µs | 657ns | **2.4x** |
| `SELECT p.id, p.email FROM "fde.users" u, u.platformData p WHERE ...` | 4.28µs | 1.96µs | **2.2x** |
| `SELECT * FROM users WHERE email IN ('a@co', 'b@co', 'c@co')` | 2.63µs | 1.09µs | **2.4x** |
| `SELECT (15 cols) FROM "fde.users" u, u.platformData p WHERE ... AND ...` | 11.0µs | 5.46µs | **2.0x** |
| `SELECT COUNT(*) FROM users WHERE active = true` | 2.13µs | 882ns | **2.4x** |
| `SELECT ... GROUP BY ... HAVING ... ORDER BY ... LIMIT` | 3.94µs | 1.69µs | **2.3x** |

**Average speedup: 2.3x**

## DML Queries — Winnow Only

LALRPOP parser doesn't support FDE DML syntax (INSERT INTO ... << >>).

| Query | Winnow |
|-------|--------|
| `INSERT INTO users <<{...}>>` | 1.18µs |
| `INSERT INTO "fde.users" <<{...}>>` | 1.44µs |
| `INSERT INTO "fde.users" <<{... nested ...}>>` | 2.35µs |
| `INSERT INTO users <<{...}, {...}, {...}>>` | 1.73µs |
| `UPSERT INTO "fde.users" <<{...}>>` | 1.17µs |
| `UPSERT INTO "fde.users" <<{... nested ...}>>` | 2.41µs |
| `REPLACE INTO "fde.users" <<{... nested ...}>>` | 2.08µs |
| `DELETE FROM "fde.users" WHERE email = 'test@co'` | 542ns |

## Key Optimizations Applied

### Phase 1 (1.8-2.1x)
1. **Zero-alloc identifiers** — `identifier()` returns `&str` slice, `.to_string()` only at AST boundary
2. **First-char dispatch** — `PrimaryStrategy` matches first byte to jump directly to the right parser, avoiding 8 failed checkpoint/backtrack cycles for identifiers
3. **Borrowed join sources** — `JoinParser` accepts `&FromSource`, clones only on successful match
4. **Word-boundary `kw()`** — prevents `OR` matching `ORDER`, `IN` matching `INSERT`
5. **`<<`/`>>` boundary** — comparison operators `<`/`>` don't match bag delimiters

### Phase 2 (2.0-2.4x)
6. **Byte-level `kw()` boundary** — `as_bytes().first()` instead of `chars().next()` (no Unicode iterator)
7. **`Cell<u32>` node IDs** — replaced `RefCell<AutoNodeIdGenerator>` with zero-overhead `Cell`
8. **Inline ASCII whitespace** — `ws()`/`ws0()` use byte loop instead of winnow scanner
9. **`#[inline]` on hot paths** — `parse_single_lt`, `parse_single_gt`, identifier functions

## Benchmark History

| Version | Avg Speedup | Key Change |
|---------|-------------|------------|
| v1 (initial) | 1.2-1.5x | Strategy + Chain of Responsibility pattern |
| v2 (zero-alloc) | 1.8-2.1x | &str identifiers, first-char dispatch, borrowed joins |
| v3 (byte-level) | **2.0-2.4x** | Byte-level kw(), Cell IDs, inline ASCII whitespace |

## Methodology

Benchmark runs Criterion with 100 samples, 3s warmup per query. Both parsers produce the same `partiql-ast` AST types, verified by 16 parity tests comparing structural properties.

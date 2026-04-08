# Winnow Parser Benchmark Report

## Date: 2026-04-08 (v3)

## Environment
- Apple Silicon (aarch64-apple-darwin)
- Rust 1.88.0, release profile
- Criterion 0.5, 100 samples per benchmark

## SELECT Queries — LALRPOP vs Winnow

| Query | LALRPOP | Winnow | Speedup |
|-------|---------|--------|---------|
| `SELECT * FROM users` | 863ns | 255ns | **3.4x** |
| `SELECT a, b, c FROM users WHERE a = 1` | 2.51µs | 650ns | **3.9x** |
| `SELECT u.email FROM users u WHERE u.email = 'test@co'` | 2.28µs | 722ns | **3.2x** |
| `SELECT * FROM "fde.users" WHERE email = 'test@co'` | 1.55µs | 415ns | **3.7x** |
| `SELECT p.id, p.email FROM "fde.users" u, u.platformData p WHERE ...` | 4.27µs | 1.42µs | **3.0x** |
| `SELECT * FROM users WHERE email IN ('a@co', 'b@co', 'c@co')` | 2.60µs | 604ns | **4.3x** |
| `SELECT (15 cols) FROM "fde.users" u, u.platformData p WHERE ... AND ...` | 10.9µs | 3.66µs | **3.0x** |
| `SELECT COUNT(*) FROM users WHERE active = true` | 2.08µs | 538ns | **3.9x** |
| `SELECT ... GROUP BY ... HAVING ... ORDER BY ... LIMIT` | 3.88µs | 944ns | **4.1x** |

**Average speedup: 3.6x**

## DML Queries — Winnow Only

LALRPOP parser doesn't support FDE DML syntax (INSERT INTO ... << >>).

| Query | Winnow |
|-------|--------|
| `INSERT INTO users <<{...}>>` | 436ns |
| `INSERT INTO "fde.users" <<{...}>>` | 519ns |
| `INSERT INTO "fde.users" <<{... nested ...}>>` | 784ns |
| `INSERT INTO users <<{...}, {...}, {...}>>` | 650ns |
| `UPSERT INTO "fde.users" <<{...}>>` | 440ns |
| `UPSERT INTO "fde.users" <<{... nested ...}>>` | 799ns |
| `REPLACE INTO "fde.users" <<{... nested ...}>>` | 697ns |
| `DELETE FROM "fde.users" WHERE email = 'test@co'` | 316ns |

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

### Phase 3 (3.0-4.4x)
10. **Pratt parser** — replaced 9-level recursive `ExprChain` with single-loop binding power parser. For literal `1`: 1 call instead of 9. For `a = 1 AND b = 2`: ~7 calls instead of ~45.
11. **Byte-level operator dispatch** — `peek_infix` matches operators via `as_bytes()` without winnow combinators
12. **Direct PrimaryStrategy call** — no ExprStrategy vtable indirection

## Benchmark History

| Version | Avg Speedup | Key Change |
|---------|-------------|------------|
| v1 (initial) | 1.2-1.5x | Strategy + Chain of Responsibility pattern |
| v2 (zero-alloc) | 1.8-2.1x | &str identifiers, first-char dispatch, borrowed joins |
| v3 (byte-level) | 2.0-2.4x | Byte-level kw(), Cell IDs, inline ASCII whitespace |
| v4 (Pratt) | **3.0-4.4x** | Pratt parser replaces 9-level ExprChain |

## Methodology

Benchmark runs Criterion with 100 samples, 3s warmup per query. Both parsers produce the same `partiql-ast` AST types, verified by 16 parity tests comparing structural properties.

# Winnow Parser Benchmark Report

## Date: 2026-04-08 (v4 — FDE integrated)

## Environment
- Apple Silicon (aarch64-apple-darwin)
- Rust 1.88.0, release profile
- Criterion 0.5, 100 samples per benchmark

## SELECT Queries — LALRPOP vs Winnow (all versions)

| Query | LALRPOP | v2 (zero-alloc) | v3 (byte-level) | v4 (Pratt) | Speedup |
|-------|---------|-----------------|-----------------|------------|---------|
| `SELECT * FROM users` | 863ns | 718ns | 364ns | **263ns** | **3.3x** |
| `SELECT a, b, c FROM users WHERE a = 1` | 2.51µs | 2.03µs | 1.21µs | **729ns** | **3.4x** |
| `SELECT u.email FROM users u WHERE ...` | 2.28µs | 1.72µs | 1.01µs | **758ns** | **3.0x** |
| `SELECT * FROM "fde.users" WHERE ...` | 1.55µs | 1.15µs | 657ns | **421ns** | **3.7x** |
| `SELECT ... FROM "fde.users" u, u.platformData p WHERE ...` | 4.27µs | 4.04µs | 1.96µs | **1.46µs** | **2.9x** |
| `SELECT * FROM users WHERE email IN ('a', 'b', 'c')` | 2.60µs | 1.71µs | 1.09µs | **638ns** | **4.1x** |
| `SELECT * FROM users WHERE email IN ['a', 'b', 'c']` | — | — | — | **629ns** | — |
| `SELECT (15 cols) ... WHERE ... AND ...` | 10.9µs | 9.15µs | 5.46µs | **3.96µs** | **2.8x** |
| `SELECT COUNT(*) FROM users WHERE active = true` | 2.08µs | 1.54µs | 882ns | **545ns** | **3.8x** |
| `SELECT ... GROUP BY ... HAVING ... ORDER BY ... LIMIT` | 3.88µs | 2.94µs | 1.69µs | **967ns** | **4.0x** |
| `SELECT ... WHERE u.email IN [...] AND p.originalPlatformId = '...'` | — | — | — | **1.31µs** | — |

**Average speedup vs LALRPOP: 3.4x**

## DML Queries — Winnow (all versions)

| Query | v3 (byte-level) | v4 (Pratt) | Improvement |
|-------|-----------------|------------|-------------|
| `INSERT INTO users <<{...}>>` | 1.18µs | **440ns** | 2.7x |
| `INSERT INTO "fde.users" <<{...}>>` | 1.44µs | **519ns** | 2.8x |
| `INSERT INTO "fde.users" <<{... nested ...}>>` | 2.35µs | **790ns** | 3.0x |
| `INSERT INTO users <<{...}, {...}, {...}>>` | 1.73µs | **649ns** | 2.7x |
| `UPSERT INTO "fde.users" <<{...}>>` | 1.17µs | **445ns** | 2.6x |
| `UPSERT INTO "fde.users" <<{... nested ...}>>` | 2.41µs | **812ns** | 3.0x |
| `REPLACE INTO "fde.users" <<{... nested ...}>>` | 2.08µs | **691ns** | 3.0x |
| `DELETE FROM "fde.users" WHERE email = 'test@co'` | 542ns | **317ns** | 1.7x |

## ON CONFLICT Queries — Winnow

| Query | Time |
|-------|------|
| `ON CONFLICT DO NOTHING` | **21ns** |
| `ON CONFLICT DO REPLACE EXCLUDED` | **35ns** |
| `ON CONFLICT DO UPDATE EXCLUDED WHERE email = '...'` | **157ns** |
| `ON CONFLICT DO UPDATE SET name = '...', age = 30` | **224ns** |
| `ON CONFLICT DO UPDATE SET ... = EXCLUDED..., ... = array_union(...)` (3 clauses + merge fn) | **796ns** |

## Key Optimizations Applied

### Phase 1 (1.8-2.1x)
1. **Zero-alloc identifiers** — `identifier()` returns `&str` slice, `.to_string()` only at AST boundary
2. **First-char dispatch** — `PrimaryStrategy` matches first byte to jump directly to the right parser
3. **Borrowed join sources** — `JoinParser` accepts `&FromSource`, clones only on successful match
4. **Word-boundary `kw()`** — prevents `OR` matching `ORDER`, `IN` matching `INSERT`
5. **`<<`/`>>` boundary** — comparison operators `<`/`>` don't match bag delimiters

### Phase 2 (2.0-2.4x)
6. **Byte-level `kw()` boundary** — `as_bytes().first()` instead of `chars().next()`
7. **`Cell<u32>` node IDs** — replaced `RefCell<AutoNodeIdGenerator>` with zero-overhead `Cell`
8. **Inline ASCII whitespace** — `ws()`/`ws0()` use byte loop instead of winnow scanner
9. **`#[inline]` on hot paths** — identifier functions, lt/gt boundary checks

### Phase 3 (2.8-4.1x)
10. **Pratt parser** — replaced 9-level recursive `ExprChain` with single-loop binding power parser
11. **Byte-level operator dispatch** — `peek_infix` matches operators via `as_bytes()`
12. **Direct PrimaryStrategy call** — no ExprStrategy vtable indirection

### Phase 4 (FDE integration)
13. **ParsedSelect** — single-pass parse returns AST + WHERE + table names + unnest aliases
14. **LogicalPlanner::lower()** accepts `&AstNode<TopLevelQuery>` directly (no `Parsed` wrapper)
15. **partiql-parser removed** — FDE depends only on winnow parser end-to-end
16. **IN [...] bracket syntax** — IN parser accepts any expression, not just parenthesized lists

## Benchmark History

| Version | Avg Speedup vs LALRPOP | Key Change |
|---------|----------------------|------------|
| v1 (initial) | 1.2-1.5x | Strategy + Chain of Responsibility pattern |
| v2 (zero-alloc) | 1.8-2.1x | &str identifiers, first-char dispatch, borrowed joins |
| v3 (byte-level) | 2.0-2.4x | Byte-level kw(), Cell IDs, inline ASCII whitespace |
| v4 (Pratt + FDE) | **2.8-4.1x** | Pratt parser, ParsedSelect, partiql-parser removed |

## Methodology

Benchmark runs Criterion with 100 samples, 3s warmup per query. Both parsers produce the same `partiql-ast` AST types, verified by 16 parity tests comparing structural properties.

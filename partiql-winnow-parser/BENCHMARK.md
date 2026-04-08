# Winnow Parser Benchmark Report

## Date: 2026-04-08

## Environment
- Apple Silicon (aarch64-apple-darwin)
- Rust 1.88.0, release profile
- Criterion 0.5, 100 samples per benchmark

## SELECT Queries — LALRPOP vs Winnow

| Query | LALRPOP | Winnow | Speedup |
|-------|---------|--------|---------|
| `SELECT * FROM users` | 879ns | 441ns | **2.0x** |
| `SELECT a, b, c FROM users WHERE a = 1` | 2.50µs | 1.45µs | **1.7x** |
| `SELECT u.email FROM users u WHERE u.email = 'test@co'` | 2.30µs | 1.18µs | **2.0x** |
| `SELECT * FROM "fde.users" WHERE email = 'test@co'` | 1.58µs | 774ns | **2.0x** |
| `SELECT p.id, p.email FROM "fde.users" u, u.platformData p WHERE ...` | 4.31µs | 2.24µs | **1.9x** |
| `SELECT * FROM users WHERE email IN ('a@co', 'b@co', 'c@co')` | 2.62µs | 1.26µs | **2.1x** |
| `SELECT (15 cols) FROM "fde.users" u, u.platformData p WHERE ... AND ...` | 11.0µs | 6.15µs | **1.8x** |
| `SELECT COUNT(*) FROM users WHERE active = true` | 2.13µs | 1.03µs | **2.1x** |
| `SELECT ... GROUP BY ... HAVING ... ORDER BY ... LIMIT` | 3.90µs | 2.06µs | **1.9x** |

**Average speedup: 1.9x**

## DML Queries — Winnow Only

LALRPOP parser doesn't support FDE DML syntax (INSERT INTO ... << >>).

| Query | Winnow |
|-------|--------|
| `INSERT INTO users <<{...}>>` | 1.34µs |
| `INSERT INTO "fde.users" <<{...}>>` | 1.72µs |
| `INSERT INTO "fde.users" <<{... nested ...}>>` | 2.72µs |
| `INSERT INTO users <<{...}, {...}, {...}>>` | 2.03µs |
| `UPSERT INTO "fde.users" <<{...}>>` | 1.34µs |
| `UPSERT INTO "fde.users" <<{... nested ...}>>` | 2.76µs |
| `REPLACE INTO "fde.users" <<{... nested ...}>>` | 2.40µs |
| `DELETE FROM "fde.users" WHERE email = 'test@co'` | 655ns |

## Key Optimizations Applied

1. **Zero-alloc identifiers** — `identifier()` returns `&str` slice, `.to_string()` only at AST boundary
2. **First-char dispatch** — `PrimaryStrategy` matches first byte to jump directly to the right parser, avoiding 8 failed checkpoint/backtrack cycles for identifiers
3. **Borrowed join sources** — `JoinParser` accepts `&FromSource`, clones only on successful match
4. **Word-boundary `kw()`** — prevents `OR` matching `ORDER`, `IN` matching `INSERT`
5. **`<<`/`>>` boundary** — comparison operators `<`/`>` don't match bag delimiters

## Methodology

Benchmark runs Criterion with 100 samples, 3s warmup per query. Both parsers produce the same `partiql-ast` AST types, verified by 16 parity tests comparing structural properties.

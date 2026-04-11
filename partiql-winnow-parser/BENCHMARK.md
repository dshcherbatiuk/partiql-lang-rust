# Winnow Parser Benchmark Report

## Date: 2026-04-11 (v6 — parser feature coverage)

## Environment
- Apple Silicon (aarch64-apple-darwin)
- Rust 1.88.0, release profile
- Criterion 0.5, 100 samples per benchmark
- Bench file: `fde/fde-core/benches/parser_benchmark.rs`

## SELECT Queries — LALRPOP vs Winnow (all versions)

| Query | LALRPOP | v2 (zero-alloc) | v3 (byte-level) | v4 (Pratt) | v5 (current) | Speedup |
|-------|---------|-----------------|-----------------|------------|--------------|---------|
| `SELECT * FROM users` | 863ns | 718ns | 364ns | 263ns | **254ns** | **3.4x** |
| `SELECT a, b, c FROM users WHERE a = 1` | 2.51µs | 2.03µs | 1.21µs | 729ns | **656ns** | **3.8x** |
| `SELECT u.email FROM users u WHERE ...` | 2.28µs | 1.72µs | 1.01µs | 758ns | **729ns** | **3.1x** |
| `SELECT * FROM "fde.users" WHERE ...` | 1.55µs | 1.15µs | 657ns | 421ns | **419ns** | **3.7x** |
| `SELECT ... FROM "fde.users" u, u.platformData p WHERE ...` | 4.27µs | 4.04µs | 1.96µs | 1.46µs | **1.46µs** | **2.9x** |
| `SELECT * FROM users WHERE email IN ('a', 'b', 'c')` | 2.60µs | 1.71µs | 1.09µs | 638ns | **618ns** | **4.2x** |
| `SELECT * FROM users WHERE email IN ['a', 'b', 'c']` | — | — | — | 629ns | **605ns** | — |
| `SELECT (15 cols) ... WHERE ... AND ...` | 10.9µs | 9.15µs | 5.46µs | 3.96µs | **3.72µs** | **2.9x** |
| `SELECT COUNT(*) FROM users WHERE active = true` | 2.08µs | 1.54µs | 882ns | 545ns | **534ns** | **3.9x** |
| `SELECT ... GROUP BY ... HAVING ... ORDER BY ... LIMIT` | 3.88µs | 2.94µs | 1.69µs | 967ns | **966ns** | **4.0x** |
| `SELECT ... WHERE u.email IN [...] AND p.originalPlatformId = '...'` | — | — | — | 1.31µs | **1.26µs** | — |

**Average speedup vs LALRPOP: 3.5x**

## v6 — Parser feature coverage (SELECT)

These benches pin the parser features added in v6 — each one was a
gap that broke real downstream tests until the matching commit landed
on `feature/winnow-parser`. They double as a perf budget for the new
code paths and a regression smoke test (criterion fails if the parser
stops accepting them).

| Bench | Median time | Pinned feature |
|---|---:|---|
| `alias_inference_path_projection` | **1.04 µs** | `u.id`, `u.name` round-trip with `VarRef`-shaped path steps so the column-name aliases survive lowering. |
| `path_unpivot_star` | **938 ns** | `SELECT DISTINCT u.*` — `PathStep::PathUnpivot`. |
| `path_for_each_bracket_star` | **431 ns** | `a[*]` — `PathStep::PathForEach`. |
| `backtick_ion_literal_in_function` | **494 ns** | `UNIX_TIMESTAMP(\`2020T\`)` — backtick-delimited Ion literal as expression. |
| `backtick_ion_literal_with_offset` | **479 ns** | `UNIX_TIMESTAMP(\`2024-01-01T10:00:00.500Z\`)` — full ISO 8601 inside backticks. |
| `bare_ion_timestamp_compare` | **482 ns** | `WHERE created_at > 2024-01-01T00:00:00Z` — bare Ion timestamp wired into `PrimaryStrategy`. |
| `negative_int_literal_compare` | **446 ns** | `WHERE version > -1` — `UniOp(Neg, Int64Lit(1))` parser shape pinned for downstream consumers that bypass the evaluator. |

## DML Queries — Winnow (all versions)

| Query | v3 (byte-level) | v4 (Pratt) | v5 (current) | Improvement |
|-------|-----------------|------------|--------------|-------------|
| `INSERT INTO users <<{...}>>` | 1.18µs | 440ns | **291ns** | **4.1x** |
| `INSERT INTO "fde.users" <<{...}>>` | 1.44µs | 519ns | **370ns** | **3.9x** |
| `INSERT INTO "fde.users" <<{... nested ...}>>` | 2.35µs | 790ns | **619ns** | **3.8x** |
| `INSERT INTO users <<{...}, {...}, {...}>>` | 1.73µs | 649ns | **476ns** | **3.6x** |
| `UPSERT INTO "fde.users" <<{...}>>` | 1.17µs | 445ns | **292ns** | **4.0x** |
| `UPSERT INTO "fde.users" <<{... nested ...}>>` | 2.41µs | 812ns | **617ns** | **3.9x** |
| `REPLACE INTO "fde.users" <<{... nested ...}>>` | 2.08µs | 691ns | **529ns** | **3.9x** |
| `DELETE FROM "fde.users" WHERE email = 'test@co'` | 542ns | 317ns | **166ns** | **3.3x** |

**v5 DML average improvement vs v3: 3.8x**

## v6 — Parser feature coverage (DML)

| Bench | Median time | Pinned feature |
|---|---:|---|
| `insert_negative_int` | **321 ns** | `'version': -1` — `UniOp(Neg, Int64Lit(1))` shape (folded by FDE's value conversion). |
| `insert_decimal_literal` | **302 ns** | `'realValue': 8.8` — `Lit::DecimalLit` produced by the unsigned-decimal grammar branch. |
| `insert_bare_ion_timestamp` | **348 ns** | `'start': 2024-01-01T10:00:00Z` — bare Ion timestamp inside DML, dispatched ahead of the numeric literal strategy. |
| `insert_ion_blob_literal` | **325 ns** | `'payload': {{dGVzdCBkYXRh}}` — Ion blob literal `{{ base64 }}` in `literal/ion/blob.rs`. |
| `delete_aliased_path_where` | **250 ns** | `DELETE FROM "msteams.users" u WHERE u.id = '1'` — exercises the path-aware field reference shape that downstream WHERE-extraction depends on. |

## ON CONFLICT Queries — Winnow

| Query | v4 (Pratt) | v5 (current) |
|-------|------------|--------------|
| `ON CONFLICT DO NOTHING` | 21ns | **33ns** |
| `ON CONFLICT DO REPLACE EXCLUDED` | 35ns | **47ns** |
| `ON CONFLICT DO UPDATE EXCLUDED WHERE email = '...'` | 157ns | **176ns** |
| `ON CONFLICT DO UPDATE SET name = '...', age = 30` | 224ns | **266ns** |
| `ON CONFLICT DO UPDATE SET ... = EXCLUDED..., ... = array_union(...)` (3 clauses + merge fn) | 796ns | **832ns** |

Slight regression in ON CONFLICT (3-15%) — `OnConflictParser::new()` allocation now happens per-call due to refactor; `&PrattParser` no longer reused across iterations in the bench.

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
13. **ParsedDql** — single-pass parse returns AST + WHERE + table names + unnest aliases
14. **LogicalPlanner::lower()** accepts `&AstNode<TopLevelQuery>` directly (no `Parsed` wrapper)
15. **partiql-parser removed** — FDE depends only on winnow parser end-to-end
16. **IN [...] bracket syntax** — IN parser accepts any expression, not just parenthesized lists

### Phase 5 (single-pass DML, Ion-native)
17. **`DmlQueryParser::default()`** — owns `ExprChain`, no lifetime parameter
18. **`WinnowParser::parse()` single entry point** — SELECT first, then DML with backtracking
19. **`ParsedDml::table_name()`** — convenient accessor avoids match arms
20. **DML handlers single-pass** — convert AST→Ion bytes inline, no intermediate Vec<Value>
21. **AST→Ion direct path** — INSERT/REPLACE bypass PartiQL Value entirely on write

### Phase 6 (parser feature coverage)
22. **VarRef-shaped path projections** — `.field` produces `PathProject(VarRef("field"))` instead of `Lit::CharStringLit`, so `name_resolver::infer_alias` can recover the projection alias.
23. **PathUnpivot `.*` and PathForEach `[*]`** — added to the postfix loop in `try_postfix`.
24. **Backtick-delimited Ion literal** — `` `2020T` ``, ` ```a`b``` `, etc., parsed in `PrimaryStrategy::parse_primary` to `Lit::EmbeddedDocLit` (matching LALRPOP).
25. **Bare Ion timestamp in expression position** — `2024-01-01T10:00:00Z` dispatched before the numeric strategy via a cheap `looks_like_ion_timestamp` structural check; `ion_timestamp` helper is now wired.
26. **Ion blob literal `{{ base64 }}`** — new `literal/ion/blob.rs` module, dispatched on `b'{'` + peek of next byte for `b'{'`. Whitespace inside the delimiters is permitted and stripped before producing `Lit::TypedLit(payload, BlobType)`.

## Benchmark History

| Version | Avg Speedup vs LALRPOP | Key Change |
|---------|----------------------|------------|
| v1 (initial) | 1.2-1.5x | Strategy + Chain of Responsibility pattern |
| v2 (zero-alloc) | 1.8-2.1x | &str identifiers, first-char dispatch, borrowed joins |
| v3 (byte-level) | 2.0-2.4x | Byte-level kw(), Cell IDs, inline ASCII whitespace |
| v4 (Pratt + FDE) | 2.8-4.1x | Pratt parser, ParsedDql, partiql-parser removed |
| v5 (Ion-native) | **2.9-4.2x** | Single-pass DML, AST→Ion direct, DmlQueryParser::default |
| v6 (feature coverage) | **2.9-4.2x** | PathUnpivot/ForEach, backtick + bare Ion timestamps, Ion blob literal, VarRef-shaped path projections |

## Methodology

Benchmark runs Criterion with 100 samples, 3s warmup per query. Both parsers produce the same `partiql-ast` AST types, verified by 17 parity tests comparing structural properties.

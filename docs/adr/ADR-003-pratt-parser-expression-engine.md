# ADR-003: Pratt Parser for Expression Engine

**Status**: Proposed
**Date**: 2026-04-08
**Branch**: `feature/winnow-parser`

## Context

The winnow parser currently uses a **recursive chain of 9 ExprStrategy levels** for expression parsing. Each level is a separate trait object with virtual dispatch.

### Current Architecture — Recursive Chain

```mermaid
graph TD
    A[ExprChain.parse_expr] --> B[OrStrategy]
    B -->|"calls next level"| C[AndStrategy]
    C -->|"calls next level"| D[NotStrategy]
    D -->|"calls next level"| E[ComparisonStrategy]
    E -->|"calls next level"| F[AddSubStrategy]
    F -->|"calls next level"| G[MulDivStrategy]
    G -->|"calls next level"| H[UnaryStrategy]
    H -->|"calls next level"| I[PostfixStrategy]
    I -->|"calls next level"| J[PrimaryStrategy]
    J -->|"first-char dispatch"| K[Literal / Identifier / Function]

    style A fill:#f96,stroke:#333
    style J fill:#6f9,stroke:#333
    style K fill:#6f9,stroke:#333
```

### Problem: Simple Literal Traversal

For a simple expression like `1`, the parser traverses all 9 levels doing nothing:

```mermaid
sequenceDiagram
    participant Chain as ExprChain
    participant L0 as OrStrategy
    participant L1 as AndStrategy
    participant L2 as NotStrategy
    participant L3 as ComparisonStrategy
    participant L4 as AddSubStrategy
    participant L5 as MulDivStrategy
    participant L6 as UnaryStrategy
    participant L7 as PostfixStrategy
    participant L8 as PrimaryStrategy

    Chain->>L0: parse("1")
    L0->>L1: ws0, no OR → next level
    L1->>L2: ws0, no AND → next level
    L2->>L3: ws0, no NOT → next level
    L3->>L4: ws0, no operator → next level
    L4->>L5: ws0, no +/- → next level
    L5->>L6: ws0, no */÷ → next level
    L6->>L7: ws0, no unary → next level
    L7->>L8: ws0, no ./[] → next level
    L8-->>Chain: Lit(1)

    Note over Chain,L8: 9 virtual dispatches + 9 ws0 scans for 1 token
```

### Call Count Comparison

| Expression | Chain (current) | Pratt (proposed) |
|---|---|---|
| `1` (literal) | 9 calls | 1 call |
| `'hello'` | 9 calls | 1 call |
| `a = 1` | 20 calls | 3 calls |
| `a = 1 AND b = 2` | 45 calls | 7 calls |
| `a + b * c` | 30 calls | 5 calls |
| `p.id = 'x' AND p.platform = 'MS'` | 55 calls | 9 calls |

### Estimated Impact

ExprChain recursion accounts for **12-20%** of total parse time. Eliminating it would push the parser from **2.3x** to **2.8-3.0x** faster than LALRPOP.

## Decision

Replace the recursive `ExprStrategy` chain with a **Pratt parser** (top-down operator precedence parser).

### Proposed Architecture — Pratt Parser

```mermaid
graph TD
    A[PrattParser.parse_expr] -->|"min_bp=0"| B{parse_prefix}
    B -->|"'"| C[String Literal]
    B -->|"0-9"| D[Number Literal]
    B -->|"a-z"| E[Identifier / Function]
    B -->|"("| F[Paren Expr]
    B -->|"NOT/-/+"| G[Prefix Op → recurse]
    B -->|"<<"| H[Bag Constructor]
    B -->|"["| I[List Constructor]
    B -->|"{"| J[Struct Constructor]

    A --> K{infix loop}
    K -->|"peek operator"| L{l_bp >= min_bp?}
    L -->|"yes"| M[consume op + recurse with r_bp]
    M --> K
    L -->|"no"| N[return lhs]

    style A fill:#6f9,stroke:#333
    style K fill:#ff9,stroke:#333
    style N fill:#6f9,stroke:#333
```

### Pratt Parser Flow for `a = 1 AND b = 2`

```mermaid
sequenceDiagram
    participant PP as PrattParser

    Note over PP: parse_expr(min_bp=0)
    PP->>PP: parse_prefix → VarRef("a")
    PP->>PP: peek "=" (l_bp=5 >= 0) ✓
    PP->>PP: consume "=", recurse(min_bp=6)
    Note over PP: parse_expr(min_bp=6)
    PP->>PP: parse_prefix → Lit(1)
    PP->>PP: peek "AND" (l_bp=3 < 6) ✗ return
    PP->>PP: build BinOp(Eq, a, 1)
    PP->>PP: peek "AND" (l_bp=3 >= 0) ✓
    PP->>PP: consume "AND", recurse(min_bp=4)
    Note over PP: parse_expr(min_bp=4)
    PP->>PP: parse_prefix → VarRef("b")
    PP->>PP: peek "=" (l_bp=5 >= 4) ✓
    PP->>PP: consume "=", recurse(min_bp=6)
    PP->>PP: parse_prefix → Lit(2)
    PP->>PP: peek EOF → return Lit(2)
    PP->>PP: build BinOp(Eq, b, 2)
    PP->>PP: peek EOF → return
    PP->>PP: build BinOp(And, a=1, b=2)

    Note over PP: 7 function calls (was ~45)
```

### Binding Power Table

```mermaid
graph LR
    subgraph "Binding Power (low → high)"
        OR["OR<br/>bp(1,2)"] --> AND["AND<br/>bp(3,4)"]
        AND --> CMP["= != < ><br/>IS IN LIKE BETWEEN<br/>bp(5,6)"]
        CMP --> ADD["+ - ||<br/>bp(7,8)"]
        ADD --> MUL["* / %<br/>bp(9,10)"]
        MUL --> PREFIX["NOT - +<br/>(prefix)<br/>bp(_,11)"]
        PREFIX --> POSTFIX[". [] ()<br/>(postfix)<br/>bp(13,_)"]
    end

    style OR fill:#fdd
    style AND fill:#fed
    style CMP fill:#ffd
    style ADD fill:#dfd
    style MUL fill:#dff
    style PREFIX fill:#ddf
    style POSTFIX fill:#fdf
```

### Migration Plan

```mermaid
graph LR
    S1["1. Create PrattParser<br/>in expr/pratt.rs"] --> S2["2. Reuse PrimaryStrategy<br/>+ ComparisonParsers"]
    S2 --> S3["3. Wire into SelectParser<br/>replace ExprChain"]
    S3 --> S4["4. Run parity tests<br/>276 unit + 16 parity"]
    S4 --> S5["5. Benchmark<br/>target 2.8-3.0x"]
    S5 --> S6["6. Remove old chain<br/>8 strategy files"]

    style S1 fill:#dfd
    style S5 fill:#ff9
    style S6 fill:#fdd
```

### What We Keep vs Remove

```mermaid
graph TD
    subgraph "Keep ✓"
        A1[PrimaryStrategy<br/>literals + identifiers]
        A2[ComparisonParser trait<br/>IS/IN/LIKE/BETWEEN]
        A3[LiteralStrategy trait<br/>string/number/null/bool/bag/list/struct/case]
        A4[JoinParser trait<br/>comma/cross/inner/left/right/full]
        A5[ClauseParser trait<br/>projection/from/where/group_by/having/order_by/limit]
        A6[DmlStrategy trait<br/>insert/replace/upsert/update/delete]
    end

    subgraph "Remove ✗"
        B1[ExprStrategy trait]
        B2[ExprChain struct]
        B3[StrategyContext struct]
        B4[OrStrategy]
        B5[AndStrategy]
        B6[NotStrategy]
        B7[ComparisonStrategy]
        B8[AddSubStrategy]
        B9[MulDivStrategy]
        B10[UnaryStrategy]
        B11[PostfixStrategy]
    end

    style A1 fill:#dfd
    style A2 fill:#dfd
    style A3 fill:#dfd
    style A4 fill:#dfd
    style A5 fill:#dfd
    style A6 fill:#dfd
    style B1 fill:#fdd
    style B2 fill:#fdd
    style B3 fill:#fdd
    style B4 fill:#fdd
    style B5 fill:#fdd
    style B6 fill:#fdd
    style B7 fill:#fdd
    style B8 fill:#fdd
    style B9 fill:#fdd
    style B10 fill:#fdd
    style B11 fill:#fdd
```

## Consequences

### Positive
- **30-40% fewer function calls** for typical expressions
- **No virtual dispatch** for operator precedence
- **Single loop** instead of 9-level recursion
- **Simpler code** — one file instead of 8 strategy files
- Enables future optimizations (token lookahead, operator fusion)

### Negative
- Larger single function (Pratt parser is ~150 lines vs 8 small files)
- Less "strategy pattern" — but the pattern was causing the performance problem
- Requires careful testing to ensure precedence/associativity matches LALRPOP parser

### Risks
- Precedence bugs — mitigated by 16 parity tests + 260 unit tests
- Special forms (IS/IN/LIKE/BETWEEN) need careful integration into the Pratt loop
- Postfix `.`/`[]` chaining must handle arbitrary depth

## Key Files

| File | Change |
|------|--------|
| `partiql-winnow-parser/src/expr/pratt.rs` | New: Pratt parser implementation |
| `partiql-winnow-parser/src/expr/mod.rs` | Replace `ExprChain` with `PrattParser` |
| `partiql-winnow-parser/src/expr/primary_strategy.rs` | Keep: prefix parsing (literals, identifiers, functions) |
| `partiql-winnow-parser/src/expr/comparison/` | Keep: IS/IN/LIKE/BETWEEN special forms |
| `partiql-winnow-parser/src/select/select_parser.rs` | Wire `PrattParser` instead of `ExprChain` |
| `partiql-winnow-parser/src/expr/or_strategy.rs` | Remove |
| `partiql-winnow-parser/src/expr/and_strategy.rs` | Remove |
| `partiql-winnow-parser/src/expr/not_strategy.rs` | Remove |
| `partiql-winnow-parser/src/expr/comparison_strategy.rs` | Remove (operators move to Pratt loop) |
| `partiql-winnow-parser/src/expr/add_sub_strategy.rs` | Remove |
| `partiql-winnow-parser/src/expr/mul_div_strategy.rs` | Remove |
| `partiql-winnow-parser/src/expr/unary_strategy.rs` | Remove |
| `partiql-winnow-parser/src/expr/postfix_strategy.rs` | Remove |

## References

- [Pratt Parsing Made Easy](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html) — matklad (rust-analyzer author)
- [Simple but Powerful Pratt Parsing](https://journal.stuffwithstuff.com/2011/03/19/pratt-parsers-expression-parsing-made-easy/) — Bob Nystrom (Crafting Interpreters)

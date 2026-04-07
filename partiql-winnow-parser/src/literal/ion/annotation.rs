//! Ion annotation parsing.
//!
//! Any Ion value can be prefixed with one or more annotations:
//! ```text
//! annotated_value ::= (symbol '::')* value
//! annotation      ::= unquoted_symbol '::' | quoted_symbol '::' | symbol_id '::'
//! symbol_id       ::= '$' [0-9]+
//! ```
//!
//! Annotations are symbols — unquoted identifiers, single-quoted `'...'`,
//! or symbol IDs `$0`, `$10`. The `::` separator connects annotation to value.
//!
//! Examples: `dollars::100`, `a::b::42`, `'custom type'::null`

// TODO: implement annotation parsing in Step 2 (depends on expression/value parser)

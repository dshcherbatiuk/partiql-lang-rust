//! PartiQL literal value parsing — strings, numbers, booleans, null, Ion.
//!
//! PartiQL literals follow SQL conventions with Ion extensions:
//!
//! | Type | Examples |
//! |------|---------|
//! | String | `'hello'`, `'it''s'` (escaped single quote) |
//! | Integer | `42`, `-1`, `0` |
//! | Decimal | `3.14`, `-0.5` |
//! | Boolean | `TRUE`, `FALSE`, `true`, `false` |
//! | Null | `NULL`, `null` |
//! | Missing | `MISSING` |
//! | Ion timestamp | `2024-01-01T00:00:00Z` |
//!
//! # BNF
//! ```text
//! literal          ::= string_literal | numeric_literal | boolean_literal
//!                     | null_literal | missing_literal | ion_timestamp
//! string_literal   ::= "'" ([^'] | "''")* "'"
//! numeric_literal  ::= integer_literal | decimal_literal
//! integer_literal  ::= ['-']? [0-9]+
//! decimal_literal  ::= ['-']? [0-9]+ '.' [0-9]+
//! boolean_literal  ::= TRUE | FALSE
//! null_literal     ::= NULL
//! missing_literal  ::= MISSING
//! ```

// TODO: implement literal parsers in Step 1 continued

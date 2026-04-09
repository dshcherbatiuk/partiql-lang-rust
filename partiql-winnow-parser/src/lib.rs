#![deny(rust_2018_idioms)]
#![deny(clippy::all)]

//! winnow-based PartiQL parser — drop-in replacement for partiql-parser.
//!
//! Each BNF rule from the PartiQL spec maps to one Rust function.
//! Strategy pattern for expressions, SELECT clauses, and DML statements.

pub mod dml;
pub mod dql;
pub mod expr;
mod identifier;
mod keyword;
mod literal;
pub mod parse_context;
pub mod parsed_dql;
pub mod parser;
mod whitespace;

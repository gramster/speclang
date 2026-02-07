//! Parser and pretty-printer for Core IR textual form.
//!
//! Parses the canonical textual representation defined in ir-grammar.md
//! and can round-trip Core IR ASTs.

pub mod lexer;
pub mod parser;
pub mod printer;

pub use parser::parse_module;
pub use printer::print_module;

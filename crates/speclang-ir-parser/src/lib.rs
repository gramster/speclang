//! Parser and pretty-printer for Core IR textual form.
//!
//! This crate handles the canonical text serialization of Core IR as
//! defined in [`docs/ir-grammar.md`].  It supports full round-tripping:
//!
//! ```ignore
//! use speclang_ir_parser::{parse_module, print_module};
//!
//! let module = parse_module(source_text)?;
//! let text   = print_module(&module);
//! assert_eq!(parse_module(&text)?, module);
//! ```
//!
//! # Crate layout
//!
//! - [`lexer`] — tokenizer for the IR text format
//! - [`parser`] — recursive-descent parser → [`speclang_ir::Module`]
//! - [`printer`] — pretty-printer [`speclang_ir::Module`] → text

pub mod lexer;
pub mod parser;
pub mod printer;

pub use parser::parse_module;
pub use printer::print_module;

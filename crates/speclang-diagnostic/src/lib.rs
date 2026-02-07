//! Structured diagnostics for the speclang compiler.
//!
//! Provides a unified [`Diagnostic`] type that can represent errors,
//! warnings, and notes from any pipeline stage.  Each diagnostic carries
//! a [`Severity`], a message, and zero or more [`Label`]s that attach
//! source spans with annotations.
//!
//! The [`render_diagnostics()`] function formats diagnostics in a
//! `rustc`-style presentation with source snippets, line numbers, and
//! caret/underline highlighting:
//!
//! ```text
//! error: type mismatch
//!   --> src/main.spl:12:5
//!    |
//! 12 |     x + "hello"
//!    |         ^^^^^^^ expected Int, found String
//! ```
//!
//! Feed it a [`SourceFile`] (built from file contents) and a list of
//! diagnostics to get formatted output.

mod diagnostic;
mod render;
mod source;

pub use diagnostic::{Diagnostic, Label, Severity};
pub use render::render_diagnostics;
pub use source::SourceFile;

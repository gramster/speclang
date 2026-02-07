//! Structured diagnostics for the speclang compiler.
//!
//! Provides a unified `Diagnostic` type that can represent errors, warnings,
//! and notes from any pipeline stage, with optional source locations.
//! Includes a renderer that produces human-readable output with source
//! snippets, line numbers, and caret/underline highlighting.

mod diagnostic;
mod render;
mod source;

pub use diagnostic::{Diagnostic, Label, Severity};
pub use render::render_diagnostics;
pub use source::SourceFile;

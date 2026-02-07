//! SPL and IMPL source code formatter and pretty-printer.
//!
//! Provides canonical formatting for both SPL spec files and IMPL
//! implementation files.  The formatters parse source text, rebuild it
//! with consistent indentation and spacing, and emit the result.
//!
//! # Usage
//!
//! ```ignore
//! use speclang_fmt::{format_spl, format_impl};
//!
//! let formatted = format_spl(source_text)?;
//! let formatted = format_impl(source_text)?;
//! ```
//!
//! Both functions return the formatted source as a `String`.  They are
//! idempotent: formatting already-formatted code produces identical output.

mod spl_fmt;
mod impl_fmt;

pub use spl_fmt::format_spl;
pub use impl_fmt::format_impl;

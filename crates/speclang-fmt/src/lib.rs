//! SPL and IMPL source code formatter and pretty-printer.
//!
//! Provides canonical formatting for both SPL spec files and IMPL
//! implementation files. Parse → format → emit produces consistently
//! formatted source code.

mod spl_fmt;
mod impl_fmt;

pub use spl_fmt::format_spl;
pub use impl_fmt::format_impl;

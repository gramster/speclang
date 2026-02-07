//! Standard library modules for the speclang compiler.
//!
//! Each submodule provides a `module()` function that returns a
//! [`speclang_ir::Module`] containing the type definitions and extern
//! function declarations for that part of the standard library.
//!
//! # Modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`core`] | `Option`, `Result`, equality, ordering |
//! | [`math`] | Arithmetic, trigonometry, constants |
//! | [`mem`] | Region allocation, pointer operations |
//! | [`bytes`] | Raw byte buffer operations |
//! | [`text`] | UTF-8 string operations |
//! | [`collections`] | `Vec`, `Set`, `Map` |
//! | [`contracts`] | Pure helpers for contract lowering |
//!
//! Use [`all_stdlib_modules()`] to get every module at once, or
//! [`find_stdlib_module()`] to look up a single module by name.

pub mod core;
pub mod math;
pub mod mem;
pub mod bytes;
pub mod text;
pub mod collections;
pub mod contracts;

use speclang_ir::Module;

/// Return all standard library modules.
pub fn all_stdlib_modules() -> Vec<Module> {
    vec![
        core::module(),
        math::module(),
        mem::module(),
        bytes::module(),
        text::module(),
        collections::module(),
        contracts::module(),
    ]
}

/// Look up a standard library module by qualified name.
pub fn find_stdlib_module(name: &[String]) -> Option<Module> {
    all_stdlib_modules()
        .into_iter()
        .find(|m| m.name == name)
}

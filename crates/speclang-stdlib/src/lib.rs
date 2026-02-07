//! Standard library modules for the speclang compiler.
//!
//! Each submodule provides a function that returns a `Module` containing
//! the type definitions and extern function declarations for that part
//! of the standard library. These modules are referenced during SPL
//! name resolution, type checking, and lowering.

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

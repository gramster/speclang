//! Rust transpiler backend for Core IR.
//!
//! Generates idiomatic Rust source code from Core IR modules, mapping:
//! - Core IR types → Rust types (struct, enum, tuple, primitives)
//! - Ownership/regions → Rust `Box`, `&`, `&mut`, `&[T]`, `&mut [T]`
//! - Capabilities → Rust trait-based capability tokens
//! - Functions/contracts → Rust functions with `debug_assert!` guards
//! - Control flow → Rust `if`, `match`, `let`, etc.

pub mod codegen;

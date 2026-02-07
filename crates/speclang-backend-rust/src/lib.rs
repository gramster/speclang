//! Rust transpiler backend for Core IR.
//!
//! Generates idiomatic Rust source code from a [`speclang_ir::Module`].
//!
//! # Type mapping
//!
//! | Core IR | Rust |
//! |---------|------|
//! | `i32`, `u64`, … | `i32`, `u64`, … |
//! | `own[T]` | `Box<T>` |
//! | `ref[T]` / `mutref[T]` | `&T` / `&mut T` |
//! | `slice[T]` / `mutslice[T]` | `&[T]` / `&mut [T]` |
//! | `struct { … }` | `struct` with named fields |
//! | `enum { … }` | `enum` with variants |
//! | capabilities | Zero-size marker types |
//!
//! Contracts are emitted as `debug_assert!` guards.  The generated code
//! compiles with `rustc` and preserves the ownership semantics of Core IR.
//!
//! # Usage
//!
//! ```ignore
//! use speclang_backend_rust::codegen::RustCodeGen;
//!
//! let rust_source = RustCodeGen::new().generate(&module);
//! ```

pub mod codegen;

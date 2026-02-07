//! WASM/WASI backend for Core IR.
//!
//! Generates WebAssembly Text format (WAT) from Core IR modules, with
//! WASI preview-1 support for I/O operations. The generated WAT can be
//! assembled to `.wasm` using any standard tool (e.g., `wat2wasm`).
//!
//! ## Architecture
//!
//! The backend maps Core IR constructs to WASM as follows:
//!
//! - **Primitives**: `i32`, `i64`, `f32`, `f64` map directly to WASM value types.
//!   `Bool` and smaller integers become `i32`. `String` and `Bytes` use linear
//!   memory with `(i32, i32)` pointer+length pairs.
//!
//! - **Structs**: Flattened into linear memory. Struct values are represented as
//!   `i32` pointers into linear memory.
//!
//! - **Enums**: Tagged unions in linear memory. Discriminant tag + payload.
//!
//! - **Functions**: Each Core IR function becomes a WASM function. Public
//!   functions are exported.
//!
//! - **Capabilities**: Represented as empty tokens (zero-size). Capability
//!   parameters are elided from WASM function signatures.
//!
//! - **Contracts**: `requires` contracts become `unreachable`-trap guards in
//!   debug mode. `ensures` are comments only.
//!
//! - **WASI**: Import `fd_write` for console output, `proc_exit` for exits.

pub mod codegen;

pub use codegen::generate_wasm;

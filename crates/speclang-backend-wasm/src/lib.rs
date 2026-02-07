//! WASM/WASI backend for Core IR.
//!
//! Generates WebAssembly Text format (WAT) from a [`speclang_ir::Module`],
//! with WASI preview-1 support for I/O operations.  The generated WAT can
//! be assembled to `.wasm` using any standard tool (e.g., `wat2wasm`).
//!
//! # Usage
//!
//! ```ignore
//! use speclang_backend_wasm::generate_wasm;
//!
//! let wat_source = generate_wasm(&module);
//! std::fs::write("output.wat", &wat_source)?;
//! // Then: wat2wasm output.wat -o output.wasm
//! //       wasmtime output.wasm
//! ```
//!
//! # Architecture
//!
//! The backend maps Core IR constructs to WASM as follows:
//!
//! | Core IR | WASM representation |
//! |---------|--------------------|
//! | `i32`, `u32`, `Bool` | `i32` |
//! | `i64`, `u64` | `i64` |
//! | `f32` | `f32` |
//! | `f64` | `f64` |
//! | `String`, `Bytes` | `(i32, i32)` pointer+length in linear memory |
//! | `struct` | Flattened into linear memory; passed as `i32` pointer |
//! | `enum` | Tagged union in linear memory (discriminant + payload) |
//! | Capabilities | Zero-size; elided from function signatures |
//! | `requires` contracts | `unreachable`-trap guards |
//! | `ensures` contracts | Comments only |
//!
//! WASI preview-1 imports: `fd_write` for console output, `proc_exit`
//! for process termination.

pub mod codegen;

pub use codegen::generate_wasm;

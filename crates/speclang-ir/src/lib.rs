//! Core IR types and AST for the speclang compiler.
//!
//! This crate defines the intermediate representation that:
//! - SPL (spec layer) lowers into
//! - Backends (LLVM/MLIR, Rust/Zig/C transpilers, WASM/WASI) consume
//! - Tooling uses to enforce contracts, capabilities, and refactor-stable IDs

pub mod types;
pub mod expr;
pub mod module;
pub mod capability;
pub mod contract;

// Re-export key types at crate root
pub use module::{Module, Function, ExternFunction};
pub use types::{Type, PrimitiveType, Region};
pub use expr::{Expr, Stmt, Block, Pattern, MatchArm};
pub use capability::{CapabilityDef, CapabilityType};
pub use contract::{Contract, ContractPolicy};

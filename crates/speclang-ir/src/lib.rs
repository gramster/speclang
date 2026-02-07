//! Core IR types and AST for the speclang compiler.
//!
//! This crate defines the intermediate representation that sits at the
//! center of the speclang compilation pipeline.  Every frontend (SPL,
//! IMPL) lowers into Core IR, and every backend (Rust transpiler, WASM)
//! consumes it.
//!
//! # Key types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Module`] | Top-level compilation unit — types, functions, externs, capabilities |
//! | [`Function`] | Named function with params, body, contracts |
//! | [`Type`] | Full type algebra — primitives, aggregates, references, regions |
//! | [`Expr`] / [`Stmt`] | Expression and statement trees |
//! | [`Contract`] | Pre/post-conditions and invariants |
//! | [`CapabilityDef`] | First-class capability (effect) declarations |
//!
//! # Design goals
//!
//! 1. **Unambiguous semantics** — every allocation, copy, and effect is visible.
//! 2. **Small surface** — easy to implement new backends and verification passes.
//! 3. **Explicit cost model** — no hidden heap allocations or implicit conversions.
//!
//! # Crate layout
//!
//! - [`types`] — [`Type`], [`PrimitiveType`], [`Region`]
//! - [`expr`] — [`Expr`], [`Stmt`], [`Block`], [`Pattern`], [`MatchArm`]
//! - [`module`] — [`Module`], [`Function`], [`ExternFunction`]
//! - [`capability`] — [`CapabilityDef`], [`CapabilityType`]
//! - [`contract`] — [`Contract`], [`ContractPolicy`]

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

//! Core IR verifier.
//!
//! Enforces:
//! - Type safety (well-formed types, expressions match expected types)
//! - Duplicate name detection
//! - Function body return type consistency
//! - Named type resolution within the module
//!
//! Future phases (stubs):
//! - Ownership and borrowing rules (Rust-like) → `ownership`
//! - Capability threading (no hidden I/O) → `capabilities`
//! - Exhaustive pattern matching → `exhaustiveness`

pub mod capabilities;
pub mod exhaustiveness;
pub mod ownership;
pub mod regions;
pub mod typecheck;

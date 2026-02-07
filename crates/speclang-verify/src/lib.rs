//! Core IR verifier.
//!
//! Enforces:
//! - Type safety (well-formed types, expressions match expected types)
//! - Duplicate name detection
//! - Function body return type consistency
//! - Named type resolution within the module
//!
//! Future phases (stubs):
//! - Ownership and borrowing rules (Rust-like)
//! - Region lifetime constraints
//! - Capability threading (no hidden I/O)
//! - Exhaustive pattern matching

pub mod typecheck;

//! Core IR verifier.
//!
//! Enforces:
//! - Type safety
//! - Ownership and borrowing rules (Rust-like)
//! - Region lifetime constraints
//! - Capability threading (no hidden I/O)
//! - Exhaustive pattern matching

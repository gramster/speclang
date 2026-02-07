//! Rust transpiler backend for Core IR.
//!
//! Generates Rust source code from Core IR, mapping:
//! - Regions/allocations → Rust allocator APIs (bumpalo/arena)
//! - Capability tokens → Rust types threaded through call graph
//! - Ownership/borrowing → Rust ownership model

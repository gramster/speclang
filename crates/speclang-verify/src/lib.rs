//! Core IR verifier.
//!
//! A suite of analysis and verification passes that run over a
//! [`speclang_ir::Module`] after lowering.  Each pass is independent
//! and reports diagnostics on failure.
//!
//! # Passes
//!
//! | Module | What it checks |
//! |--------|----------------|
//! | [`typecheck`] | Well-formed types, expression types, return type consistency, duplicate names |
//! | [`contract_pass`] | Contract well-formedness and placement |
//! | [`capabilities`] | Effect containment — functions only use declared capabilities |
//! | [`ownership`] | Ownership and borrowing rules (move/borrow/drop) |
//! | [`exhaustiveness`] | Pattern match exhaustiveness |
//! | [`regions`] | Region lifetime consistency |
//! | [`proptest`] | Property-based test harness generation |
//! | [`fuzz`] | Fuzz target generation from specs |

pub mod capabilities;
pub mod contract_pass;
pub mod exhaustiveness;
pub mod fuzz;
pub mod ownership;
pub mod proptest;
pub mod regions;
pub mod typecheck;

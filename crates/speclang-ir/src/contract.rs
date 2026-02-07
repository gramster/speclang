//! Contract definitions and metadata for the Core IR.
//!
//! Contracts arrive from SPL as metadata and/or explicit assertions.
//! They can be compiled under different policies (always, debug, sampled).

use crate::expr::Expr;

/// Contract policy: when to insert runtime checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContractPolicy {
    /// Always check (never removed).
    Always,
    /// Check in debug builds only.
    Debug,
    /// Sample-based checking with the given probability (0.0..1.0 as fixed-point).
    Sampled(u32), // probability as parts-per-million
}

impl Default for ContractPolicy {
    fn default() -> Self {
        ContractPolicy::Debug
    }
}

/// A contract (precondition or postcondition) attached to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct Contract {
    /// The contract kind.
    pub kind: ContractKind,
    /// The predicate expression (must be pure boolean).
    pub predicate: Expr,
    /// Compilation policy for this contract.
    pub policy: ContractPolicy,
    /// Requirement traceability tags (e.g., `["REQ-001", "REQ-002"]`).
    /// These trace back to SPL `req` declarations and appear in coverage reports.
    pub req_tags: Vec<String>,
}

/// The kind of contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContractKind {
    /// Precondition: must hold at function entry.
    Requires,
    /// Postcondition: must hold at function exit.
    /// The special variable `result` refers to the return value.
    Ensures,
    /// Type/struct invariant.
    Invariant,
}

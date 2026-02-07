//! Capability (effect) definitions for the Core IR.
//!
//! Capabilities are opaque nominal types. Functions are effectful if and only
//! if they take capability parameters. No ambient I/O; no hidden global
//! capability.

use crate::types::{Ident, Type};

/// A capability field (payload parameter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityField {
    pub name: Ident,
    pub ty: Type,
}

/// A capability definition.
///
/// Capabilities are opaque tokens declared in a module:
/// - `cap Net(host: Host)`
/// - `cap FileRead(path: Path)`
/// - `cap Clock`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDef {
    /// Capability name (e.g., `Net`, `FileRead`, `Clock`).
    pub name: Ident,
    /// Payload fields (may be empty, e.g., `Clock` has none).
    pub fields: Vec<CapabilityField>,
}

/// A reference to a capability type in function signatures.
///
/// Used in effect signatures to declare which capabilities a function requires.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityType {
    /// The capability name (e.g., `Net`).
    pub name: Ident,
}

//! Module structure, function definitions, and stable symbol IDs.
//!
//! Every public function/type exposed outside the module must have a
//! stable symbol ID (carried from SPL). Backends preserve IDs for
//! diagnostics, coverage, and semantic-diff tooling.

use crate::capability::{CapabilityDef, CapabilityType};
use crate::contract::Contract;
use crate::expr::Block;
use crate::types::{Ident, QName, Type};

// ---------------------------------------------------------------------------
// Metadata annotations
// ---------------------------------------------------------------------------

/// Metadata annotation on a function or type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annotation {
    /// Stable symbol ID: `@id "music.snap.v1"`.
    Id(String),
    /// Compatibility annotation.
    Compat(Compat),
}

/// Compatibility level for stable IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Compat {
    /// Signature compatible.
    StableCall,
    /// Behavior compatible (stricter).
    StableSemantics,
    /// Allowed to change.
    Unstable,
}

// ---------------------------------------------------------------------------
// Type definitions
// ---------------------------------------------------------------------------

/// A named type definition in a module.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    /// Type name.
    pub name: Ident,
    /// The type itself.
    pub ty: Type,
    /// Metadata annotations (e.g., stable ID).
    pub annotations: Vec<Annotation>,
}

// ---------------------------------------------------------------------------
// Function parameters
// ---------------------------------------------------------------------------

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
}

// ---------------------------------------------------------------------------
// Function definitions
// ---------------------------------------------------------------------------

/// A function definition in Core IR.
///
/// Functions are either pure (no effects) or effectful (explicit capability
/// parameters). The call graph is checked: effectful functions must thread
/// capability tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Function name.
    pub name: Ident,
    /// Parameters.
    pub params: Vec<Param>,
    /// Return type.
    pub return_type: Type,
    /// Capability requirements (effect signature).
    /// Empty means pure function.
    pub effects: Vec<CapabilityType>,
    /// Contracts (preconditions, postconditions).
    pub contracts: Vec<Contract>,
    /// Function body.
    pub body: Block,
    /// Metadata annotations (e.g., stable ID, compat).
    pub annotations: Vec<Annotation>,
}

impl Function {
    /// Returns true if this function is pure (no effects).
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty()
    }

    /// Returns the stable symbol ID if one is annotated.
    pub fn stable_id(&self) -> Option<&str> {
        self.annotations.iter().find_map(|a| match a {
            Annotation::Id(id) => Some(id.as_str()),
            _ => None,
        })
    }
}

/// An extern (FFI) function declaration.
///
/// Extern functions are declared with explicit parameter/return types,
/// explicit capability requirements, and explicit ownership conventions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternFunction {
    /// Function name.
    pub name: Ident,
    /// Parameters.
    pub params: Vec<Param>,
    /// Return type.
    pub return_type: Type,
    /// Capability requirements.
    pub effects: Vec<CapabilityType>,
    /// Metadata annotations.
    pub annotations: Vec<Annotation>,
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// A Core IR compilation unit (module).
///
/// Contains type definitions, capability definitions, function definitions,
/// and optional extern (FFI) function declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Fully qualified module name.
    pub name: QName,
    /// Type definitions.
    pub type_defs: Vec<TypeDef>,
    /// Capability definitions.
    pub cap_defs: Vec<CapabilityDef>,
    /// Function definitions.
    pub functions: Vec<Function>,
    /// Extern function declarations (FFI).
    pub externs: Vec<ExternFunction>,
}

impl Module {
    /// Create an empty module with the given name.
    pub fn new(name: QName) -> Self {
        Module {
            name,
            type_defs: vec![],
            cap_defs: vec![],
            functions: vec![],
            externs: vec![],
        }
    }

    /// Find a function by name.
    pub fn find_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find a type definition by name.
    pub fn find_type(&self, name: &str) -> Option<&TypeDef> {
        self.type_defs.iter().find(|t| t.name == name)
    }

    /// Find a capability definition by name.
    pub fn find_capability(&self, name: &str) -> Option<&CapabilityDef> {
        self.cap_defs.iter().find(|c| c.name == name)
    }
}

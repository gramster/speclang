//! SPL-to-Core IR lowering.
//!
//! Lowers SPL constructs to Core IR:
//! - Type declarations → Core IR types
//! - Refine types → newtype wrappers with checked constructors
//! - Function specs → Core IR function signatures with contract metadata
//! - Examples → generated test IR functions
//! - Effects/capabilities → Core IR capability parameters

//! SPL-to-Core IR lowering.
//!
//! Translates a type-checked SPL AST (from [`speclang_spl`]) into a
//! Core IR [`speclang_ir::Module`].  The lowering is deterministic and
//! preserves all contract and capability information so that downstream
//! verification and backends have a complete picture.
//!
//! # Lowering rules
//!
//! | SPL construct | Core IR output |
//! |---------------|----------------|
//! | `type` (struct/enum/alias) | `TypeDef` |
//! | `refine` | Newtype wrapper + checked constructor function |
//! | `fn` spec | `Function` signature with [`Contract`] metadata |
//! | `examples` | Generated test functions |
//! | `effects` / `capability` | `CapabilityDef` + capability parameters |
//!
//! [`Contract`]: speclang_ir::Contract

pub mod lower;

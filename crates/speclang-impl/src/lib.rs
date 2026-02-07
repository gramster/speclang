//! IMPL (Implementation Layer) parser, verification, and lowering.
//!
//! Handles parsing of `.impl` files, binding verification against SPL specs,
//! effects checking, and lowering to Core IR.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod bind;
pub mod effects;
pub mod lower;

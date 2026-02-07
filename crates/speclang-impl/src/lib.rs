//! IMPL (Implementation Layer) parser, verification, and lowering.
//!
//! Handles the front-end pipeline for `.impl` implementation files:
//!
//! 1. **Lexing** ([`lexer`]) — tokenizes IMPL source text.
//! 2. **Parsing** ([`parser`]) — builds an IMPL AST ([`ast`]).
//! 3. **Binding** ([`bind`]) — verifies that each `impl fn` matches a
//!    corresponding SPL spec (signature, stable ID).
//! 4. **Effects checking** ([`effects`]) — confirms that effects used in
//!    the body are a subset of those declared in the SPL spec.
//! 5. **Lowering** ([`lower`]) — translates the checked IMPL AST into
//!    Core IR function bodies.
//!
//! IMPL is a minimal systems language with ownership, borrowing, regions,
//! and explicit capability-token passing.  It intentionally omits its own
//! type declarations — all types originate in SPL.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod bind;
pub mod effects;
pub mod lower;

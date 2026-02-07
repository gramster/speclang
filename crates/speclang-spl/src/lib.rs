//! SPL (Spec Layer) parser and type checker.
//!
//! Handles the full front-end pipeline for `.spl` specification files:
//!
//! 1. **Lexing** ([`lexer`]) — tokenizes SPL source text.
//! 2. **Parsing** ([`parser`]) — builds an unresolved AST ([`ast`]).
//! 3. **Name resolution** ([`resolve`]) — resolves identifiers against
//!    in-scope declarations and the standard library.
//! 4. **Type checking** ([`typecheck`]) — verifies type consistency of
//!    expressions, contracts, and examples.
//!
//! SPL is purely declarative: no loops, no mutation, no I/O.  It declares
//! types, function contracts, error taxonomies, capabilities, executable
//! examples, and algebraic properties.  The checked AST is then lowered
//! to Core IR by [`speclang_lower`].
//!
//! # Supported constructs
//!
//! `module`, `import`, `type`, `refine`, `fn`, `error`, `capability`,
//! `law`, `prop`, `examples`, `perf`, `req`, `decision`, `gen`,
//! `oracle`, `policy`.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod resolve;
pub mod typecheck;

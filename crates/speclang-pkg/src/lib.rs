//! Package manifest format and dependency resolution for speclang.
//!
//! A speclang package is defined by a `pkg.toml` file at its root:
//!
//! ```toml
//! [package]
//! name = "my-project"
//! version = "0.1.0"
//! edition = "2025"
//!
//! [dependencies]
//! std = { version = "0.1" }
//! utils = { path = "../utils" }
//!
//! [[target]]
//! name = "main"
//! spec = "src/main.spl"
//! impl = "src/main.impl"
//! ```
//!
//! # Crate layout
//!
//! - [`manifest`] — TOML parsing into [`Manifest`], [`PackageMeta`],
//!   [`Dependency`], [`Target`] types.
//! - [`resolve`] — Dependency graph construction ([`DepGraph`]) and
//!   version resolution.
//! - [`discover`] — Walk the filesystem to [`find_manifest()`].

pub mod manifest;
pub mod resolve;
pub mod discover;

pub use manifest::{Manifest, PackageMeta, Dependency, Target, DependencyKind};
pub use resolve::{ResolveError, DepGraph, ResolvedDep};
pub use discover::find_manifest;

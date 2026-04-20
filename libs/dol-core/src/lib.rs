//! # dol-core — DOL Core Language
//!
//! The foundational crate for DOL: type system, expressions, operations,
//! and the `Backend` trait. This is a true leaf crate with no dependencies
//! on schema or builder crates.
//!
//! ## Modules
//!
//! - **`types`** — `DataType` descriptors, `Value`/`Literal` data carriers
//! - **`expr`** — Composable, backend-agnostic expression AST
//! - **`op`** — Statement enum, all operation types, Backend trait, output types
//! - **`constraint`** — Constraint types shared between entity definitions and operations

#![deny(unsafe_code)]

pub mod constraint;
pub mod expr;
pub mod op;
pub mod types;

// ── Top-level convenience re-exports ──

pub use constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
pub use op::{BackendError, Statement};
pub use types::DataType;

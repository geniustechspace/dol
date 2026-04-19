//! # dol-core — DOL Core Language
//!
//! The foundational crate for DOL: type system, expressions, intermediate
//! representation, and the `Backend` trait. This is a true leaf crate with
//! no dependencies on schema or builder crates.
//!
//! ## Modules
//!
//! - **`types`** — `DataType` descriptors, `Value`/`Literal` data carriers
//! - **`expr`** — Composable, backend-agnostic expression AST
//! - **`ir`** — Statement enum, all IR types, Backend trait, output types
//! - **`constraint`** — Constraint types shared between entity definitions and IR

#![deny(unsafe_code)]

pub mod types;
pub mod expr;
pub mod ir;
pub mod constraint;

// ── Top-level convenience re-exports ──

pub use types::DataType;
pub use constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
pub use ir::{BackendError, Statement};

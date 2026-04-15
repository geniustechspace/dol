//! # dol-core — The DOL Language Layer
//!
//! This is the standalone "language" crate for DOL. It re-exports every module
//! needed to define schemas, build queries, produce IR, and define the
//! `Backend` interface — with **zero engine dependencies**.
//!
//! Custom engine authors only need `dol-core` to implement `Backend`.
//!
//! ## Internal Sub-crates
//!
//! - **`dol-expr`** — Expressions, operators, functions (leaf, no deps)
//! - **`dol-entity`** — Entity, Field, FieldType, constraints (leaf, no deps)
//! - **`dol-ir`** — Statement enum, all IR types, Backend trait, output types
//! - **`dol-builder`** — Composable builder API (GetBuilder, InsertBuilder, etc.)

#![deny(unsafe_code)]

/// Expression engine — composable, backend-agnostic expression AST.
pub use dol_expr as expr;

/// Schema language — Entity, Field, FieldType, and constraints.
pub use dol_entity as model;

/// Intermediate representation — backend-agnostic AST, Backend trait, output types.
pub use dol_ir as ir;

/// Builder API — composable method-chain builders that produce IR.
pub use dol_builder as builder;

// ── Top-level convenience re-exports ──

pub use model::{Entity, Field, FieldType};

pub use ir::{Backend, BackendError, RenderedOutput, Statement};

pub use builder::{EntityBuilderExt, EntityDefineExt};

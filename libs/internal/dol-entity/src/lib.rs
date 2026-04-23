//! DOL Entity, Field, DataType, and constraints (internal).
//!
//! This crate mirrors `dol-entity` but depends on `dol-expr` rather than
//! `dol-core`, keeping it free of the heavier operation and session types.

#![deny(unsafe_code)]

pub mod constraint;
pub mod entity;
pub mod field;

pub use constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
pub use dol_expr::types::DataType;
pub use entity::Entity;
pub use field::Field;

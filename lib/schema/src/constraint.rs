//! Re-exports of the canonical schema constraint types from
//! [`dol_core::schema`].
//!
//! The owning crate is `dol-core` so downstream IR crates (`dol-command`,
//! `dol-query`, …) can reference these primitives without depending on
//! `dol-schema`. Names are re-exported here at their historical paths.

pub use dol_core::schema::{ComputedKind, EntityConstraint, RefAction, RelationRef};

//! DOL command language — universal `Operation` enum, `Program`,
//! `Backend` trait, capability layer, and DDL / ACL / Tx / storage
//! builders.
//!
//! The IR is a noun/verb hybrid: structural / governance variants are nouns
//! (`Schema`, `Field`, `Index`, `Lookup`, `Policy`, `Mask`, `Quota`,
//! `Audit`) and carry a [`StructuralVerb`](operation::StructuralVerb).
//! Data / query / authorization variants are verbs (`Insert`, `Update`,
//! `Replace`, `Delete`, `Upsert`, `Append`, `Query`, `Probe`, `Describe`,
//! `Grant`, `Revoke`). Meta variants (`Tx`, `Extension`, feature-gated
//! `Raw`) round it out.
//!
//! Top-down builders that emit ready-made [`Program`](program::Program)
//! values for non-fluent verbs (`define_entity`, `grant`, `tx_begin`,
//! `read_file`, …) live in [`builders`].
//!
//! See `docs/IR.md` for the design overview and `docs/rfcs/0001-ir.md`
//! for the rationale.
//!
//! Schema addressing primitives (`SchemaRef`, `SchemaCatalog`, `TypeBody`,
//! `EntityConstraint`, `RefAction`, `ComputedKind`, `RelationRef`) live
//! in [`dol_schema`] and are imported from there directly — there are no
//! convenience re-exports at the [`dol_command`](crate) crate root.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![warn(missing_docs)]

extern crate alloc;

pub mod backend;
pub mod builders;
pub mod capabilities;
pub mod operation;
pub mod prelude;
pub mod privilege;
pub mod program;
pub mod program_ref;
pub mod target;

/// Lower the fluent [`dol_query`] DSL builders into [`program::Program`]
/// values. Only available with `feature = "query"` (default-on); see the
/// module docs for the full surface.
#[cfg(feature = "query")]
pub mod lower_query;

/// Typed [`operation::OperationExtension`] payloads wrapping
/// [`dol_stream`] and [`dol_pipeline`] data into
/// [`operation::Operation::Extension`]. Only available with
/// `feature = "query"`.
#[cfg(feature = "query")]
pub mod query_extensions;

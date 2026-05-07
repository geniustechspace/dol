//! DOL IR — universal `Operation` enum and Backend trait.
//!
//! The IR is a noun/verb hybrid: structural / governance variants are nouns
//! (`Schema`, `Field`, `Index`, `Lookup`, `Policy`, `Mask`, `Quota`,
//! `Audit`) and carry a [`StructuralVerb`](operation::StructuralVerb).
//! Data / query / authorization variants are verbs (`Insert`, `Update`,
//! `Replace`, `Delete`, `Upsert`, `Append`, `Query`, `Probe`, `Describe`,
//! `Grant`, `Revoke`). Meta variants (`Tx`, `Extension`, feature-gated
//! `Raw`) round it out.
//!
//! See `docs/IR.md` for the design overview and `docs/rfcs/0001-ir.md`
//! for the rationale.
//!
//! Schema constraint types (`RefAction`, `ComputedKind`, `RelationRef`,
//! `EntityConstraint`) live in [`dol_schema`] and are re-exported here for
//! convenience.

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
pub mod capabilities;
pub mod operation;
pub mod prelude;
pub mod privilege;
pub mod program;
pub mod program_ref;
pub mod target;

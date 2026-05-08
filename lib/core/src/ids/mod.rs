//! # `ids` — typed, niche-optimised handles.
//!
//! `dol-core` exposes a single generic [`Id<Tag>`] type for compact,
//! strongly-typed handles into arenas, pools, and tables.
//!
//! Each owner defines its own zero-sized tag and type alias close to the
//! storage it identifies:
//!
//! ```
//! use dol_core::ids::Id;
//!
//! pub enum NodeTag {}
//! pub type NodeId = Id<NodeTag>;
//!
//! pub enum FieldTag {}
//! pub type FieldId = Id<FieldTag>;
//! ```
//!
//! Different tags produce structurally distinct types, so a `NodeId` cannot
//! accidentally be passed where a `FieldId` is expected.
//!
//! ## Niche
//!
//! [`Id<Tag>`] wraps [`NonZeroU32`](core::num::NonZeroU32). This means
//! `Option<Id<Tag>>` is exactly four bytes: `0` is reserved for `None`, while
//! valid ids occupy the range `1..=u32::MAX`.
//!
//! Code that needs a missing handle should use `Option<Id<Tag>>`, not a
//! sentinel id.
//!
//! ## Ownership convention
//!
//! This module owns only the generic id primitive. Concrete aliases such as
//! `StrId`, `NodeId`, `FieldId`, or `TypeId` should live beside the pool or
//! arena they identify.

mod base;

pub use base::Id;

//! String interning pools — [`StringPool`] (std, sharded `RwLock`) and
//! [`StaticStringPool`] (bare-metal, fixed-size).
//!
//! Per `dol-rewrite-plan-v2.md` §7.4.
//!
//! ## Two implementations, one job
//!
//! Both pools issue [`StrId`](crate::handle::StrId) handles and resolve
//! them back to `&str`. They differ in storage strategy:
//!
//! | pool                | storage              | sync          | requires      |
//! |---------------------|----------------------|---------------|---------------|
//! | [`StringPool`]      | global byte buffer + slot table | one `RwLock` | `feature = "std"` |
//! | [`StaticStringPool`]| fixed-size byte slab + slot table | none (`!Sync`) | `no_std + no_alloc` |
//!
//! Both use xxHash3 ([`dol_core::hash::fast64_seeded`]) for lookup and
//! lazy BLAKE3-128 ([`dol_core::hash::content128`]) for `to_cid()`.
//!
//! ### Sharding (deferred to M3)
//!
//! The plan specifies a sharded interior to reduce contention. M2
//! ships a single-locked variant because per-shard slot tables would
//! force shard-index bits into the `StrId` (breaking the simple
//! `Lid::index() == slot_idx` contract). M3 will re-introduce
//! sharding either via upper-bit encoding or by sharding only the
//! lookup index while keeping slot/byte vectors global. The
//! user-visible API is identical.
//!
//! ### `impl PathSegment for Lid<StrTag>` — closed
//!
//! The trait is implemented for [`StrId`](crate::handle::StrId) with
//! [`StringPool`] as its resolver (see [`segment_impl`]). Resolution
//! is zero-copy: `StringPool::get` borrows directly into the slot's
//! stable heap allocation, so a `Path<Lid<StrTag>>` can be resolved
//! against any pool that issued its handles.

mod r#static;

pub use r#static::{StaticInternError, StaticStringPool};

#[cfg(feature = "std")]
mod dynamic;
#[cfg(feature = "std")]
pub use dynamic::{InternError, StringPool};

#[cfg(feature = "std")]
mod segment_impl;

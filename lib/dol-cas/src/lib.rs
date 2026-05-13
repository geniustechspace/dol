//! # `dol-cas` — content addressing system
//!
//! Per `dol-rewrite-plan-v2.md` §7.
//!
//! Identity and pooling layer that sits between [`dol_core`] (the
//! foundation) and `dol-ir` (M3, expression arena and lowering).
//!
//! ## Public surface
//!
//! | item | shape | purpose |
//! |------|-------|---------|
//! | [`handle::Lid<Tag>`] | 4-byte `NonZeroU32` (alias of `dol_core::ids::Id`) | local in-process handle |
//! | [`handle::Cid<Tag>`] | 16-byte BLAKE3-128 | cross-process stable content address |
//! | [`handle::Gid<Tag>`] | 32-byte BLAKE3-256 | global identity for signed manifests |
//! | [`pool::DynPool<T>`] | `Vec`-backed arena (std) | generic node storage |
//! | [`pool::StaticPool<T, CAP>`] | fixed-size arena (no-alloc) | bare-metal arena |
//! | [`string_pool::StringPool`] | sync interner (std) | replaces v1 `dol_expr::Interner` |
//! | [`string_pool::StaticStringPool`] | fixed-size interner (no-alloc) | `iot-min` deployment |
//! | [`content_index::ContentIndex`] | `HashMap<NodeId, [u8;16]>` (std) | lazy node → content address |
//!
//! ## Two-mode path bridge — closed
//!
//! Both segment shapes resolve through the same
//! [`PathSegment`](dol_core::path::PathSegment) trait:
//!
//! ```text
//! Path<Name>         → resolver = &()           → no pool needed
//! Path<Lid<StrTag>>  → resolver = &StringPool   → pool-backed, compact
//! ```
//!
//! `dol-cas` ships [`impl PathSegment for Lid<StrTag>`](handle::StrId)
//! whose `Resolver` type is [`string_pool::StringPool`]. Resolution is
//! zero-copy: [`StringPool::get`](string_pool::StringPool::get) borrows
//! into the slot's stable per-allocation byte storage, so
//! `PathSegment::resolve` can return `&'a str` without copying or
//! locking the slot.

#![cfg_attr(not(feature = "std"), no_std)]
// `unsafe` is forbidden everywhere except the single, narrowly-scoped
// lifetime extension inside `string_pool::dynamic::StringPool::get`,
// which carries a documented safety justification grounded in the
// per-slot `Box<[u8]>` storage. See that function's `SAFETY:` comment.
#![deny(unsafe_code)]
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
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod content_index;
pub mod handle;
pub mod pool;
pub mod string_pool;

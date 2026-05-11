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
//! ## Two-mode path bridge — partial
//!
//! M2 ships every primitive (`Lid<StrTag>`, `StringPool`, `Cid<StrTag>`)
//! but **defers** the `impl PathSegment for Lid<StrTag>`. The
//! `PathSegment::resolve` lifetime contract (`&'a str` borrowed from
//! the resolver) cannot be satisfied while string bytes live behind
//! an `RwLock`. Closing the bridge requires either reshaping the
//! trait (e.g. adding an associated `Resolved<'a>` type) or layering
//! an append-only resolver — both touch dol-core and fit better with
//! M3 lowering. Until then, callers can manually resolve `StrId`s via
//! [`string_pool::StringPool::get`] and feed the resulting owned
//! strings into `Path<Name>`.

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
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod content_index;
pub mod handle;
pub mod pool;
pub mod string_pool;

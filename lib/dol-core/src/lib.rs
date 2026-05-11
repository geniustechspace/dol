//! # `dol-core` — foundation for the DOL stack
//!
//! `dol-core` bundles four independently coherent pieces that every other
//! `dol-*` crate builds on:
//!
//! - **Source spans** ([`span`]) — compact `(file, start, length)` triples
//!   used by every diagnostic and AST node table.
//! - **Diagnostics** ([`diagnostic`]) — structured, code-driven error reports with
//!   labels, notes, and fix-its. Used in lieu of panics by every validator.
//! - **Foundation primitives** ([`policy`], [`ids`], [`storage`]) — [`policy::Limits`]
//!   and [`Budget`] for bounded recursive traversals, [`Id`] for typed
//!   niche-optimised arena handles (`Option<Id<T>>` is 4 bytes), and the
//!   [`storage::Storage`] trait that lets future arenas swap their backing
//!   container between `Vec` (default) and bounded embedded alternatives
//!   (`heapless::Vec`, `arrayvec::ArrayVec`).
//! - **The DOL value/type system** — the universal logical type vocabulary:
//!   [`Value`], [`Literal`], [`DataType`], plus the validated primitives
//!   (`Decimal`, `Date`, `Time`, `Interval`, `IpAddr`, `MacAddr`, `BitString`,
//!   `geo::Point`, …) and [`TypeError`].
//!
//! The crate is `no_std + alloc`-clean; building with `--no-default-features`
//! yields an embedded-friendly library that omits the wall-clock factory
//! helpers ([`datetime::today`], [`datetime::now`], [`datetime::now_tz`]) and
//! drops the `geo`, `network`, `datetime`, and `numeric` modules along with
//! the corresponding `Value` / `Literal` / `DataType` / `TypeError` variants.
//!
//! # Public surface
//!
//! The most-used items are re-exported flat at the crate root:
//!
//! ```ignore
//! use dol_core::{Value, Literal, DataType, TypeError, Span, FileId,
//!                Diagnostic, Severity, ErrorCode, Id, Limits, Budget};
//! ```
//!
//! Sub-namespaces remain accessible for items not promoted to the root:
//!
//! ```ignore
//! use dol_core::span::SpanTable;
//! use dol_core::diag::{Label, Note, FixIt};
//! use dol_core::policy::BudgetError;
//! use dol_core::storage::Storage;
//! use dol_core::datetime::{Date, Time, DateTime, Interval, Offset, TimestampTz};
//! use dol_core::geo::{Point, Line, Polygon, Rect, Circle, Path, Segment};
//! use dol_core::network::{IpAddr, MacAddr};
//! use dol_core::numeric::Decimal;
//! use dol_core::binary::BitString;
//! ```
//!
//! # Cargo features
//!
//! | feature    | default | effect                                                                                                |
//! | ---------- | :-----: | ----------------------------------------------------------------------------------------------------- |
//! | `std`      |   ✓     | Enables wall-clock factory helpers (implies `datetime`).                                              |
//! | `serde`    |   ✓     | `Serialize` for every public type, including [`Span`] and [`Diagnostic`]. v2 wire-in goes through `dol_wire::Decode` — no `Deserialize` impls. |
//! | `geo`      |   ✓     | The `geo` module + `Value`/`Literal`/`DataType`/`TypeError` variants for geometric types.             |
//! | `network`  |   ✓     | The `network` module + `Inet`/`MacAddr` variants on `Value`/`Literal`/`DataType`.                     |
//! | `datetime` |   ✓     | The `datetime` module + `Date`/`Time`/`DateTime`/`TimestampTz`/`Interval` variants.                   |
//! | `numeric`  |   ✓     | The `numeric` module + `Decimal` variants and decimal-related `TypeError` arms.                       |
//!
//! `core::error::Error` is always available for [`TypeError`] and
//! `network::ParseMacAddrError` (stable since Rust 1.81).
//!
//! # Size guarantees (64-bit targets)
//!
//! ```text
//! size_of::<Value>()            == 24
//! size_of::<Literal<'static>>() == 32
//! size_of::<Span>()             ==  8
//! ```

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
// TODO(docs): chip away at the missing-docs allowlist below. Each entry
// represents a module whose public API surface still needs rustdoc coverage.
// The crate-level `#![warn(missing_docs)]` guarantees no new undocumented
// public items will appear.

extern crate alloc;

// ─── Namespaced infrastructure ───────────────────────────────────────────────

// `content_addressing` is preserved on disk as the v1 reference but is not
// re-exported during the M0 v2 scaffold: it relied on `crate::budget` /
// `crate::id` paths that no longer exist and on const-generic expressions
// (`[u8; BITS/8]`) that require nightly. The M3 `dol-ir` content index will
// supersede it.
// pub mod content_addressing;
pub mod budget;
pub mod config;
pub mod diagnostic;
#[cfg(feature = "hash")]
pub mod hash;
pub mod ids;
pub mod path;
pub mod policy;
pub mod raw;
pub mod signing;
pub mod span;
pub mod storage;
pub mod strings;

// ─── Type system sub-namespaces ──────────────────────────────────────────────

#[allow(missing_docs)] // tracking: docs follow-up
pub mod binary;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod data_type;
#[cfg(feature = "datetime")]
#[allow(missing_docs)] // tracking: docs follow-up
pub mod datetime;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod error;
mod format;
#[cfg(feature = "geo")]
#[allow(missing_docs)] // tracking: docs follow-up
pub mod geo;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod literal;
#[cfg(feature = "network")]
#[allow(missing_docs)] // tracking: docs follow-up
pub mod network;
#[cfg(feature = "numeric")]
#[allow(missing_docs)] // tracking: docs follow-up
pub mod numeric;
pub mod prelude;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod value;

// ─── Flat top-level re-exports — the 5–10 most-used items ────────────────────

pub use error::TypeError;

pub use binary::BitString;
#[cfg(feature = "datetime")]
pub use datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
#[cfg(feature = "geo")]
pub use geo::{Circle, Line, Path as GeoPath, Point, Polygon, Rect, Segment};
#[cfg(feature = "network")]
pub use network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
pub use numeric::Decimal;

pub use data_type::{DataType, StructField};

pub use literal::{Literal, LiteralRange};
pub use value::{Value, ValueRange};

// span / diag flat re-exports — the headline boundary items.
pub use diagnostic::{Diagnostic, ErrorCode, Severity};
pub use span::{FileId, Span};

// policy / id flat re-exports — the load-bearing foundation primitives
// every recursive or arena-bearing path threads.
pub use ids::Id;
pub use policy::{Budget, BudgetError, Limits};

pub use path::{Path, PathSegment};

//! # `dol-core` — foundation for the DOL stack
//!
//! `dol-core` bundles three independently coherent pieces that every other
//! `dol-*` crate builds on:
//!
//! - **Source spans** ([`span`]) — compact `(file, start, length)` triples
//!   used by every diagnostic and AST node table.
//! - **Diagnostics** ([`diag`]) — structured, code-driven error reports with
//!   labels, notes, and fix-its. Used in lieu of panics by every validator.
//! - **The DOL value/type system** — the universal logical type vocabulary:
//!   [`Value`], [`Literal`], [`DataType`], plus the validated primitives
//!   (`Decimal`, `Date`, `Time`, `Interval`, `IpAddr`, `MacAddr`, `BitString`,
//!   `geo::Point`, …) and [`TypeError`].
//!
//! The crate is `no_std + alloc`-clean; building with `--no-default-features`
//! yields an embedded-friendly library that omits the `std::error::Error`
//! impl on [`TypeError`] and the wall-clock factory helpers
//! ([`datetime::today`], [`datetime::now`], [`datetime::now_tz`]).
//!
//! # Public surface
//!
//! The most-used items are re-exported flat at the crate root:
//!
//! ```ignore
//! use dol_core::{Value, Literal, DataType, TypeError, Span, FileId,
//!                Diagnostic, Severity, Code};
//! ```
//!
//! Sub-namespaces remain accessible for items not promoted to the root:
//!
//! ```ignore
//! use dol_core::span::SpanTable;
//! use dol_core::diag::{Label, Note, FixIt};
//! use dol_core::datetime::{Date, Time, DateTime, Interval, Offset, TimestampTz};
//! use dol_core::geo::{Point, Line, Polygon, Rect, Circle, Path, Segment};
//! use dol_core::network::{IpAddr, MacAddr};
//! use dol_core::numeric::Decimal;
//! use dol_core::binary::BitString;
//! ```
//!
//! # Cargo features
//!
//! | feature | default | effect                                                                            |
//! | ------- | :-----: | --------------------------------------------------------------------------------- |
//! | `std`   |   ✓     | Enables `std::error::Error` for [`TypeError`] and the wall-clock factory helpers. |
//! | `serde` |   ✓     | `Serialize` / `Deserialize` for every public type, including [`Span`] and [`Diagnostic`]. |
//!
//! # Size guarantees (64-bit targets)
//!
//! ```text
//! size_of::<Value>()            == 24
//! size_of::<Literal<'static>>() == 32
//! size_of::<Span>()             ==  8
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
// TODO(docs): chip away at the missing-docs allowlist below. Each entry
// represents a module whose public API surface still needs rustdoc coverage.
// The crate-level `#![warn(missing_docs)]` guarantees no new undocumented
// public items will appear.

extern crate alloc;

// ─── Namespaced infrastructure ───────────────────────────────────────────────

pub mod diag;
pub mod span;

// ─── Type system sub-namespaces ──────────────────────────────────────────────

#[allow(missing_docs)] // tracking: docs follow-up
pub mod binary;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod data_type;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod datetime;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod error;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod geo;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod network;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod numeric;
pub mod prelude;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod value;

/// Compatibility re-export — the old `descriptor` module has moved to [`data_type`].
#[doc(hidden)]
pub use data_type as descriptor;

// ─── Flat top-level re-exports — the 5–10 most-used items ────────────────────

pub use error::TypeError;

pub use binary::BitString;
pub use datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
pub use geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
pub use network::{IpAddr, MacAddr};
pub use numeric::Decimal;

pub use data_type::{DataType, StructField};

pub use value::{Literal, LiteralRange, Value, ValueRange};

// span / diag flat re-exports — the headline boundary items.
pub use diag::{Code, Diagnostic, Severity};
pub use span::{FileId, Span};

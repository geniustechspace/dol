//! DOL type system — the single source of truth for every type in DOL.
//!
//! This crate is a leaf crate with no DOL dependencies; `dol-expr` re-exports
//! from it so there is exactly one definition of every type.
//!
//! # Layers
//!
//! | Layer | Types | Purpose |
//! |-------|-------|---------|
//! | **Values** | [`Value`], [`Literal`] | Carry actual data at runtime and in ASTs |
//! | **Descriptors** | [`DataType`], [`StructField`] | Describe the expected shape of a position |
//! | **Primitives** | [`Decimal`], [`Date`], [`Time`], [`Interval`], [`IpAddr`], [`MacAddr`], [`BitString`], [`geo::Point`], etc. | Structural sub-types with validated constructors |
//! | **Errors** | [`TypeError`] | All validation and conformance errors |
//!
//! # The `DataType` / `Value` contract
//!
//! [`DataType`] describes what a field or parameter expects.
//! [`Value`] is what actually arrives.
//! [`DataType::accepts`] is the bridge: it returns `Ok(())` if the value
//! satisfies the type, or a structured [`TypeError`] otherwise.
//!
//! # Size guarantees (64-bit targets)
//!
//! ```text
//! size_of::<Value>()            == 24
//! size_of::<Literal<'static>>() == 32
//! ```
//!
//! These are enforced by tests in [`value`] and must not regress.
//!
//! # Cargo features
//!
//! | feature | default | effect                                                                            |
//! | ------- | :-----: | --------------------------------------------------------------------------------- |
//! | `std`   |   ✓     | Enables `std::error::Error` for [`TypeError`] and the wall-clock factory helpers ([`datetime::today`], [`datetime::now`], [`datetime::now_tz`]). Disable for `no_std + alloc` targets. |
//! | `serde` |   ✓     | `Serialize` / `Deserialize` for every public type.                                |
//!
//! Building with `--no-default-features` yields a `no_std + alloc` library
//! suitable for embedded targets that cannot link `std`.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
// TODO(docs): chip away at the missing-docs allowlist below. Each entry
// represents a module whose public API surface still needs rustdoc coverage.
// The crate-level `#![warn(missing_docs)]` guarantees no new undocumented
// public items will appear.

extern crate alloc;

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

// ─── Re-exports ───────────────────────────────────────────────────────────────

pub use error::TypeError;

pub use binary::BitString;
pub use datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
pub use geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
pub use network::{IpAddr, MacAddr};
pub use numeric::Decimal;

pub use data_type::{DataType, StructField};

pub use value::{Literal, LiteralRange, Value, ValueRange};

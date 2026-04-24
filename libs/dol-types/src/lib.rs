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
//! | **Primitives** | [`Decimal`], [`Date`], [`Time`], [`Interval`], [`IpAddr`], [`MacAddr`], [`MacAddr8`], [`BitString`], [`geo::Point`], etc. | Structural sub-types with validated constructors |
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

#![deny(unsafe_code)]

pub mod binary;
pub mod datetime;
pub mod descriptor;
pub mod error;
pub mod geo;
pub mod network;
pub mod numeric;
pub mod value;

// ─── Re-exports ───────────────────────────────────────────────────────────────

pub use error::TypeError;

pub use binary::BitString;
pub use datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
pub use geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
pub use network::{IpAddr, MacAddr, MacAddr8};
pub use numeric::Decimal;

pub use descriptor::{DataType, StructField};

pub use value::{Literal, LiteralRange, Value, ValueRange};

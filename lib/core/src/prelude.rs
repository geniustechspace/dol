//! Curated re-exports — the stable, recommended surface for downstream users.
//!
//! ```ignore
//! use dol_core::prelude::*;
//! ```

pub use crate::error::TypeError;

pub use crate::binary::BitString;
pub use crate::data_type::{DataType, StructField};
#[cfg(feature = "datetime")]
pub use crate::datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
#[cfg(feature = "geo")]
pub use crate::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
pub use crate::literal::{Literal, LiteralRange};
#[cfg(feature = "network")]
pub use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
pub use crate::numeric::Decimal;
pub use crate::value::{Value, ValueRange};

// NOTE: `Span` / `FileId` / `Diagnostic` / `ErrorCode` / `Severity` are *not*
// glob-exported through the prelude — `dol-expr` has its own arena-internal
// `Span` type, and downstream code that wants the diagnostic surface should
// reach for it explicitly via `dol_core::{Span, Diagnostic, …}` or
// `dol_core::span::*` / `dol_core::diag::*`.

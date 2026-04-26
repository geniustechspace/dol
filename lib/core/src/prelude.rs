//! Curated re-exports — the stable, recommended surface for downstream users.
//!
//! ```ignore
//! use dol_core::prelude::*;
//! ```

pub use crate::error::TypeError;

pub use crate::binary::BitString;
pub use crate::data_type::{DataType, StructField};
pub use crate::datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
pub use crate::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
pub use crate::network::{IpAddr, MacAddr};
pub use crate::numeric::Decimal;
pub use crate::value::{Literal, LiteralRange, Value, ValueRange};

// NOTE: `Span` / `FileId` / `Diagnostic` / `Code` / `Severity` are *not*
// glob-exported through the prelude — `dol-expr` has its own arena-internal
// `Span` type, and downstream code that wants the diagnostic surface should
// reach for it explicitly via `dol_core::{Span, Diagnostic, …}` or
// `dol_core::span::*` / `dol_core::diag::*`.

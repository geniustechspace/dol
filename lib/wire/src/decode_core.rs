//! [`Decode`] impls for the `dol-core` leaf type families.
//!
//! Each impl is byte-for-byte compatible with the postcard encoding
//! produced by the corresponding `serde::Serialize` derivation on the same
//! type, unless noted otherwise. The `decode_core_roundtrip` integration
//! test asserts this against `postcard::to_allocvec(&value)` for the
//! postcard-compatible types, and self-round-trips the rest.
//!
//! Coverage in v2 (0.2.0) — **complete**:
//!
//! - **Always-on (postcard-compatible):** `BitString`, `FileId`, `Span`,
//!   `SpanTable`.
//! - **`datetime` feature (postcard-compatible):** `Date`, `Time`,
//!   `DateTime`, `Offset`, `TimestampTz`, `Interval`.
//! - **`numeric` feature (postcard-compatible):** `Decimal`.
//! - **`geo` feature (postcard-compatible):** `Point`, `Line`, `Segment`,
//!   `Rect`, `Circle`, `Path`, `Polygon`.
//! - **`network` feature (custom discriminant, NOT postcard-compatible):**
//!   `IpAddr`, `MacAddr`. Both use `serde(untagged)` which postcard cannot
//!   handle; we emit a 1-byte explicit discriminant instead.
//! - **Always-on (custom stable discriminants, NOT postcard-compatible):**
//!   `DataType`, `StructField`, `Value`, `ValueRange`. The feature-gated
//!   variants in these recursive enums cause postcard discriminants to shift
//!   with the active feature set, so we assign stable fixed discriminants
//!   with gaps between feature ranges. See the discriminant constant tables
//!   in each impl block for the stable assignment.
//! - **Always-on (custom stable discriminants, produces `'static` copies):**
//!   `Literal<'static>`, `LiteralRange<'static>`. Same discriminant gap
//!   scheme as `Value`; string/bytes payloads are decoded into owned
//!   (`Box<str>` / `Box<[u8]>`) giving the caller a `'static` value.

use alloc::boxed::Box;
use alloc::vec::Vec;

use dol_core::policy::Budget;

use crate::decoder::{Decode, DecodeError, Reader};

// ─── Always-on leaves ───────────────────────────────────────────────────────

impl Decode for dol_core::BitString {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        // Postcard layout for `BitString { len: u32, bytes: Box<[u8]> }`:
        // varint(len) followed by varint(byte_count) + byte_count raw bytes.
        let len = budget.descend(|b| u32::decode(reader, b))??;
        let bytes = budget.descend(|b| Box::<[u8]>::decode(reader, b))??;
        // Validate the byte-count invariant promised by `BitString::try_new`.
        let required = len.div_ceil(8) as usize;
        if bytes.len() != required {
            return Err(DecodeError::Custom(
                "BitString: declared bit length does not match byte count",
            ));
        }
        Ok(Self::new_unchecked(len, bytes))
    }
}

impl Decode for dol_core::FileId {
    #[inline]
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        Ok(Self(u16::decode(reader, budget)?))
    }
}

// ─── Datetime leaves ────────────────────────────────────────────────────────

#[cfg(feature = "datetime")]
mod datetime_impls {
    use super::*;

    impl Decode for dol_core::Date {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let year = budget.descend(|b| i32::decode(reader, b))??;
            let month = budget.descend(|b| u8::decode(reader, b))??;
            let day = budget.descend(|b| u8::decode(reader, b))??;
            // `try_new` validates the calendar fields. A wire-coming Date
            // with month 0 / 13 / day 32 is malformed input, not a bug —
            // surface it as a `DecodeError`.
            Self::try_new(year, month, day)
                .map_err(|_| DecodeError::Custom("Date: invalid calendar field"))
        }
    }

    impl Decode for dol_core::Time {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let hour = budget.descend(|b| u8::decode(reader, b))??;
            let minute = budget.descend(|b| u8::decode(reader, b))??;
            let second = budget.descend(|b| u8::decode(reader, b))??;
            let nanosecond = budget.descend(|b| u32::decode(reader, b))??;
            Self::try_new(hour, minute, second, nanosecond)
                .map_err(|_| DecodeError::Custom("Time: invalid clock field"))
        }
    }

    impl Decode for dol_core::DateTime {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let date = budget.descend(|b| dol_core::Date::decode(reader, b))??;
            let time = budget.descend(|b| dol_core::Time::decode(reader, b))??;
            Ok(Self::new(date, time))
        }
    }

    impl Decode for dol_core::Offset {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let seconds = budget.descend(|b| i32::decode(reader, b))??;
            Self::try_from_seconds(seconds)
                .map_err(|_| DecodeError::Custom("Offset: out of range"))
        }
    }

    impl Decode for dol_core::TimestampTz {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let datetime = budget.descend(|b| dol_core::DateTime::decode(reader, b))??;
            let offset = budget.descend(|b| dol_core::Offset::decode(reader, b))??;
            Ok(Self::new(datetime, offset))
        }
    }

    impl Decode for dol_core::Interval {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let months = budget.descend(|b| i32::decode(reader, b))??;
            let days = budget.descend(|b| i32::decode(reader, b))??;
            let nanoseconds = budget.descend(|b| i64::decode(reader, b))??;
            Ok(Self::new(months, days, nanoseconds))
        }
    }
}

// ─── Numeric leaves ─────────────────────────────────────────────────────────

#[cfg(feature = "numeric")]
mod numeric_impls {
    use super::*;

    impl Decode for dol_core::Decimal {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let unscaled = budget.descend(|b| i128::decode(reader, b))??;
            let scale = budget.descend(|b| u32::decode(reader, b))??;
            // Validate scale; out-of-range is malformed input, not a bug.
            Self::try_new(unscaled, scale)
                .map_err(|_| DecodeError::Custom("Decimal: scale exceeds MAX_SCALE"))
        }
    }
}

// ─── Geo leaves ─────────────────────────────────────────────────────────────

#[cfg(feature = "geo")]
mod geo_impls {
    use super::*;

    impl Decode for dol_core::Point {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let x = budget.descend(|b| f64::decode(reader, b))??;
            let y = budget.descend(|b| f64::decode(reader, b))??;
            // `try_new` rejects NaN / ±∞; do the same for wire input.
            Self::try_new(x, y)
                .map_err(|_| DecodeError::Custom("Point: non-finite coordinate"))
        }
    }

    impl Decode for dol_core::geo::Line {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let a = budget.descend(|b| f64::decode(reader, b))??;
            let b = budget.descend(|bg| f64::decode(reader, bg))??;
            let c = budget.descend(|b| f64::decode(reader, b))??;
            Self::try_new(a, b, c)
                .map_err(|_| DecodeError::Custom("Line: non-finite coefficient"))
        }
    }

    impl Decode for dol_core::geo::Segment {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let start = budget.descend(|b| dol_core::Point::decode(reader, b))??;
            let end = budget.descend(|b| dol_core::Point::decode(reader, b))??;
            Ok(Self::new(start, end))
        }
    }

    impl Decode for dol_core::geo::Rect {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let low = budget.descend(|b| dol_core::Point::decode(reader, b))??;
            let high = budget.descend(|b| dol_core::Point::decode(reader, b))??;
            Ok(Self::new(low, high))
        }
    }

    impl Decode for dol_core::geo::Circle {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let center = budget.descend(|b| dol_core::Point::decode(reader, b))??;
            let radius = budget.descend(|b| f64::decode(reader, b))??;
            Self::try_new(center, radius)
                .map_err(|_| DecodeError::Custom("Circle: non-finite or negative radius"))
        }
    }
}

// ─── Span and SpanTable (always-on, postcard-compatible) ─────────────────────

impl Decode for dol_core::Span {
    #[inline]
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        // Postcard layout for `Span(u64)`: transparent newtype → varint u64.
        let raw = u64::decode(reader, budget)?;
        Ok(Self::from_raw_u64(raw))
    }
}

impl Decode for dol_core::span::SpanTable {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        // Postcard layout for `SpanTable { spans: Vec<Span> }`:
        // varint(len) followed by len `Span` values.
        let len = reader.read_varint_u32()? as usize;
        if len > 1 << 20 {
            return Err(DecodeError::LengthOverflow);
        }
        let mut table = dol_core::span::SpanTable::with_capacity(len.min(64));
        for _ in 0..len {
            let span = budget.descend(|b| dol_core::Span::decode(reader, b))??;
            table.push(span);
        }
        Ok(table)
    }
}

// ─── Network leaves (custom discriminant, NOT postcard-compatible) ────────────
//
// `IpAddr` and `MacAddr` use `serde(untagged)` so postcard returns
// `WontImplement` for them.  Our v2 encoding adds a 1-byte discriminant:
//
//   IpAddr  — 0x00 = V4 (4 bytes follow), 0x01 = V6 (16 bytes follow)
//   MacAddr — 0x00 = Eui48 (6 bytes follow), 0x01 = Eui64 (8 bytes follow)

#[cfg(feature = "network")]
mod network_impls {
    use super::*;

    impl Decode for dol_core::IpAddr {
        fn decode(reader: &mut Reader<'_>, _budget: &mut Budget) -> Result<Self, DecodeError> {
            match reader.read_u8()? {
                0 => Ok(dol_core::IpAddr::V4(reader.read_array::<4>()?)),
                1 => Ok(dol_core::IpAddr::V6(reader.read_array::<16>()?)),
                seen => Err(DecodeError::InvalidVariant {
                    type_name: "IpAddr",
                    seen: seen as u32,
                }),
            }
        }
    }

    impl Decode for dol_core::MacAddr {
        fn decode(reader: &mut Reader<'_>, _budget: &mut Budget) -> Result<Self, DecodeError> {
            match reader.read_u8()? {
                0 => Ok(dol_core::MacAddr::Eui48(reader.read_array::<6>()?)),
                1 => Ok(dol_core::MacAddr::Eui64(reader.read_array::<8>()?)),
                seen => Err(DecodeError::InvalidVariant {
                    type_name: "MacAddr",
                    seen: seen as u32,
                }),
            }
        }
    }
}

// ─── Geo compound leaves (postcard-compatible) ────────────────────────────────

#[cfg(feature = "geo")]
mod geo_compound_impls {
    use super::*;

    // Helper: decode a length-prefixed sequence of Points as a boxed slice.
    fn decode_points(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<Box<[dol_core::Point]>, DecodeError> {
        let len = reader.read_varint_u32()? as usize;
        if len > 1 << 20 {
            return Err(DecodeError::LengthOverflow);
        }
        let mut pts = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            let p = budget.descend(|b| dol_core::Point::decode(reader, b))??;
            pts.push(p);
        }
        Ok(pts.into_boxed_slice())
    }

    impl Decode for dol_core::geo::Path {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            // Postcard layout: bool (closed) + varint(len) + [Point…].
            let closed = budget.descend(|b| bool::decode(reader, b))??;
            let points = decode_points(reader, budget)?;
            Ok(Self::new(closed, points.into_vec()))
        }
    }

    impl Decode for dol_core::geo::Polygon {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            // Postcard layout: varint(len) + [Point…].
            let points = decode_points(reader, budget)?;
            Ok(Self::new(points.into_vec()))
        }
    }
}

// ─── DataType and StructField (stable custom discriminants) ───────────────────
//
// Feature-gated variants cause postcard's sequential discriminants to shift
// with the active feature set, so we assign STABLE fixed discriminants:
//
//   Always-on primitives  :  0-13
//   Always-on text/binary : 14-19
//   Always-on composite   : 20-28
//   numeric feature       : 30
//   datetime feature      : 40-44
//   network feature       : 50-52
//   geo feature           : 60-66
//
// These discriminants are encoded as a varint u32.

#[allow(dead_code)]
mod data_type_impls {
    use super::*;
    use dol_core::DataType;

    // ── Discriminant constants ──────────────────────────────────────────────
    const DT_NULL: u32 = 0;
    const DT_BOOL: u32 = 1;
    const DT_INT8: u32 = 2;
    const DT_INT16: u32 = 3;
    const DT_INT32: u32 = 4;
    const DT_INT64: u32 = 5;
    const DT_INT128: u32 = 6;
    const DT_UINT8: u32 = 7;
    const DT_UINT16: u32 = 8;
    const DT_UINT32: u32 = 9;
    const DT_UINT64: u32 = 10;
    const DT_UINT128: u32 = 11;
    const DT_FLOAT32: u32 = 12;
    const DT_FLOAT64: u32 = 13;
    const DT_STRING: u32 = 14;
    const DT_JSON: u32 = 15;
    const DT_XML: u32 = 16;
    const DT_BYTES: u32 = 17;
    const DT_UUID: u32 = 18;
    const DT_BITSTRING: u32 = 19;
    const DT_ARRAY: u32 = 20;
    const DT_SET: u32 = 21;
    const DT_MAP: u32 = 22;
    const DT_RANGE: u32 = 23;
    const DT_TUPLE: u32 = 24;
    const DT_STRUCT: u32 = 25;
    const DT_ENUM: u32 = 26;
    const DT_TYPEREF: u32 = 27;
    const DT_EXTENSION: u32 = 28;
    // numeric (gap at 29)
    #[cfg(feature = "numeric")]
    const DT_DECIMAL: u32 = 30;
    // datetime (gap at 31-39)
    #[cfg(feature = "datetime")]
    const DT_DATE: u32 = 40;
    #[cfg(feature = "datetime")]
    const DT_TIME: u32 = 41;
    #[cfg(feature = "datetime")]
    const DT_DATETIME: u32 = 42;
    #[cfg(feature = "datetime")]
    const DT_OFFSET_DATETIME: u32 = 43;
    #[cfg(feature = "datetime")]
    const DT_INTERVAL: u32 = 44;
    // network (gap at 45-49)
    #[cfg(feature = "network")]
    const DT_IPADDR: u32 = 50;
    #[cfg(feature = "network")]
    const DT_IPNETWORK: u32 = 51;
    #[cfg(feature = "network")]
    const DT_MACADDR: u32 = 52;
    // geo (gap at 53-59)
    #[cfg(feature = "geo")]
    const DT_POINT: u32 = 60;
    #[cfg(feature = "geo")]
    const DT_LINE: u32 = 61;
    #[cfg(feature = "geo")]
    const DT_LINESEGMENT: u32 = 62;
    #[cfg(feature = "geo")]
    const DT_RECT: u32 = 63;
    #[cfg(feature = "geo")]
    const DT_CIRCLE: u32 = 64;
    #[cfg(feature = "geo")]
    const DT_PATH: u32 = 65;
    #[cfg(feature = "geo")]
    const DT_POLYGON: u32 = 66;

    impl Decode for dol_core::StructField {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            // Struct layout: name (Box<str>) + data_type (DataType) + nullable (bool).
            let name = budget.descend(|b| Box::<str>::decode(reader, b))??;
            let data_type = budget.descend(|b| DataType::decode(reader, b))??;
            let nullable = budget.descend(|b| bool::decode(reader, b))??;
            Ok(dol_core::StructField::new(name, data_type, nullable))
        }
    }

    impl Decode for DataType {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let disc = reader.read_varint_u32()?;
            match disc {
                DT_NULL => Ok(DataType::Null),
                DT_BOOL => Ok(DataType::Bool),
                DT_INT8 => Ok(DataType::Int8),
                DT_INT16 => Ok(DataType::Int16),
                DT_INT32 => Ok(DataType::Int32),
                DT_INT64 => Ok(DataType::Int64),
                DT_INT128 => Ok(DataType::Int128),
                DT_UINT8 => Ok(DataType::UInt8),
                DT_UINT16 => Ok(DataType::UInt16),
                DT_UINT32 => Ok(DataType::UInt32),
                DT_UINT64 => Ok(DataType::UInt64),
                DT_UINT128 => Ok(DataType::UInt128),
                DT_FLOAT32 => Ok(DataType::Float32),
                DT_FLOAT64 => Ok(DataType::Float64),
                DT_STRING => {
                    let max_len = budget.descend(|b| Option::<u32>::decode(reader, b))??;
                    let fixed = budget.descend(|b| bool::decode(reader, b))??;
                    Ok(DataType::String { max_len, fixed })
                }
                DT_JSON => Ok(DataType::Json),
                DT_XML => Ok(DataType::Xml),
                DT_BYTES => {
                    let max_len = budget.descend(|b| Option::<u32>::decode(reader, b))??;
                    let fixed = budget.descend(|b| bool::decode(reader, b))??;
                    Ok(DataType::Bytes { max_len, fixed })
                }
                DT_UUID => Ok(DataType::Uuid),
                DT_BITSTRING => {
                    let max_len = budget.descend(|b| Option::<u32>::decode(reader, b))??;
                    let fixed = budget.descend(|b| bool::decode(reader, b))??;
                    Ok(DataType::BitString { max_len, fixed })
                }
                DT_ARRAY => {
                    let inner = budget.descend(|b| DataType::decode(reader, b))??;
                    Ok(DataType::Array(Box::new(inner)))
                }
                DT_SET => {
                    let inner = budget.descend(|b| DataType::decode(reader, b))??;
                    Ok(DataType::Set(Box::new(inner)))
                }
                DT_MAP => {
                    let value = budget.descend(|b| DataType::decode(reader, b))??;
                    Ok(DataType::Map { value: Box::new(value) })
                }
                DT_RANGE => {
                    let inner = budget.descend(|b| DataType::decode(reader, b))??;
                    Ok(DataType::Range(Box::new(inner)))
                }
                DT_TUPLE => {
                    let elems = budget.descend(|b| Vec::<DataType>::decode(reader, b))??;
                    Ok(DataType::Tuple(elems))
                }
                DT_STRUCT => {
                    let fields =
                        budget.descend(|b| Vec::<dol_core::StructField>::decode(reader, b))??;
                    Ok(DataType::Struct(fields))
                }
                DT_ENUM => {
                    let variants = budget.descend(|b| Vec::<Box<str>>::decode(reader, b))??;
                    Ok(DataType::Enum(variants))
                }
                DT_TYPEREF => {
                    let name = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(DataType::TypeRef(name))
                }
                DT_EXTENSION => {
                    let name = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    let params =
                        budget.descend(|b| Vec::<DataType>::decode(reader, b))??;
                    Ok(DataType::Extension { name, params })
                }
                #[cfg(feature = "numeric")]
                DT_DECIMAL => {
                    let precision = budget.descend(|b| Option::<u8>::decode(reader, b))??;
                    let scale = budget.descend(|b| Option::<u8>::decode(reader, b))??;
                    Ok(DataType::Decimal { precision, scale })
                }
                #[cfg(feature = "datetime")]
                DT_DATE => Ok(DataType::Date),
                #[cfg(feature = "datetime")]
                DT_TIME => {
                    let precision = budget.descend(|b| u8::decode(reader, b))??;
                    Ok(DataType::Time { precision })
                }
                #[cfg(feature = "datetime")]
                DT_DATETIME => {
                    let precision = budget.descend(|b| u8::decode(reader, b))??;
                    Ok(DataType::DateTime { precision })
                }
                #[cfg(feature = "datetime")]
                DT_OFFSET_DATETIME => {
                    let precision = budget.descend(|b| u8::decode(reader, b))??;
                    Ok(DataType::OffsetDateTime { precision })
                }
                #[cfg(feature = "datetime")]
                DT_INTERVAL => Ok(DataType::Interval),
                #[cfg(feature = "network")]
                DT_IPADDR => Ok(DataType::IpAddr),
                #[cfg(feature = "network")]
                DT_IPNETWORK => {
                    let prefix_len = budget.descend(|b| u8::decode(reader, b))??;
                    Ok(DataType::IpNetwork { prefix_len })
                }
                #[cfg(feature = "network")]
                DT_MACADDR => Ok(DataType::MacAddr),
                #[cfg(feature = "geo")]
                DT_POINT => Ok(DataType::Point),
                #[cfg(feature = "geo")]
                DT_LINE => Ok(DataType::Line),
                #[cfg(feature = "geo")]
                DT_LINESEGMENT => Ok(DataType::LineSegment),
                #[cfg(feature = "geo")]
                DT_RECT => Ok(DataType::Rect),
                #[cfg(feature = "geo")]
                DT_CIRCLE => Ok(DataType::Circle),
                #[cfg(feature = "geo")]
                DT_PATH => Ok(DataType::Path),
                #[cfg(feature = "geo")]
                DT_POLYGON => Ok(DataType::Polygon),
                _ => Err(DecodeError::InvalidVariant {
                    type_name: "DataType",
                    seen: disc,
                }),
            }
        }
    }
}

// ─── Value and ValueRange (stable custom discriminants) ───────────────────────
//
// Stable discriminant assignment (same gap-based scheme as DataType):
//
//   Always-on primitives  :  0-20
//   Always-on composite   : 21-27
//   numeric feature       : 30
//   network feature       : 40-41
//   datetime feature      : 50-54
//   geo feature           : 60-66

#[allow(dead_code)]
mod value_impls {
    use super::*;
    use core::ops::Bound;
    use dol_core::Value;

    // ── Discriminant constants ──────────────────────────────────────────────
    const V_NULL: u32 = 0;
    const V_BOOL: u32 = 1;
    const V_STRING: u32 = 2;
    const V_JSON: u32 = 3;
    const V_XML: u32 = 4;
    const V_ENUM: u32 = 5;
    const V_BYTES: u32 = 6;
    const V_UUID: u32 = 7;
    const V_BITSTRING: u32 = 8;
    const V_INT8: u32 = 9;
    const V_INT16: u32 = 10;
    const V_INT32: u32 = 11;
    const V_INT64: u32 = 12;
    const V_INT128: u32 = 13;
    const V_UINT8: u32 = 14;
    const V_UINT16: u32 = 15;
    const V_UINT32: u32 = 16;
    const V_UINT64: u32 = 17;
    const V_UINT128: u32 = 18;
    const V_FLOAT32: u32 = 19;
    const V_FLOAT64: u32 = 20;
    const V_ARRAY: u32 = 21;
    const V_SET: u32 = 22;
    const V_TUPLE: u32 = 23;
    const V_MAP: u32 = 24;
    const V_STRUCT: u32 = 25;
    const V_RANGE: u32 = 26;
    const V_EXTENSION: u32 = 27;
    // numeric (gap at 28-29)
    #[cfg(feature = "numeric")]
    const V_DECIMAL: u32 = 30;
    // network (gap at 31-39)
    #[cfg(feature = "network")]
    const V_INET: u32 = 40;
    #[cfg(feature = "network")]
    const V_MACADDR: u32 = 41;
    // datetime (gap at 42-49)
    #[cfg(feature = "datetime")]
    const V_DATE: u32 = 50;
    #[cfg(feature = "datetime")]
    const V_TIME: u32 = 51;
    #[cfg(feature = "datetime")]
    const V_DATETIME: u32 = 52;
    #[cfg(feature = "datetime")]
    const V_TIMESTAMPTZ: u32 = 53;
    #[cfg(feature = "datetime")]
    const V_INTERVAL: u32 = 54;
    // geo (gap at 55-59)
    #[cfg(feature = "geo")]
    const V_POINT: u32 = 60;
    #[cfg(feature = "geo")]
    const V_LINE: u32 = 61;
    #[cfg(feature = "geo")]
    const V_SEGMENT: u32 = 62;
    #[cfg(feature = "geo")]
    const V_RECT: u32 = 63;
    #[cfg(feature = "geo")]
    const V_CIRCLE: u32 = 64;
    #[cfg(feature = "geo")]
    const V_PATH: u32 = 65;
    #[cfg(feature = "geo")]
    const V_POLYGON: u32 = 66;

    // ── Bound<Box<Value>> helper (Included=0, Excluded=1, Unbounded=2) ──────
    fn decode_bound(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<Bound<Box<Value>>, DecodeError> {
        let disc = reader.read_u8()?;
        match disc {
            0 => {
                let v = budget.descend(|b| Value::decode(reader, b))??;
                Ok(Bound::Included(Box::new(v)))
            }
            1 => {
                let v = budget.descend(|b| Value::decode(reader, b))??;
                Ok(Bound::Excluded(Box::new(v)))
            }
            2 => Ok(Bound::Unbounded),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "Bound",
                seen: seen as u32,
            }),
        }
    }

    // Helper: decode a boxed slice of Values.
    fn decode_value_slice(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<Box<[Value]>, DecodeError> {
        let v = Vec::<Value>::decode(reader, budget)?;
        Ok(v.into_boxed_slice())
    }

    // Helper: decode a boxed slice of (Box<str>, Value) pairs.
    fn decode_kv_slice(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<Box<[(Box<str>, Value)]>, DecodeError> {
        let len = reader.read_varint_u32()? as usize;
        if len > 1 << 20 {
            return Err(DecodeError::LengthOverflow);
        }
        let mut out: Vec<(Box<str>, Value)> = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            let k = budget.descend(|b| Box::<str>::decode(reader, b))??;
            let v = budget.descend(|b| Value::decode(reader, b))??;
            out.push((k, v));
        }
        Ok(out.into_boxed_slice())
    }

    impl Decode for dol_core::ValueRange {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let start = budget.descend(|b| decode_bound(reader, b))??;
            let end = budget.descend(|b| decode_bound(reader, b))??;
            Ok(dol_core::ValueRange { start, end })
        }
    }

    impl Decode for Value {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let disc = reader.read_varint_u32()?;
            match disc {
                V_NULL => Ok(Value::Null),
                V_BOOL => Ok(Value::Bool(bool::decode(reader, budget)?)),
                V_STRING => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Value::String(s))
                }
                V_JSON => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Value::Json(s))
                }
                V_XML => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Value::Xml(s))
                }
                V_ENUM => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Value::Enum(s))
                }
                V_BYTES => {
                    let b = budget.descend(|b| Box::<[u8]>::decode(reader, b))??;
                    Ok(Value::Bytes(b))
                }
                V_UUID => Ok(Value::Uuid(reader.read_array::<16>()?)),
                V_BITSTRING => {
                    let bs = budget.descend(|b| dol_core::BitString::decode(reader, b))??;
                    Ok(Value::BitString(Box::new(bs)))
                }
                V_INT8 => Ok(Value::Int8(i8::decode(reader, budget)?)),
                V_INT16 => Ok(Value::Int16(i16::decode(reader, budget)?)),
                V_INT32 => Ok(Value::Int32(i32::decode(reader, budget)?)),
                V_INT64 => Ok(Value::Int64(i64::decode(reader, budget)?)),
                V_INT128 => {
                    let v = budget.descend(|b| i128::decode(reader, b))??;
                    Ok(Value::Int128(Box::new(v)))
                }
                V_UINT8 => Ok(Value::UInt8(u8::decode(reader, budget)?)),
                V_UINT16 => Ok(Value::UInt16(u16::decode(reader, budget)?)),
                V_UINT32 => Ok(Value::UInt32(u32::decode(reader, budget)?)),
                V_UINT64 => Ok(Value::UInt64(u64::decode(reader, budget)?)),
                V_UINT128 => {
                    let v = budget.descend(|b| u128::decode(reader, b))??;
                    Ok(Value::UInt128(Box::new(v)))
                }
                V_FLOAT32 => Ok(Value::Float32(f32::decode(reader, budget)?)),
                V_FLOAT64 => Ok(Value::Float64(f64::decode(reader, budget)?)),
                V_ARRAY => {
                    let elems = budget.descend(|b| decode_value_slice(reader, b))??;
                    Ok(Value::Array(elems))
                }
                V_SET => {
                    let elems = budget.descend(|b| decode_value_slice(reader, b))??;
                    Ok(Value::Set(elems))
                }
                V_TUPLE => {
                    let elems = budget.descend(|b| decode_value_slice(reader, b))??;
                    Ok(Value::Tuple(elems))
                }
                V_MAP => {
                    let pairs = budget.descend(|b| decode_kv_slice(reader, b))??;
                    Ok(Value::Map(pairs))
                }
                V_STRUCT => {
                    let pairs = budget.descend(|b| decode_kv_slice(reader, b))??;
                    Ok(Value::Struct(pairs))
                }
                V_RANGE => {
                    let range =
                        budget.descend(|b| dol_core::ValueRange::decode(reader, b))??;
                    Ok(Value::Range(Box::new(range)))
                }
                V_EXTENSION => {
                    let name = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    let data = budget.descend(|b| Box::<[u8]>::decode(reader, b))??;
                    Ok(Value::Extension(Box::new((name, data))))
                }
                #[cfg(feature = "numeric")]
                V_DECIMAL => {
                    let d = budget.descend(|b| dol_core::Decimal::decode(reader, b))??;
                    Ok(Value::Decimal(Box::new(d)))
                }
                #[cfg(feature = "network")]
                V_INET => {
                    let ip = budget.descend(|b| dol_core::IpAddr::decode(reader, b))??;
                    Ok(Value::Inet(ip))
                }
                #[cfg(feature = "network")]
                V_MACADDR => {
                    let mac = budget.descend(|b| dol_core::MacAddr::decode(reader, b))??;
                    Ok(Value::MacAddr(mac))
                }
                #[cfg(feature = "datetime")]
                V_DATE => {
                    let d = budget.descend(|b| dol_core::Date::decode(reader, b))??;
                    Ok(Value::Date(d))
                }
                #[cfg(feature = "datetime")]
                V_TIME => {
                    let t = budget.descend(|b| dol_core::Time::decode(reader, b))??;
                    Ok(Value::Time(t))
                }
                #[cfg(feature = "datetime")]
                V_DATETIME => {
                    let dt = budget.descend(|b| dol_core::DateTime::decode(reader, b))??;
                    Ok(Value::DateTime(dt))
                }
                #[cfg(feature = "datetime")]
                V_TIMESTAMPTZ => {
                    let ts =
                        budget.descend(|b| dol_core::TimestampTz::decode(reader, b))??;
                    Ok(Value::TimestampTz(Box::new(ts)))
                }
                #[cfg(feature = "datetime")]
                V_INTERVAL => {
                    let iv = budget.descend(|b| dol_core::Interval::decode(reader, b))??;
                    Ok(Value::Interval(Box::new(iv)))
                }
                #[cfg(feature = "geo")]
                V_POINT => {
                    let p = budget.descend(|b| dol_core::Point::decode(reader, b))??;
                    Ok(Value::Point(p))
                }
                #[cfg(feature = "geo")]
                V_LINE => {
                    let l = budget.descend(|b| dol_core::geo::Line::decode(reader, b))??;
                    Ok(Value::Line(Box::new(l)))
                }
                #[cfg(feature = "geo")]
                V_SEGMENT => {
                    let s =
                        budget.descend(|b| dol_core::geo::Segment::decode(reader, b))??;
                    Ok(Value::Segment(Box::new(s)))
                }
                #[cfg(feature = "geo")]
                V_RECT => {
                    let r = budget.descend(|b| dol_core::geo::Rect::decode(reader, b))??;
                    Ok(Value::Rect(Box::new(r)))
                }
                #[cfg(feature = "geo")]
                V_CIRCLE => {
                    let c =
                        budget.descend(|b| dol_core::geo::Circle::decode(reader, b))??;
                    Ok(Value::Circle(Box::new(c)))
                }
                #[cfg(feature = "geo")]
                V_PATH => {
                    let path =
                        budget.descend(|b| dol_core::geo::Path::decode(reader, b))??;
                    Ok(Value::Path(Box::new(path)))
                }
                #[cfg(feature = "geo")]
                V_POLYGON => {
                    let poly =
                        budget.descend(|b| dol_core::geo::Polygon::decode(reader, b))??;
                    Ok(Value::Polygon(Box::new(poly)))
                }
                _ => Err(DecodeError::InvalidVariant {
                    type_name: "Value",
                    seen: disc,
                }),
            }
        }
    }
}

// ─── Literal<'static> and LiteralRange<'static> (stable custom discriminants)
//
// `Literal<'a>` contains `Cow<'a, str>` / `Cow<'a, [u8]>`. `Decode` always
// produces `'static` values by decoding strings/bytes into owned allocations.
//
// Discriminants mirror the `Value` scheme exactly:
//
//   Always-on primitives  :  0-20
//   Always-on composite   : 21-27
//   numeric feature       : 30
//   network feature       : 40-41
//   datetime feature      : 50-54
//   geo feature           : 60-66

#[allow(dead_code)]
mod literal_impls {
    use alloc::borrow::Cow;

    use super::*;
    use dol_core::literal::{Literal, LiteralRange};

    const L_NULL: u32 = 0;
    const L_BOOL: u32 = 1;
    const L_STRING: u32 = 2;
    const L_JSON: u32 = 3;
    const L_XML: u32 = 4;
    const L_ENUM: u32 = 5;
    const L_BYTES: u32 = 6;
    const L_UUID: u32 = 7;
    const L_BITSTRING: u32 = 8;
    const L_INT8: u32 = 9;
    const L_INT16: u32 = 10;
    const L_INT32: u32 = 11;
    const L_INT64: u32 = 12;
    const L_INT128: u32 = 13;
    const L_UINT8: u32 = 14;
    const L_UINT16: u32 = 15;
    const L_UINT32: u32 = 16;
    const L_UINT64: u32 = 17;
    const L_UINT128: u32 = 18;
    const L_FLOAT32: u32 = 19;
    const L_FLOAT64: u32 = 20;
    const L_ARRAY: u32 = 21;
    const L_SET: u32 = 22;
    const L_TUPLE: u32 = 23;
    const L_MAP: u32 = 24;
    const L_STRUCT: u32 = 25;
    const L_RANGE: u32 = 26;
    const L_EXTENSION: u32 = 27;
    #[cfg(feature = "numeric")]
    const L_DECIMAL: u32 = 30;
    #[cfg(feature = "network")]
    const L_INET: u32 = 40;
    #[cfg(feature = "network")]
    const L_MACADDR: u32 = 41;
    #[cfg(feature = "datetime")]
    const L_DATE: u32 = 50;
    #[cfg(feature = "datetime")]
    const L_TIME: u32 = 51;
    #[cfg(feature = "datetime")]
    const L_DATETIME: u32 = 52;
    #[cfg(feature = "datetime")]
    const L_TIMESTAMPTZ: u32 = 53;
    #[cfg(feature = "datetime")]
    const L_INTERVAL: u32 = 54;
    #[cfg(feature = "geo")]
    const L_POINT: u32 = 60;
    #[cfg(feature = "geo")]
    const L_LINE: u32 = 61;
    #[cfg(feature = "geo")]
    const L_SEGMENT: u32 = 62;
    #[cfg(feature = "geo")]
    const L_RECT: u32 = 63;
    #[cfg(feature = "geo")]
    const L_CIRCLE: u32 = 64;
    #[cfg(feature = "geo")]
    const L_PATH: u32 = 65;
    #[cfg(feature = "geo")]
    const L_POLYGON: u32 = 66;

    // Helper: decode a boxed slice of Literals.
    fn decode_lit_slice(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<Box<[Literal<'static>]>, DecodeError> {
        let v = Vec::<Literal<'static>>::decode(reader, budget)?;
        Ok(v.into_boxed_slice())
    }

    // Helper: decode boxed (Box<str>, Literal) key-value pairs.
    fn decode_kv_slice(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<Box<[(Cow<'static, str>, Literal<'static>)]>, DecodeError> {
        let len = reader.read_varint_u32()? as usize;
        if len > 1 << 20 {
            return Err(DecodeError::LengthOverflow);
        }
        let mut out: Vec<(Cow<'static, str>, Literal<'static>)> = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            let k = budget.descend(|b| Box::<str>::decode(reader, b))??;
            let v = budget.descend(|b| Literal::decode(reader, b))??;
            out.push((Cow::Owned(k.into_string()), v));
        }
        Ok(out.into_boxed_slice())
    }

    impl Decode for LiteralRange<'static> {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let start = budget.descend(|b| decode_bound(reader, b))??;
            let end = budget.descend(|b| decode_bound(reader, b))??;
            Ok(LiteralRange { start, end })
        }
    }

    fn decode_bound(
        reader: &mut Reader<'_>,
        budget: &mut Budget,
    ) -> Result<core::ops::Bound<Box<Literal<'static>>>, DecodeError> {
        match reader.read_u8()? {
            0 => {
                let v = budget.descend(|b| Literal::decode(reader, b))??;
                Ok(core::ops::Bound::Included(Box::new(v)))
            }
            1 => {
                let v = budget.descend(|b| Literal::decode(reader, b))??;
                Ok(core::ops::Bound::Excluded(Box::new(v)))
            }
            2 => Ok(core::ops::Bound::Unbounded),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "Bound<Literal>",
                seen: seen as u32,
            }),
        }
    }

    impl Decode for Literal<'static> {
        fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
            let disc = reader.read_varint_u32()?;
            match disc {
                L_NULL => Ok(Literal::Null),
                L_BOOL => Ok(Literal::Bool(bool::decode(reader, budget)?)),
                L_STRING => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Literal::String(Cow::Owned(s.into_string())))
                }
                L_JSON => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Literal::Json(Cow::Owned(s.into_string())))
                }
                L_XML => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Literal::Xml(Cow::Owned(s.into_string())))
                }
                L_ENUM => {
                    let s = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    Ok(Literal::Enum(Cow::Owned(s.into_string())))
                }
                L_BYTES => {
                    let b = budget.descend(|b| Box::<[u8]>::decode(reader, b))??;
                    Ok(Literal::Bytes(Cow::Owned(b.into_vec())))
                }
                L_UUID => Ok(Literal::Uuid(reader.read_array::<16>()?)),
                L_BITSTRING => {
                    let bs = budget.descend(|b| dol_core::BitString::decode(reader, b))??;
                    Ok(Literal::BitString(Box::new(bs)))
                }
                L_INT8 => Ok(Literal::Int8(i8::decode(reader, budget)?)),
                L_INT16 => Ok(Literal::Int16(i16::decode(reader, budget)?)),
                L_INT32 => Ok(Literal::Int32(i32::decode(reader, budget)?)),
                L_INT64 => Ok(Literal::Int64(i64::decode(reader, budget)?)),
                L_INT128 => Ok(Literal::Int128(i128::decode(reader, budget)?)),
                L_UINT8 => Ok(Literal::UInt8(u8::decode(reader, budget)?)),
                L_UINT16 => Ok(Literal::UInt16(u16::decode(reader, budget)?)),
                L_UINT32 => Ok(Literal::UInt32(u32::decode(reader, budget)?)),
                L_UINT64 => Ok(Literal::UInt64(u64::decode(reader, budget)?)),
                L_UINT128 => Ok(Literal::UInt128(u128::decode(reader, budget)?)),
                L_FLOAT32 => Ok(Literal::Float32(f32::decode(reader, budget)?)),
                L_FLOAT64 => Ok(Literal::Float64(f64::decode(reader, budget)?)),
                L_ARRAY => {
                    let elems = budget.descend(|b| decode_lit_slice(reader, b))??;
                    Ok(Literal::Array(elems))
                }
                L_SET => {
                    let elems = budget.descend(|b| decode_lit_slice(reader, b))??;
                    Ok(Literal::Set(elems))
                }
                L_TUPLE => {
                    let elems = budget.descend(|b| decode_lit_slice(reader, b))??;
                    Ok(Literal::Tuple(elems))
                }
                L_MAP => {
                    let pairs = budget.descend(|b| decode_kv_slice(reader, b))??;
                    Ok(Literal::Map(pairs))
                }
                L_STRUCT => {
                    let pairs = budget.descend(|b| decode_kv_slice(reader, b))??;
                    Ok(Literal::Struct(pairs))
                }
                L_RANGE => {
                    let r = budget
                        .descend(|b| LiteralRange::<'static>::decode(reader, b))??;
                    Ok(Literal::Range(Box::new(r)))
                }
                L_EXTENSION => {
                    let name = budget.descend(|b| Box::<str>::decode(reader, b))??;
                    let data = budget.descend(|b| Box::<[u8]>::decode(reader, b))??;
                    Ok(Literal::Extension(Box::new((name, data))))
                }
                #[cfg(feature = "numeric")]
                L_DECIMAL => {
                    let d = budget.descend(|b| dol_core::Decimal::decode(reader, b))??;
                    Ok(Literal::Decimal(Box::new(d)))
                }
                #[cfg(feature = "network")]
                L_INET => {
                    let ip = budget.descend(|b| dol_core::IpAddr::decode(reader, b))??;
                    Ok(Literal::Inet(ip))
                }
                #[cfg(feature = "network")]
                L_MACADDR => {
                    let mac = budget.descend(|b| dol_core::MacAddr::decode(reader, b))??;
                    Ok(Literal::MacAddr(mac))
                }
                #[cfg(feature = "datetime")]
                L_DATE => {
                    let d = budget.descend(|b| dol_core::Date::decode(reader, b))??;
                    Ok(Literal::Date(d))
                }
                #[cfg(feature = "datetime")]
                L_TIME => {
                    let t = budget.descend(|b| dol_core::Time::decode(reader, b))??;
                    Ok(Literal::Time(t))
                }
                #[cfg(feature = "datetime")]
                L_DATETIME => {
                    let dt = budget.descend(|b| dol_core::DateTime::decode(reader, b))??;
                    Ok(Literal::DateTime(dt))
                }
                #[cfg(feature = "datetime")]
                L_TIMESTAMPTZ => {
                    let ts =
                        budget.descend(|b| dol_core::TimestampTz::decode(reader, b))??;
                    Ok(Literal::TimestampTz(Box::new(ts)))
                }
                #[cfg(feature = "datetime")]
                L_INTERVAL => {
                    let iv = budget.descend(|b| dol_core::Interval::decode(reader, b))??;
                    Ok(Literal::Interval(Box::new(iv)))
                }
                #[cfg(feature = "geo")]
                L_POINT => {
                    let p = budget.descend(|b| dol_core::Point::decode(reader, b))??;
                    Ok(Literal::Point(p))
                }
                #[cfg(feature = "geo")]
                L_LINE => {
                    let l = budget.descend(|b| dol_core::geo::Line::decode(reader, b))??;
                    Ok(Literal::Line(Box::new(l)))
                }
                #[cfg(feature = "geo")]
                L_SEGMENT => {
                    let s =
                        budget.descend(|b| dol_core::geo::Segment::decode(reader, b))??;
                    Ok(Literal::Segment(Box::new(s)))
                }
                #[cfg(feature = "geo")]
                L_RECT => {
                    let r = budget.descend(|b| dol_core::geo::Rect::decode(reader, b))??;
                    Ok(Literal::Rect(Box::new(r)))
                }
                #[cfg(feature = "geo")]
                L_CIRCLE => {
                    let c =
                        budget.descend(|b| dol_core::geo::Circle::decode(reader, b))??;
                    Ok(Literal::Circle(Box::new(c)))
                }
                #[cfg(feature = "geo")]
                L_PATH => {
                    let path =
                        budget.descend(|b| dol_core::geo::Path::decode(reader, b))??;
                    Ok(Literal::Path(Box::new(path)))
                }
                #[cfg(feature = "geo")]
                L_POLYGON => {
                    let poly =
                        budget.descend(|b| dol_core::geo::Polygon::decode(reader, b))??;
                    Ok(Literal::Polygon(Box::new(poly)))
                }
                _ => Err(DecodeError::InvalidVariant {
                    type_name: "Literal",
                    seen: disc,
                }),
            }
        }
    }
}

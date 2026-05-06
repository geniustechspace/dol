//! [`Encode`] impls for the `dol-core` leaf type families.
//!
//! Each impl is byte-for-byte compatible with the postcard encoding
//! produced by the corresponding `serde::Serialize` derivation on the
//! same type, unless noted otherwise, mirroring the
//! [`Decode`](crate::decoder::Decode) impls in [`crate::decode_core`].
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
//!   `DataType`, `StructField`, `Value`, `ValueRange`. See
//!   [`crate::decode_core`] for the stable discriminant assignment.
//! - **Always-on (custom stable discriminants, works for any lifetime):**
//!   `Literal<'_>`, `LiteralRange<'_>`. Encode is generic over `'a`; same
//!   discriminant scheme as `Value`.

use alloc::boxed::Box;
use alloc::vec::Vec;

use dol_core::policy::Budget;

use crate::encoder::{Encode, EncodeError, Writer};

// ─── Always-on leaves ───────────────────────────────────────────────────────

impl Encode for dol_core::BitString {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        // Mirror of `Decode for BitString`: varint(len) + Box<[u8]> bytes.
        b.descend(|b| self.len.encode(w, b))??;
        b.descend(|b| <[u8] as Encode>::encode(&self.bytes, w, b))??;
        Ok(())
    }
}

impl Encode for dol_core::FileId {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        self.0.encode(w, b)
    }
}

// ─── Datetime leaves ────────────────────────────────────────────────────────

#[cfg(feature = "datetime")]
mod datetime_impls {
    use super::*;

    impl Encode for dol_core::Date {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.year.encode(w, b))??;
            b.descend(|b| self.month.encode(w, b))??;
            b.descend(|b| self.day.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::Time {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.hour.encode(w, b))??;
            b.descend(|b| self.minute.encode(w, b))??;
            b.descend(|b| self.second.encode(w, b))??;
            b.descend(|b| self.nanosecond.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::DateTime {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.date.encode(w, b))??;
            b.descend(|b| self.time.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::Offset {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            // `Offset(i32)` is a private tuple field; postcard's
            // `Serialize` derive on a single-field newtype emits the inner
            // value transparently. Use `as_seconds()` (the public
            // accessor used by `Decode`) to read out the i32.
            b.descend(|b| self.as_seconds().encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::TimestampTz {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.datetime.encode(w, b))??;
            b.descend(|b| self.offset.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::Interval {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.months.encode(w, b))??;
            b.descend(|b| self.days.encode(w, b))??;
            b.descend(|b| self.nanoseconds.encode(w, b))??;
            Ok(())
        }
    }
}

// ─── Numeric leaves ─────────────────────────────────────────────────────────

#[cfg(feature = "numeric")]
mod numeric_impls {
    use super::*;

    impl Encode for dol_core::Decimal {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.unscaled.encode(w, b))??;
            b.descend(|b| self.scale.encode(w, b))??;
            Ok(())
        }
    }
}

// ─── Geo leaves ─────────────────────────────────────────────────────────────

#[cfg(feature = "geo")]
mod geo_impls {
    use super::*;

    impl Encode for dol_core::Point {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.x.encode(w, b))??;
            b.descend(|b| self.y.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::geo::Line {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.a.encode(w, b))??;
            b.descend(|b| self.b.encode(w, b))??;
            b.descend(|b| self.c.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::geo::Segment {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.start.encode(w, b))??;
            b.descend(|b| self.end.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::geo::Rect {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.low.encode(w, b))??;
            b.descend(|b| self.high.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for dol_core::geo::Circle {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.center.encode(w, b))??;
            b.descend(|b| self.radius.encode(w, b))??;
            Ok(())
        }
    }
}


// ─── Span and SpanTable (always-on, postcard-compatible) ─────────────────────

impl Encode for dol_core::Span {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        // Postcard layout for `Span(u64)`: transparent newtype → varint u64.
        self.to_raw_u64().encode(w, b)
    }
}

impl Encode for dol_core::span::SpanTable {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        // Postcard layout: varint(len) followed by len `Span` values.
        let len: u32 = self
            .len()
            .try_into()
            .map_err(|_| EncodeError::LengthOverflow)?;
        w.write_varint_u32(len)?;
        for i in 0..len {
            b.descend(|b| self.get(i).encode(w, b))??;
        }
        Ok(())
    }
}

// ─── Network leaves (custom discriminant, NOT postcard-compatible) ────────────

#[cfg(feature = "network")]
mod network_impls {
    use super::*;

    impl Encode for dol_core::IpAddr {
        fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
            match self {
                dol_core::IpAddr::V4(bytes) => {
                    w.write_u8(0)?;
                    w.write_bytes(bytes)
                }
                dol_core::IpAddr::V6(bytes) => {
                    w.write_u8(1)?;
                    w.write_bytes(bytes)
                }
            }
        }
    }

    impl Encode for dol_core::MacAddr {
        fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
            match self {
                dol_core::MacAddr::Eui48(bytes) => {
                    w.write_u8(0)?;
                    w.write_bytes(bytes)
                }
                dol_core::MacAddr::Eui64(bytes) => {
                    w.write_u8(1)?;
                    w.write_bytes(bytes)
                }
            }
        }
    }
}

// ─── Geo compound leaves (postcard-compatible) ────────────────────────────────

#[cfg(feature = "geo")]
mod geo_compound_impls {
    use super::*;

    // Helper: encode a boxed slice of Points as a length-prefixed sequence.
    fn encode_points(
        pts: &[dol_core::Point],
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        let len: u32 = pts.len().try_into().map_err(|_| EncodeError::LengthOverflow)?;
        w.write_varint_u32(len)?;
        for p in pts.iter() {
            b.descend(|b| p.encode(w, b))??;
        }
        Ok(())
    }

    impl Encode for dol_core::geo::Path {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            // Postcard layout: bool (closed) + varint(len) + [Point…].
            b.descend(|b| self.closed.encode(w, b))??;
            encode_points(&self.points, w, b)
        }
    }

    impl Encode for dol_core::geo::Polygon {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            // Postcard layout: varint(len) + [Point…].
            encode_points(&self.points, w, b)
        }
    }
}

// ─── DataType and StructField (stable custom discriminants) ───────────────────

#[allow(dead_code)]
mod data_type_impls {
    use super::*;
    use dol_core::DataType;

    // Same discriminant constants as decode_core — kept in sync by the
    // `encode_core_roundtrip` self-round-trip tests.
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
    #[cfg(feature = "numeric")]
    const DT_DECIMAL: u32 = 30;
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
    #[cfg(feature = "network")]
    const DT_IPADDR: u32 = 50;
    #[cfg(feature = "network")]
    const DT_IPNETWORK: u32 = 51;
    #[cfg(feature = "network")]
    const DT_MACADDR: u32 = 52;
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

    impl Encode for dol_core::StructField {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| self.name.encode(w, b))??;
            b.descend(|b| self.data_type.encode(w, b))??;
            b.descend(|b| self.nullable.encode(w, b))??;
            Ok(())
        }
    }

    impl Encode for DataType {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            match self {
                DataType::Null => w.write_varint_u32(DT_NULL),
                DataType::Bool => w.write_varint_u32(DT_BOOL),
                DataType::Int8 => w.write_varint_u32(DT_INT8),
                DataType::Int16 => w.write_varint_u32(DT_INT16),
                DataType::Int32 => w.write_varint_u32(DT_INT32),
                DataType::Int64 => w.write_varint_u32(DT_INT64),
                DataType::Int128 => w.write_varint_u32(DT_INT128),
                DataType::UInt8 => w.write_varint_u32(DT_UINT8),
                DataType::UInt16 => w.write_varint_u32(DT_UINT16),
                DataType::UInt32 => w.write_varint_u32(DT_UINT32),
                DataType::UInt64 => w.write_varint_u32(DT_UINT64),
                DataType::UInt128 => w.write_varint_u32(DT_UINT128),
                DataType::Float32 => w.write_varint_u32(DT_FLOAT32),
                DataType::Float64 => w.write_varint_u32(DT_FLOAT64),
                DataType::String { max_len, fixed } => {
                    w.write_varint_u32(DT_STRING)?;
                    b.descend(|b| max_len.encode(w, b))??;
                    b.descend(|b| fixed.encode(w, b))??;
                    Ok(())
                }
                DataType::Json => w.write_varint_u32(DT_JSON),
                DataType::Xml => w.write_varint_u32(DT_XML),
                DataType::Bytes { max_len, fixed } => {
                    w.write_varint_u32(DT_BYTES)?;
                    b.descend(|b| max_len.encode(w, b))??;
                    b.descend(|b| fixed.encode(w, b))??;
                    Ok(())
                }
                DataType::Uuid => w.write_varint_u32(DT_UUID),
                DataType::BitString { max_len, fixed } => {
                    w.write_varint_u32(DT_BITSTRING)?;
                    b.descend(|b| max_len.encode(w, b))??;
                    b.descend(|b| fixed.encode(w, b))??;
                    Ok(())
                }
                DataType::Array(inner) => {
                    w.write_varint_u32(DT_ARRAY)?;
                    b.descend(|b| (**inner).encode(w, b))??;
                    Ok(())
                }
                DataType::Set(inner) => {
                    w.write_varint_u32(DT_SET)?;
                    b.descend(|b| (**inner).encode(w, b))??;
                    Ok(())
                }
                DataType::Map { value } => {
                    w.write_varint_u32(DT_MAP)?;
                    b.descend(|b| (**value).encode(w, b))??;
                    Ok(())
                }
                DataType::Range(inner) => {
                    w.write_varint_u32(DT_RANGE)?;
                    b.descend(|b| (**inner).encode(w, b))??;
                    Ok(())
                }
                DataType::Tuple(elems) => {
                    w.write_varint_u32(DT_TUPLE)?;
                    b.descend(|b| elems.encode(w, b))??;
                    Ok(())
                }
                DataType::Struct(fields) => {
                    w.write_varint_u32(DT_STRUCT)?;
                    b.descend(|b| fields.encode(w, b))??;
                    Ok(())
                }
                DataType::Enum(variants) => {
                    w.write_varint_u32(DT_ENUM)?;
                    b.descend(|b| variants.encode(w, b))??;
                    Ok(())
                }
                DataType::TypeRef(name) => {
                    w.write_varint_u32(DT_TYPEREF)?;
                    b.descend(|b| name.encode(w, b))??;
                    Ok(())
                }
                DataType::Extension { name, params } => {
                    w.write_varint_u32(DT_EXTENSION)?;
                    b.descend(|b| name.encode(w, b))??;
                    b.descend(|b| params.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "numeric")]
                DataType::Decimal { precision, scale } => {
                    w.write_varint_u32(DT_DECIMAL)?;
                    b.descend(|b| precision.encode(w, b))??;
                    b.descend(|b| scale.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                DataType::Date => w.write_varint_u32(DT_DATE),
                #[cfg(feature = "datetime")]
                DataType::Time { precision } => {
                    w.write_varint_u32(DT_TIME)?;
                    b.descend(|b| precision.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                DataType::DateTime { precision } => {
                    w.write_varint_u32(DT_DATETIME)?;
                    b.descend(|b| precision.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                DataType::OffsetDateTime { precision } => {
                    w.write_varint_u32(DT_OFFSET_DATETIME)?;
                    b.descend(|b| precision.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                DataType::Interval => w.write_varint_u32(DT_INTERVAL),
                #[cfg(feature = "network")]
                DataType::IpAddr => w.write_varint_u32(DT_IPADDR),
                #[cfg(feature = "network")]
                DataType::IpNetwork { prefix_len } => {
                    w.write_varint_u32(DT_IPNETWORK)?;
                    b.descend(|b| prefix_len.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "network")]
                DataType::MacAddr => w.write_varint_u32(DT_MACADDR),
                #[cfg(feature = "geo")]
                DataType::Point => w.write_varint_u32(DT_POINT),
                #[cfg(feature = "geo")]
                DataType::Line => w.write_varint_u32(DT_LINE),
                #[cfg(feature = "geo")]
                DataType::LineSegment => w.write_varint_u32(DT_LINESEGMENT),
                #[cfg(feature = "geo")]
                DataType::Rect => w.write_varint_u32(DT_RECT),
                #[cfg(feature = "geo")]
                DataType::Circle => w.write_varint_u32(DT_CIRCLE),
                #[cfg(feature = "geo")]
                DataType::Path => w.write_varint_u32(DT_PATH),
                #[cfg(feature = "geo")]
                DataType::Polygon => w.write_varint_u32(DT_POLYGON),
                _ => Err(EncodeError::Custom("DataType: unknown variant")),
            }
        }
    }
}

// ─── Value and ValueRange (stable custom discriminants) ───────────────────────

#[allow(dead_code)]
mod value_impls {
    use super::*;
    use core::ops::Bound;
    use dol_core::Value;

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
    #[cfg(feature = "numeric")]
    const V_DECIMAL: u32 = 30;
    #[cfg(feature = "network")]
    const V_INET: u32 = 40;
    #[cfg(feature = "network")]
    const V_MACADDR: u32 = 41;
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
    fn encode_bound(
        bound: &Bound<Box<Value>>,
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        match bound {
            Bound::Included(v) => {
                w.write_u8(0)?;
                b.descend(|b| (**v).encode(w, b))??;
            }
            Bound::Excluded(v) => {
                w.write_u8(1)?;
                b.descend(|b| (**v).encode(w, b))??;
            }
            Bound::Unbounded => w.write_u8(2)?,
        }
        Ok(())
    }

    // Helper: encode a boxed slice of Values as a length-prefixed sequence.
    fn encode_value_slice(
        slice: &[Value],
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        let len: u32 = slice.len().try_into().map_err(|_| EncodeError::LengthOverflow)?;
        w.write_varint_u32(len)?;
        for item in slice.iter() {
            b.descend(|b| item.encode(w, b))??;
        }
        Ok(())
    }

    // Helper: encode a boxed slice of (Box<str>, Value) pairs.
    fn encode_kv_slice(
        slice: &[(Box<str>, Value)],
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        let len: u32 = slice.len().try_into().map_err(|_| EncodeError::LengthOverflow)?;
        w.write_varint_u32(len)?;
        for (k, v) in slice.iter() {
            b.descend(|b| k.encode(w, b))??;
            b.descend(|b| v.encode(w, b))??;
        }
        Ok(())
    }

    impl Encode for dol_core::ValueRange {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| encode_bound(&self.start, w, b))??;
            b.descend(|b| encode_bound(&self.end, w, b))??;
            Ok(())
        }
    }

    impl Encode for Value {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            match self {
                Value::Null => w.write_varint_u32(V_NULL),
                Value::Bool(v) => {
                    w.write_varint_u32(V_BOOL)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::String(s) => {
                    w.write_varint_u32(V_STRING)?;
                    b.descend(|b| s.encode(w, b))??;
                    Ok(())
                }
                Value::Json(s) => {
                    w.write_varint_u32(V_JSON)?;
                    b.descend(|b| s.encode(w, b))??;
                    Ok(())
                }
                Value::Xml(s) => {
                    w.write_varint_u32(V_XML)?;
                    b.descend(|b| s.encode(w, b))??;
                    Ok(())
                }
                Value::Enum(s) => {
                    w.write_varint_u32(V_ENUM)?;
                    b.descend(|b| s.encode(w, b))??;
                    Ok(())
                }
                Value::Bytes(bytes) => {
                    w.write_varint_u32(V_BYTES)?;
                    b.descend(|b| <[u8] as crate::encoder::Encode>::encode(bytes, w, b))??;
                    Ok(())
                }
                Value::Uuid(u) => {
                    w.write_varint_u32(V_UUID)?;
                    w.write_bytes(u)
                }
                Value::BitString(bs) => {
                    w.write_varint_u32(V_BITSTRING)?;
                    b.descend(|b| bs.as_ref().encode(w, b))??;
                    Ok(())
                }
                Value::Int8(v) => {
                    w.write_varint_u32(V_INT8)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::Int16(v) => {
                    w.write_varint_u32(V_INT16)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::Int32(v) => {
                    w.write_varint_u32(V_INT32)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::Int64(v) => {
                    w.write_varint_u32(V_INT64)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::Int128(v) => {
                    w.write_varint_u32(V_INT128)?;
                    b.descend(|b| (**v).encode(w, b))??;
                    Ok(())
                }
                Value::UInt8(v) => {
                    w.write_varint_u32(V_UINT8)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::UInt16(v) => {
                    w.write_varint_u32(V_UINT16)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::UInt32(v) => {
                    w.write_varint_u32(V_UINT32)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::UInt64(v) => {
                    w.write_varint_u32(V_UINT64)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::UInt128(v) => {
                    w.write_varint_u32(V_UINT128)?;
                    b.descend(|b| (**v).encode(w, b))??;
                    Ok(())
                }
                Value::Float32(v) => {
                    w.write_varint_u32(V_FLOAT32)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::Float64(v) => {
                    w.write_varint_u32(V_FLOAT64)?;
                    b.descend(|b| v.encode(w, b))??;
                    Ok(())
                }
                Value::Array(elems) => {
                    w.write_varint_u32(V_ARRAY)?;
                    b.descend(|b| encode_value_slice(elems, w, b))??;
                    Ok(())
                }
                Value::Set(elems) => {
                    w.write_varint_u32(V_SET)?;
                    b.descend(|b| encode_value_slice(elems, w, b))??;
                    Ok(())
                }
                Value::Tuple(elems) => {
                    w.write_varint_u32(V_TUPLE)?;
                    b.descend(|b| encode_value_slice(elems, w, b))??;
                    Ok(())
                }
                Value::Map(pairs) => {
                    w.write_varint_u32(V_MAP)?;
                    b.descend(|b| encode_kv_slice(pairs, w, b))??;
                    Ok(())
                }
                Value::Struct(pairs) => {
                    w.write_varint_u32(V_STRUCT)?;
                    b.descend(|b| encode_kv_slice(pairs, w, b))??;
                    Ok(())
                }
                Value::Range(range) => {
                    w.write_varint_u32(V_RANGE)?;
                    b.descend(|b| (**range).encode(w, b))??;
                    Ok(())
                }
                Value::Extension(inner) => {
                    w.write_varint_u32(V_EXTENSION)?;
                    b.descend(|b| inner.0.encode(w, b))??;
                    b.descend(|b| <[u8] as crate::encoder::Encode>::encode(&inner.1, w, b))??;
                    Ok(())
                }
                #[cfg(feature = "numeric")]
                Value::Decimal(d) => {
                    w.write_varint_u32(V_DECIMAL)?;
                    b.descend(|b| d.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "network")]
                Value::Inet(ip) => {
                    w.write_varint_u32(V_INET)?;
                    b.descend(|b| ip.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "network")]
                Value::MacAddr(mac) => {
                    w.write_varint_u32(V_MACADDR)?;
                    b.descend(|b| mac.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Value::Date(d) => {
                    w.write_varint_u32(V_DATE)?;
                    b.descend(|b| d.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Value::Time(t) => {
                    w.write_varint_u32(V_TIME)?;
                    b.descend(|b| t.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Value::DateTime(dt) => {
                    w.write_varint_u32(V_DATETIME)?;
                    b.descend(|b| dt.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Value::TimestampTz(ts) => {
                    w.write_varint_u32(V_TIMESTAMPTZ)?;
                    b.descend(|b| ts.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Value::Interval(iv) => {
                    w.write_varint_u32(V_INTERVAL)?;
                    b.descend(|b| iv.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Point(p) => {
                    w.write_varint_u32(V_POINT)?;
                    b.descend(|b| p.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Line(l) => {
                    w.write_varint_u32(V_LINE)?;
                    b.descend(|b| l.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Segment(s) => {
                    w.write_varint_u32(V_SEGMENT)?;
                    b.descend(|b| s.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Rect(r) => {
                    w.write_varint_u32(V_RECT)?;
                    b.descend(|b| r.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Circle(c) => {
                    w.write_varint_u32(V_CIRCLE)?;
                    b.descend(|b| c.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Path(path) => {
                    w.write_varint_u32(V_PATH)?;
                    b.descend(|b| path.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Value::Polygon(poly) => {
                    w.write_varint_u32(V_POLYGON)?;
                    b.descend(|b| poly.as_ref().encode(w, b))??;
                    Ok(())
                }
                _ => Err(EncodeError::Custom("Value: unknown variant")),
            }
        }
    }
}

// ─── Literal<'_> and LiteralRange<'_> (stable custom discriminants) ───────────
//
// `Encode` is generic over `'a` — it works for both borrowed and owned
// literals since it only reads the payload. Discriminants are identical to
// the `Value` / `Decode` scheme in decode_core.rs.

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

    fn encode_lit_slice<'a>(
        items: &[Literal<'a>],
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        w.write_varint_u32(items.len() as u32)?;
        for item in items {
            b.descend(|b| item.encode(w, b))??;
        }
        Ok(())
    }

    fn encode_kv_slice<'a>(
        pairs: &[(Cow<'a, str>, Literal<'a>)],
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        w.write_varint_u32(pairs.len() as u32)?;
        for (k, v) in pairs {
            b.descend(|b| k.as_ref().encode(w, b))??;
            b.descend(|b| v.encode(w, b))??;
        }
        Ok(())
    }

    fn encode_bound<'a>(
        bound: &core::ops::Bound<Box<Literal<'a>>>,
        w: &mut Writer<'_>,
        b: &mut Budget,
    ) -> Result<(), EncodeError> {
        match bound {
            core::ops::Bound::Included(v) => {
                w.write_u8(0)?;
                b.descend(|b| v.as_ref().encode(w, b))??;
            }
            core::ops::Bound::Excluded(v) => {
                w.write_u8(1)?;
                b.descend(|b| v.as_ref().encode(w, b))??;
            }
            core::ops::Bound::Unbounded => w.write_u8(2)?,
        }
        Ok(())
    }

    impl<'a> Encode for LiteralRange<'a> {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            b.descend(|b| encode_bound(&self.start, w, b))??;
            b.descend(|b| encode_bound(&self.end, w, b))??;
            Ok(())
        }
    }

    impl<'a> Encode for Literal<'a> {
        fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
            match self {
                Literal::Null => w.write_varint_u32(L_NULL),
                Literal::Bool(v) => {
                    w.write_varint_u32(L_BOOL)?;
                    v.encode(w, b)
                }
                Literal::String(s) => {
                    w.write_varint_u32(L_STRING)?;
                    b.descend(|b| s.as_ref().encode(w, b))??;
                    Ok(())
                }
                Literal::Json(s) => {
                    w.write_varint_u32(L_JSON)?;
                    b.descend(|b| s.as_ref().encode(w, b))??;
                    Ok(())
                }
                Literal::Xml(s) => {
                    w.write_varint_u32(L_XML)?;
                    b.descend(|b| s.as_ref().encode(w, b))??;
                    Ok(())
                }
                Literal::Enum(s) => {
                    w.write_varint_u32(L_ENUM)?;
                    b.descend(|b| s.as_ref().encode(w, b))??;
                    Ok(())
                }
                Literal::Bytes(bs) => {
                    w.write_varint_u32(L_BYTES)?;
                    b.descend(|b| <[u8] as Encode>::encode(bs.as_ref(), w, b))??;
                    Ok(())
                }
                Literal::Uuid(uuid) => {
                    w.write_varint_u32(L_UUID)?;
                    w.write_bytes(uuid)
                }
                Literal::BitString(bs) => {
                    w.write_varint_u32(L_BITSTRING)?;
                    b.descend(|b| bs.as_ref().encode(w, b))??;
                    Ok(())
                }
                Literal::Int8(v) => {
                    w.write_varint_u32(L_INT8)?;
                    v.encode(w, b)
                }
                Literal::Int16(v) => {
                    w.write_varint_u32(L_INT16)?;
                    v.encode(w, b)
                }
                Literal::Int32(v) => {
                    w.write_varint_u32(L_INT32)?;
                    v.encode(w, b)
                }
                Literal::Int64(v) => {
                    w.write_varint_u32(L_INT64)?;
                    v.encode(w, b)
                }
                Literal::Int128(v) => {
                    w.write_varint_u32(L_INT128)?;
                    v.encode(w, b)
                }
                Literal::UInt8(v) => {
                    w.write_varint_u32(L_UINT8)?;
                    v.encode(w, b)
                }
                Literal::UInt16(v) => {
                    w.write_varint_u32(L_UINT16)?;
                    v.encode(w, b)
                }
                Literal::UInt32(v) => {
                    w.write_varint_u32(L_UINT32)?;
                    v.encode(w, b)
                }
                Literal::UInt64(v) => {
                    w.write_varint_u32(L_UINT64)?;
                    v.encode(w, b)
                }
                Literal::UInt128(v) => {
                    w.write_varint_u32(L_UINT128)?;
                    v.encode(w, b)
                }
                Literal::Float32(v) => {
                    w.write_varint_u32(L_FLOAT32)?;
                    v.encode(w, b)
                }
                Literal::Float64(v) => {
                    w.write_varint_u32(L_FLOAT64)?;
                    v.encode(w, b)
                }
                Literal::Array(elems) => {
                    w.write_varint_u32(L_ARRAY)?;
                    b.descend(|b| encode_lit_slice(elems, w, b))??;
                    Ok(())
                }
                Literal::Set(elems) => {
                    w.write_varint_u32(L_SET)?;
                    b.descend(|b| encode_lit_slice(elems, w, b))??;
                    Ok(())
                }
                Literal::Tuple(elems) => {
                    w.write_varint_u32(L_TUPLE)?;
                    b.descend(|b| encode_lit_slice(elems, w, b))??;
                    Ok(())
                }
                Literal::Map(pairs) => {
                    w.write_varint_u32(L_MAP)?;
                    b.descend(|b| encode_kv_slice(pairs, w, b))??;
                    Ok(())
                }
                Literal::Struct(pairs) => {
                    w.write_varint_u32(L_STRUCT)?;
                    b.descend(|b| encode_kv_slice(pairs, w, b))??;
                    Ok(())
                }
                Literal::Range(range) => {
                    w.write_varint_u32(L_RANGE)?;
                    b.descend(|b| range.as_ref().encode(w, b))??;
                    Ok(())
                }
                Literal::Extension(inner) => {
                    w.write_varint_u32(L_EXTENSION)?;
                    b.descend(|b| inner.0.encode(w, b))??;
                    b.descend(|b| <[u8] as Encode>::encode(&inner.1, w, b))??;
                    Ok(())
                }
                #[cfg(feature = "numeric")]
                Literal::Decimal(d) => {
                    w.write_varint_u32(L_DECIMAL)?;
                    b.descend(|b| d.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "network")]
                Literal::Inet(ip) => {
                    w.write_varint_u32(L_INET)?;
                    b.descend(|b| ip.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "network")]
                Literal::MacAddr(mac) => {
                    w.write_varint_u32(L_MACADDR)?;
                    b.descend(|b| mac.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Literal::Date(d) => {
                    w.write_varint_u32(L_DATE)?;
                    b.descend(|b| d.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Literal::Time(t) => {
                    w.write_varint_u32(L_TIME)?;
                    b.descend(|b| t.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Literal::DateTime(dt) => {
                    w.write_varint_u32(L_DATETIME)?;
                    b.descend(|b| dt.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Literal::TimestampTz(ts) => {
                    w.write_varint_u32(L_TIMESTAMPTZ)?;
                    b.descend(|b| ts.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "datetime")]
                Literal::Interval(iv) => {
                    w.write_varint_u32(L_INTERVAL)?;
                    b.descend(|b| iv.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Point(p) => {
                    w.write_varint_u32(L_POINT)?;
                    b.descend(|b| p.encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Line(l) => {
                    w.write_varint_u32(L_LINE)?;
                    b.descend(|b| l.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Segment(s) => {
                    w.write_varint_u32(L_SEGMENT)?;
                    b.descend(|b| s.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Rect(r) => {
                    w.write_varint_u32(L_RECT)?;
                    b.descend(|b| r.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Circle(c) => {
                    w.write_varint_u32(L_CIRCLE)?;
                    b.descend(|b| c.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Path(path) => {
                    w.write_varint_u32(L_PATH)?;
                    b.descend(|b| path.as_ref().encode(w, b))??;
                    Ok(())
                }
                #[cfg(feature = "geo")]
                Literal::Polygon(poly) => {
                    w.write_varint_u32(L_POLYGON)?;
                    b.descend(|b| poly.as_ref().encode(w, b))??;
                    Ok(())
                }
                _ => Err(EncodeError::Custom("Literal: unknown variant")),
            }
        }
    }
}

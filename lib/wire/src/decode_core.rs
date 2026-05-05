//! [`Decode`] impls for the `dol-core` leaf type families.
//!
//! Each impl is byte-for-byte compatible with the postcard encoding
//! produced by the corresponding `serde::Serialize` derivation on the same
//! type. The `decode_core_roundtrip` integration test asserts this against
//! `postcard::to_allocvec(&value)` for every covered type.
//!
//! Coverage in v2 (0.2.0):
//!
//! - **Always-on:** `BitString`, `FileId`.
//! - **`datetime` feature:** `Date`, `Time`, `DateTime`, `Offset`,
//!   `TimestampTz`, `Interval`.
//! - **`numeric` feature:** `Decimal`.
//! - **`geo` feature:** `Point`, `Line`, `Segment`, `Rect`, `Circle`.
//!
//! Deliberately deferred to the next focused PR:
//!
//! - **Recursive enums** in dol-core: `Value`, `Literal<'a>`,
//!   `LiteralRange<'a>`, `ValueRange`, `EnumDef`, `DataType`,
//!   `StructField` (depends on `DataType`).
//! - **`serde(untagged)` types** (`IpAddr`, `MacAddr`, `Path`): the
//!   in-crate `serde::Deserialize` for these returns `WontImplement` from
//!   postcard, so they cannot share the postcard byte format. They will
//!   gain hand-written `Decode`/`Encode` pair impls with an explicit
//!   discriminant byte once the `Encode` trait lands.
//! - **`Span` / `SpanTable`:** the `Span(u64)` newtype hides its inner
//!   field; reconstructing from raw bits needs a `Span::from_raw` accessor
//!   that doesn't exist yet.

extern crate alloc;

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

// Silence "unused" warnings when the optional features aren't enabled.
#[allow(dead_code)]
fn _force_alloc_use() -> Vec<u8> {
    Vec::new()
}

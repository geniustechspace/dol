//! [`Encode`] impls for the `dol-core` leaf type families.
//!
//! Each impl is byte-for-byte compatible with the postcard encoding
//! produced by the corresponding `serde::Serialize` derivation on the
//! same type, mirroring the [`Decode`](crate::decoder::Decode) impls in
//! [`crate::decode_core`]. The `encode_core_roundtrip` integration test
//! asserts:
//!
//! - `encode_to_vec(&v) == postcard::to_allocvec(&v)` (postcard parity), and
//! - `decode(encode(v)) == v` (self round-trip via the v2 traits alone).
//!
//! Coverage in v2 (0.2.0):
//!
//! - **Always-on:** `BitString`, `FileId`.
//! - **`datetime` feature:** `Date`, `Time`, `DateTime`, `Offset`,
//!   `TimestampTz`, `Interval`.
//! - **`numeric` feature:** `Decimal`.
//! - **`geo` feature:** `Point`, `Line`, `Segment`, `Rect`, `Circle`.
//!
//! The remaining types deferred by [`crate::decode_core`] (recursive
//! enums, `serde(untagged)` leaves, `Span` / `SpanTable`) are also
//! deferred here and gain symmetric `Encode` / `Decode` pair impls in the
//! follow-up PR.

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

// Silence "unused" warnings when the optional features aren't enabled.
#[allow(dead_code)]
fn _force_alloc_use() -> Vec<u8> {
    Vec::new()
}

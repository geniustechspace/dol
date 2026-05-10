//! [`Internable`](crate::intern::Internable) impls for [`Literal`] and
//! its leaf-type building blocks ([`BitString`], [`Decimal`], the
//! `datetime`/`network`/`geo` POD types).
//!
//! See [`crate::intern`] for the contract. This file owns the canonical
//! byte form of every [`Literal`] variant — adding a new variant
//! requires extending the `match` here so the compiler refuses to build
//! a `Literal` whose hash is undefined.
//!
//! # Canonicalisation rules
//!
//! * Each variant is preceded by a unique tag byte so variants of the
//!   same shape cannot collide (e.g. `Int8(0)` ≠ `Int16(0)`).
//! * Floating-point variants normalise `-0.0 → 0.0` (so the two
//!   IEEE-754 representations of zero share an id) and any NaN payload
//!   to a single canonical NaN bit-pattern (so two structurally-equal
//!   `f64::NAN` literals share an id).
//! * `String`, `Json`, `Xml`, `Enum`, `Bytes` feed a length prefix
//!   ahead of the payload to prevent the classic `"ab" + "c"` vs
//!   `"a" + "bc"` framing collision.
//! * Composite variants (`Array`, `Set`, `Tuple`, `Map`, `Struct`,
//!   `Range`) recurse through child literals **in source order**.
//!   Set and Map ordering is preserved as-given — DOL treats `Set`
//!   ordering as semantically meaningful at the literal level (the
//!   *value*-side `Value::Set` does the multiset normalisation).
//! * Nested `Internable` children feed themselves via `child.feed(h)`
//!   rather than being re-tagged here, so the per-type canonical form
//!   stays in one place.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use core::ops::Bound;

use crate::binary::BitString;
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
use crate::hash::Hasher;
use crate::intern::Internable;
use crate::literal::{Literal, LiteralRange};
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;

// Tag byte assignments. A new variant gets a fresh tag; never reuse.
// Kept in one block so reviewers can spot a duplicate by eye.
const T_NULL: u8 = 0x00;
const T_BOOL: u8 = 0x01;
const T_STRING: u8 = 0x02;
const T_JSON: u8 = 0x03;
const T_XML: u8 = 0x04;
const T_ENUM: u8 = 0x05;
const T_BYTES: u8 = 0x06;
const T_UUID: u8 = 0x07;
const T_BITSTRING: u8 = 0x08;
const T_INT8: u8 = 0x10;
const T_INT16: u8 = 0x11;
const T_INT32: u8 = 0x12;
const T_INT64: u8 = 0x13;
const T_INT128: u8 = 0x14;
const T_UINT8: u8 = 0x15;
const T_UINT16: u8 = 0x16;
const T_UINT32: u8 = 0x17;
const T_UINT64: u8 = 0x18;
const T_UINT128: u8 = 0x19;
const T_FLOAT32: u8 = 0x20;
const T_FLOAT64: u8 = 0x21;
#[cfg(feature = "numeric")]
const T_DECIMAL: u8 = 0x22;
#[cfg(feature = "network")]
const T_INET: u8 = 0x30;
#[cfg(feature = "network")]
const T_MACADDR: u8 = 0x31;
#[cfg(feature = "datetime")]
const T_DATE: u8 = 0x40;
#[cfg(feature = "datetime")]
const T_TIME: u8 = 0x41;
#[cfg(feature = "datetime")]
const T_DATETIME: u8 = 0x42;
#[cfg(feature = "datetime")]
const T_TIMESTAMPTZ: u8 = 0x43;
#[cfg(feature = "datetime")]
const T_INTERVAL: u8 = 0x44;
#[cfg(feature = "geo")]
const T_POINT: u8 = 0x50;
#[cfg(feature = "geo")]
const T_LINE: u8 = 0x51;
#[cfg(feature = "geo")]
const T_SEGMENT: u8 = 0x52;
#[cfg(feature = "geo")]
const T_RECT: u8 = 0x53;
#[cfg(feature = "geo")]
const T_CIRCLE: u8 = 0x54;
#[cfg(feature = "geo")]
const T_PATH: u8 = 0x55;
#[cfg(feature = "geo")]
const T_POLYGON: u8 = 0x56;
const T_ARRAY: u8 = 0x60;
const T_SET: u8 = 0x61;
const T_TUPLE: u8 = 0x62;
const T_MAP: u8 = 0x63;
const T_STRUCT: u8 = 0x64;
const T_RANGE: u8 = 0x65;
const T_EXTENSION: u8 = 0x70;

// Range-bound discriminators for `feed_range_bound`.
const B_UNBOUNDED: u8 = 0x00;
const B_INCLUDED: u8 = 0x01;
const B_EXCLUDED: u8 = 0x02;

// IpAddr / MacAddr enum-arm discriminators (kept here, not re-using the
// `T_*` namespace, so a future `Literal::Foo(IpAddr)` can prepend its
// own tag without ambiguity).
#[cfg(feature = "network")]
const IP_V4: u8 = 0x01;
#[cfg(feature = "network")]
const IP_V6: u8 = 0x02;
#[cfg(feature = "network")]
const MAC_EUI48: u8 = 0x01;
#[cfg(feature = "network")]
const MAC_EUI64: u8 = 0x02;

#[inline]
fn feed_len(h: &mut Hasher, len: usize) {
    h.update(&(len as u64).to_le_bytes());
}

#[inline]
fn feed_str(h: &mut Hasher, tag: u8, s: &str) {
    h.update(&[tag]);
    feed_len(h, s.len());
    h.update(s.as_bytes());
}

#[inline]
fn feed_bytes(h: &mut Hasher, tag: u8, b: &[u8]) {
    h.update(&[tag]);
    feed_len(h, b.len());
    h.update(b);
}

/// Canonicalise an `f32` into bytes that fold `-0.0 → 0.0` and any NaN
/// payload onto a single representative NaN.
#[inline]
fn canonical_f32_bits(x: f32) -> [u8; 4] {
    if x.is_nan() {
        f32::NAN.to_bits().to_le_bytes()
    } else if x == 0.0 {
        // collapses both `0.0` and `-0.0`
        0u32.to_le_bytes()
    } else {
        x.to_bits().to_le_bytes()
    }
}

/// Canonicalise an `f64` (see [`canonical_f32_bits`]).
#[inline]
fn canonical_f64_bits(x: f64) -> [u8; 8] {
    if x.is_nan() {
        f64::NAN.to_bits().to_le_bytes()
    } else if x == 0.0 {
        0u64.to_le_bytes()
    } else {
        x.to_bits().to_le_bytes()
    }
}

fn feed_seq(h: &mut Hasher, tag: u8, items: &[Literal<'_>]) {
    h.update(&[tag]);
    feed_len(h, items.len());
    for item in items {
        item.feed(h);
    }
}

fn feed_keyed<'a>(h: &mut Hasher, tag: u8, items: &[(Cow<'a, str>, Literal<'a>)]) {
    h.update(&[tag]);
    feed_len(h, items.len());
    for (k, v) in items {
        feed_len(h, k.len());
        h.update(k.as_bytes());
        v.feed(h);
    }
}

fn feed_range_bound(h: &mut Hasher, b: &Bound<Box<Literal<'_>>>) {
    match b {
        Bound::Unbounded => {
            h.update(&[B_UNBOUNDED]);
        }
        Bound::Included(v) => {
            h.update(&[B_INCLUDED]);
            v.feed(h);
        }
        Bound::Excluded(v) => {
            h.update(&[B_EXCLUDED]);
            v.feed(h);
        }
    }
}

fn feed_range(h: &mut Hasher, r: &LiteralRange<'_>) {
    feed_range_bound(h, &r.start);
    feed_range_bound(h, &r.end);
}

// ─── Leaf-type Internable impls ──────────────────────────────────────────────
//
// Each leaf type owns its canonical byte form so `Literal::feed` can
// recurse via `child.feed(h)`. Where the type has multiple variants
// (IpAddr, MacAddr) we feed a discriminator byte; for scalar structs we
// just feed each field in declaration order.

impl Internable for BitString {
    fn feed(&self, h: &mut Hasher) {
        h.update(&self.len.to_le_bytes());
        feed_len(h, self.bytes.len());
        h.update(&self.bytes);
    }
}

#[cfg(feature = "numeric")]
impl Internable for Decimal {
    fn feed(&self, h: &mut Hasher) {
        h.update(&self.unscaled.to_le_bytes());
        h.update(&self.scale.to_le_bytes());
    }
}

#[cfg(feature = "network")]
impl Internable for IpAddr {
    fn feed(&self, h: &mut Hasher) {
        match self {
            IpAddr::V4(b) => {
                h.update(&[IP_V4]);
                h.update(b);
            }
            IpAddr::V6(b) => {
                h.update(&[IP_V6]);
                h.update(b);
            }
        }
    }
}

#[cfg(feature = "network")]
impl Internable for MacAddr {
    fn feed(&self, h: &mut Hasher) {
        match self {
            MacAddr::Eui48(b) => {
                h.update(&[MAC_EUI48]);
                h.update(b);
            }
            MacAddr::Eui64(b) => {
                h.update(&[MAC_EUI64]);
                h.update(b);
            }
        }
    }
}

#[cfg(feature = "datetime")]
impl Internable for Date {
    fn feed(&self, h: &mut Hasher) {
        h.update(&self.year.to_le_bytes());
        h.update(&[self.month, self.day]);
    }
}

#[cfg(feature = "datetime")]
impl Internable for Time {
    fn feed(&self, h: &mut Hasher) {
        h.update(&[self.hour, self.minute, self.second]);
        h.update(&self.nanosecond.to_le_bytes());
    }
}

#[cfg(feature = "datetime")]
impl Internable for DateTime {
    fn feed(&self, h: &mut Hasher) {
        self.date.feed(h);
        self.time.feed(h);
    }
}

#[cfg(feature = "datetime")]
impl Internable for Offset {
    fn feed(&self, h: &mut Hasher) {
        h.update(&self.as_seconds().to_le_bytes());
    }
}

#[cfg(feature = "datetime")]
impl Internable for TimestampTz {
    fn feed(&self, h: &mut Hasher) {
        self.datetime.feed(h);
        self.offset.feed(h);
    }
}

#[cfg(feature = "datetime")]
impl Internable for Interval {
    fn feed(&self, h: &mut Hasher) {
        h.update(&self.months.to_le_bytes());
        h.update(&self.days.to_le_bytes());
        h.update(&self.nanoseconds.to_le_bytes());
    }
}

#[cfg(feature = "geo")]
impl Internable for Point {
    fn feed(&self, h: &mut Hasher) {
        h.update(&canonical_f64_bits(self.x));
        h.update(&canonical_f64_bits(self.y));
    }
}

#[cfg(feature = "geo")]
impl Internable for Line {
    fn feed(&self, h: &mut Hasher) {
        h.update(&canonical_f64_bits(self.a));
        h.update(&canonical_f64_bits(self.b));
        h.update(&canonical_f64_bits(self.c));
    }
}

#[cfg(feature = "geo")]
impl Internable for Segment {
    fn feed(&self, h: &mut Hasher) {
        self.start.feed(h);
        self.end.feed(h);
    }
}

#[cfg(feature = "geo")]
impl Internable for Rect {
    fn feed(&self, h: &mut Hasher) {
        self.low.feed(h);
        self.high.feed(h);
    }
}

#[cfg(feature = "geo")]
impl Internable for Circle {
    fn feed(&self, h: &mut Hasher) {
        self.center.feed(h);
        h.update(&canonical_f64_bits(self.radius));
    }
}

#[cfg(feature = "geo")]
impl Internable for Path {
    fn feed(&self, h: &mut Hasher) {
        h.update(&[u8::from(self.closed)]);
        feed_len(h, self.points.len());
        for p in self.points.iter() {
            p.feed(h);
        }
    }
}

#[cfg(feature = "geo")]
impl Internable for Polygon {
    fn feed(&self, h: &mut Hasher) {
        feed_len(h, self.points.len());
        for p in self.points.iter() {
            p.feed(h);
        }
    }
}

// ─── Literal impl ────────────────────────────────────────────────────────────

impl<'a> Internable for Literal<'a> {
    fn feed(&self, h: &mut Hasher) {
        match self {
            Literal::Null => {
                h.update(&[T_NULL]);
            }
            Literal::Bool(b) => {
                h.update(&[T_BOOL, u8::from(*b)]);
            }

            Literal::String(s) => feed_str(h, T_STRING, s.as_ref()),
            Literal::Json(s) => feed_str(h, T_JSON, s.as_ref()),
            Literal::Xml(s) => feed_str(h, T_XML, s.as_ref()),
            Literal::Enum(s) => feed_str(h, T_ENUM, s.as_ref()),

            Literal::Bytes(b) => feed_bytes(h, T_BYTES, b.as_ref()),
            Literal::Uuid(u) => {
                h.update(&[T_UUID]);
                h.update(u);
            }
            Literal::BitString(bs) => {
                h.update(&[T_BITSTRING]);
                bs.feed(h);
            }

            Literal::Int8(v) => {
                h.update(&[T_INT8]);
                h.update(&v.to_le_bytes());
            }
            Literal::Int16(v) => {
                h.update(&[T_INT16]);
                h.update(&v.to_le_bytes());
            }
            Literal::Int32(v) => {
                h.update(&[T_INT32]);
                h.update(&v.to_le_bytes());
            }
            Literal::Int64(v) => {
                h.update(&[T_INT64]);
                h.update(&v.to_le_bytes());
            }
            Literal::Int128(v) => {
                h.update(&[T_INT128]);
                h.update(&v.to_le_bytes());
            }
            Literal::UInt8(v) => {
                h.update(&[T_UINT8, *v]);
            }
            Literal::UInt16(v) => {
                h.update(&[T_UINT16]);
                h.update(&v.to_le_bytes());
            }
            Literal::UInt32(v) => {
                h.update(&[T_UINT32]);
                h.update(&v.to_le_bytes());
            }
            Literal::UInt64(v) => {
                h.update(&[T_UINT64]);
                h.update(&v.to_le_bytes());
            }
            Literal::UInt128(v) => {
                h.update(&[T_UINT128]);
                h.update(&v.to_le_bytes());
            }

            Literal::Float32(v) => {
                h.update(&[T_FLOAT32]);
                h.update(&canonical_f32_bits(*v));
            }
            Literal::Float64(v) => {
                h.update(&[T_FLOAT64]);
                h.update(&canonical_f64_bits(*v));
            }

            #[cfg(feature = "numeric")]
            Literal::Decimal(d) => {
                h.update(&[T_DECIMAL]);
                d.feed(h);
            }

            #[cfg(feature = "network")]
            Literal::Inet(ip) => {
                h.update(&[T_INET]);
                ip.feed(h);
            }
            #[cfg(feature = "network")]
            Literal::MacAddr(m) => {
                h.update(&[T_MACADDR]);
                m.feed(h);
            }

            #[cfg(feature = "datetime")]
            Literal::Date(d) => {
                h.update(&[T_DATE]);
                d.feed(h);
            }
            #[cfg(feature = "datetime")]
            Literal::Time(t) => {
                h.update(&[T_TIME]);
                t.feed(h);
            }
            #[cfg(feature = "datetime")]
            Literal::DateTime(dt) => {
                h.update(&[T_DATETIME]);
                dt.feed(h);
            }
            #[cfg(feature = "datetime")]
            Literal::TimestampTz(ts) => {
                h.update(&[T_TIMESTAMPTZ]);
                ts.feed(h);
            }
            #[cfg(feature = "datetime")]
            Literal::Interval(iv) => {
                h.update(&[T_INTERVAL]);
                iv.feed(h);
            }

            #[cfg(feature = "geo")]
            Literal::Point(p) => {
                h.update(&[T_POINT]);
                p.feed(h);
            }
            #[cfg(feature = "geo")]
            Literal::Line(l) => {
                h.update(&[T_LINE]);
                l.feed(h);
            }
            #[cfg(feature = "geo")]
            Literal::Segment(s) => {
                h.update(&[T_SEGMENT]);
                s.feed(h);
            }
            #[cfg(feature = "geo")]
            Literal::Rect(r) => {
                h.update(&[T_RECT]);
                r.feed(h);
            }
            #[cfg(feature = "geo")]
            Literal::Circle(c) => {
                h.update(&[T_CIRCLE]);
                c.feed(h);
            }
            #[cfg(feature = "geo")]
            Literal::Path(p) => {
                h.update(&[T_PATH]);
                p.feed(h);
            }
            #[cfg(feature = "geo")]
            Literal::Polygon(p) => {
                h.update(&[T_POLYGON]);
                p.feed(h);
            }

            Literal::Array(items) => feed_seq(h, T_ARRAY, items),
            Literal::Set(items) => feed_seq(h, T_SET, items),
            Literal::Tuple(items) => feed_seq(h, T_TUPLE, items),
            Literal::Map(items) => feed_keyed(h, T_MAP, items),
            Literal::Struct(items) => feed_keyed(h, T_STRUCT, items),
            Literal::Range(r) => {
                h.update(&[T_RANGE]);
                feed_range(h, r);
            }

            Literal::Extension(boxed) => {
                let (name, payload) = boxed.as_ref();
                h.update(&[T_EXTENSION]);
                feed_len(h, name.len());
                h.update(name.as_bytes());
                feed_len(h, payload.len());
                h.update(payload.as_ref());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;

    enum LitTag {}

    #[test]
    fn identical_int_literals_share_id() {
        let a: Id<LitTag> = Literal::Int64(1).content_id();
        let b: Id<LitTag> = Literal::Int64(1).content_id();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_int_literals_have_distinct_ids() {
        let a: Id<LitTag> = Literal::Int64(1).content_id();
        let b: Id<LitTag> = Literal::Int64(2).content_id();
        assert_ne!(a, b);
    }

    #[test]
    fn int_variant_tag_disambiguates_same_value() {
        // Same numeric value, different concrete variants → different ids.
        let a: Id<LitTag> = Literal::Int8(0).content_id();
        let b: Id<LitTag> = Literal::Int16(0).content_id();
        let c: Id<LitTag> = Literal::Int32(0).content_id();
        let d: Id<LitTag> = Literal::Int64(0).content_id();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(c, d);
        assert_ne!(a, d);
    }

    #[test]
    fn null_and_bool_zero_are_distinct() {
        let n: Id<LitTag> = Literal::Null.content_id();
        let b: Id<LitTag> = Literal::Bool(false).content_id();
        assert_ne!(n, b);
    }

    #[test]
    fn float_zero_and_negative_zero_share_id() {
        let a: Id<LitTag> = Literal::Float64(0.0).content_id();
        let b: Id<LitTag> = Literal::Float64(-0.0).content_id();
        assert_eq!(a, b);
    }

    #[test]
    fn nan_payloads_canonicalise_to_one_id() {
        // Two distinct NaN bit-patterns must hash to the same id.
        let nan_a = f64::from_bits(0x7ff8_0000_0000_0001);
        let nan_b = f64::from_bits(0x7ff8_0000_0000_0002);
        assert!(nan_a.is_nan() && nan_b.is_nan());
        let a: Id<LitTag> = Literal::Float64(nan_a).content_id();
        let b: Id<LitTag> = Literal::Float64(nan_b).content_id();
        assert_eq!(a, b);
    }

    #[test]
    fn string_length_prefix_prevents_framing_collision() {
        // ("ab", "c") must not collide with ("a", "bc") via any framing.
        let a: Id<LitTag> = Literal::Tuple(
            alloc::vec![
                Literal::String(Cow::Borrowed("ab")),
                Literal::String(Cow::Borrowed("c")),
            ]
            .into_boxed_slice(),
        )
        .content_id();
        let b: Id<LitTag> = Literal::Tuple(
            alloc::vec![
                Literal::String(Cow::Borrowed("a")),
                Literal::String(Cow::Borrowed("bc")),
            ]
            .into_boxed_slice(),
        )
        .content_id();
        assert_ne!(a, b);
    }
}

//! Round-trip parity tests for `dol_wire::encoder::Encode` against the
//! existing postcard `Serialize` derivations on `dol-core` leaf types.
//!
//! Every test follows the same shape:
//!
//! 1. Encode a representative value via `encode_to_vec(&v, &mut budget)`.
//! 2. Encode the same value via `postcard::to_allocvec(&v)`.
//! 3. Assert the two byte streams are identical (postcard parity).
//! 4. Decode the first byte stream via `T::decode` and assert structural
//!    equality with the original (self round-trip via the v2 traits alone).
//!
//! When this test passes, `Encode` is byte-for-byte interchangeable with
//! the legacy `encode_postcard<T: Serialize>` shim, which is the
//! prerequisite for retiring the shim in a follow-up PR.

#![cfg(all(
    feature = "postcard",
    feature = "datetime",
    feature = "numeric",
    feature = "geo"
))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_core::policy::{Budget, Limits};
use dol_wire::decoder::{Decode, Reader};
use dol_wire::encoder::{Encode, encode_to_vec};

/// Encode `value` via `Encode`, assert its bytes match
/// `postcard::to_allocvec(&value)`, then decode them back via `Decode`
/// and return the round-tripped value for structural comparison.
fn rt<T>(value: &T) -> T
where
    T: serde::Serialize + Encode + Decode + core::fmt::Debug,
{
    let mut enc_budget = Budget::new(Limits::host());
    let our_bytes = encode_to_vec(value, &mut enc_budget).expect("Encode succeeds");
    let postcard_bytes = postcard::to_allocvec(value).expect("postcard encode");
    assert_eq!(
        our_bytes, postcard_bytes,
        "Encode bytes must equal postcard bytes for {value:?}"
    );

    let mut reader = Reader::new(&our_bytes);
    let mut dec_budget = Budget::new(Limits::host());
    let decoded = T::decode(&mut reader, &mut dec_budget)
        .unwrap_or_else(|e| panic!("decode of {value:?} failed: {e}"));
    assert!(
        reader.is_exhausted(),
        "trailing bytes after decoding {value:?}: {} bytes left",
        reader.remaining(),
    );
    decoded
}

// ─── Always-on leaves ───────────────────────────────────────────────────────

#[test]
fn bit_string_round_trip() {
    let cases = [
        dol_core::BitString::zeroes(0),
        dol_core::BitString::zeroes(1),
        dol_core::BitString::zeroes(8),
        dol_core::BitString::zeroes(17),
        dol_core::BitString::try_new(12, vec![0b1010_0101, 0b1100_0000].into_boxed_slice())
            .expect("valid bit string"),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
    }
}

#[test]
fn file_id_round_trip() {
    for v in [
        dol_core::FileId(0),
        dol_core::FileId(1),
        dol_core::FileId(0xFFFE),
        dol_core::FileId::NONE,
    ] {
        assert_eq!(v, rt(&v));
    }
}

// ─── Datetime leaves ────────────────────────────────────────────────────────

#[test]
fn date_round_trip() {
    let cases = [
        dol_core::Date::try_new(1970, 1, 1).unwrap(),
        dol_core::Date::try_new(2026, 5, 6).unwrap(),
        dol_core::Date::try_new(-4713, 11, 24).unwrap(),
    ];
    for v in &cases {
        assert_eq!(*v, rt(v));
    }
}

#[test]
fn time_round_trip() {
    let cases = [
        dol_core::Time::try_new(0, 0, 0, 0).unwrap(),
        dol_core::Time::try_new(12, 34, 56, 789_000_000).unwrap(),
        dol_core::Time::try_new(23, 59, 59, 999_999_999).unwrap(),
    ];
    for v in &cases {
        assert_eq!(*v, rt(v));
    }
}

#[test]
fn datetime_round_trip() {
    let v = dol_core::DateTime::new(
        dol_core::Date::try_new(2026, 5, 6).unwrap(),
        dol_core::Time::try_new(1, 2, 3, 456_789).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn offset_round_trip() {
    for secs in [0, 3600, -3600, 19_800, -39_600] {
        let v = dol_core::Offset::try_from_seconds(secs).unwrap();
        assert_eq!(v, rt(&v));
    }
}

#[test]
fn timestamp_tz_round_trip() {
    let v = dol_core::TimestampTz::new(
        dol_core::DateTime::new(
            dol_core::Date::try_new(2026, 5, 6).unwrap(),
            dol_core::Time::try_new(7, 8, 9, 10).unwrap(),
        ),
        dol_core::Offset::try_from_seconds(-18_000).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn interval_round_trip() {
    let cases = [
        dol_core::Interval::new(0, 0, 0),
        dol_core::Interval::new(13, -7, 1_234_567_890),
        dol_core::Interval::new(-100, 365, -42),
    ];
    for v in &cases {
        assert_eq!(*v, rt(v));
    }
}

// ─── Numeric leaves ─────────────────────────────────────────────────────────

#[test]
fn decimal_round_trip() {
    let cases = [
        dol_core::Decimal::try_new(0, 0).unwrap(),
        dol_core::Decimal::try_new(12345, 2).unwrap(),
        dol_core::Decimal::try_new(-12345, 2).unwrap(),
        dol_core::Decimal::try_new(i128::MAX, 0).unwrap(),
        dol_core::Decimal::try_new(i128::MIN, 0).unwrap(),
        dol_core::Decimal::try_new(1, dol_core::Decimal::MAX_SCALE).unwrap(),
    ];
    for v in &cases {
        assert_eq!(*v, rt(v));
    }
}

// ─── Geo leaves ─────────────────────────────────────────────────────────────

#[test]
fn point_round_trip() {
    let cases = [
        dol_core::Point::try_new(0.0, 0.0).unwrap(),
        dol_core::Point::try_new(-3.5, 4.5).unwrap(),
        dol_core::Point::try_new(f64::MIN, f64::MAX).unwrap(),
    ];
    for v in &cases {
        assert_eq!(*v, rt(v));
    }
}

#[test]
fn line_round_trip() {
    let v = dol_core::geo::Line::try_new(1.0, -2.0, 3.0).unwrap();
    assert_eq!(v, rt(&v));
}

#[test]
fn segment_round_trip() {
    let v = dol_core::geo::Segment::new(
        dol_core::Point::try_new(0.0, 1.0).unwrap(),
        dol_core::Point::try_new(2.0, 3.0).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn rect_round_trip() {
    let v = dol_core::geo::Rect::new(
        dol_core::Point::try_new(-1.0, -1.0).unwrap(),
        dol_core::Point::try_new(1.0, 1.0).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn circle_round_trip() {
    let v = dol_core::geo::Circle::try_new(dol_core::Point::try_new(0.0, 0.0).unwrap(), 5.0)
        .unwrap();
    assert_eq!(v, rt(&v));
}

//! Round-trip parity tests for `dol_wire::decoder::Decode` against the
//! existing postcard `Serialize` derivations on `dol-core` leaf types.
//!
//! Every test follows the same shape:
//!
//! 1. Encode a representative value via `postcard::to_allocvec(&v)`.
//! 2. Decode the same bytes via `T::decode(&mut Reader::new(&bytes), …)`.
//! 3. Assert structural equality and that the reader is fully consumed.
//!
//! When this test passes, the new `Decode` impl is byte-for-byte
//! interchangeable with the legacy `decode_postcard<T: Deserialize>`
//! shim, which is the prerequisite for retiring the shim in a follow-up PR.

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

fn rt<T>(value: &T) -> T
where
    T: serde::Serialize + Decode + core::fmt::Debug,
{
    let bytes = postcard::to_allocvec(value).expect("postcard encode");
    let mut reader = Reader::new(&bytes);
    let mut budget = Budget::new(Limits::host());
    let decoded = T::decode(&mut reader, &mut budget)
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
        dol_core::FileId(0xFEFE),
        dol_core::FileId::NONE,
    ] {
        assert_eq!(v, rt(&v));
    }
}

// ─── Datetime leaves ────────────────────────────────────────────────────────

#[test]
fn date_round_trip() {
    let cases = [
        dol_core::Date::try_new(2026, 1, 1).unwrap(),
        dol_core::Date::try_new(2026, 12, 31).unwrap(),
        dol_core::Date::try_new(-44, 3, 15).unwrap(),
        dol_core::Date::try_new(0, 1, 1).unwrap(),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
    }
}

#[test]
fn time_round_trip() {
    let cases = [
        dol_core::Time::MIDNIGHT,
        dol_core::Time::try_new(12, 34, 56, 0).unwrap(),
        dol_core::Time::try_new(23, 59, 60, 999_999_999).unwrap(),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
    }
}

#[test]
fn datetime_round_trip() {
    let v = dol_core::DateTime::new(
        dol_core::Date::try_new(2026, 5, 6).unwrap(),
        dol_core::Time::try_new(7, 8, 9, 10).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn offset_round_trip() {
    let cases = [
        dol_core::Offset::UTC,
        dol_core::Offset::try_from_seconds(3_600).unwrap(),
        dol_core::Offset::try_from_seconds(-18_000).unwrap(),
        dol_core::Offset::try_from_seconds(86_399).unwrap(),
        dol_core::Offset::try_from_seconds(-86_399).unwrap(),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
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
        dol_core::Interval::ZERO,
        dol_core::Interval::new(13, 0, 0),
        dol_core::Interval::new(0, 7, 0),
        dol_core::Interval::new(-1, -2, -3),
        dol_core::Interval::new(0, 0, 1_234_567_890_123),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
    }
}

// ─── Numeric leaves ─────────────────────────────────────────────────────────

#[test]
fn decimal_round_trip() {
    let cases = [
        dol_core::Decimal::try_new(0, 0).unwrap(),
        dol_core::Decimal::try_new(1, 0).unwrap(),
        dol_core::Decimal::try_new(-1, 0).unwrap(),
        dol_core::Decimal::try_new(12_345, 2).unwrap(),
        dol_core::Decimal::try_new(i128::MAX, 38).unwrap(),
        dol_core::Decimal::try_new(i128::MIN, 0).unwrap(),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
    }
}

// ─── Geo leaves ─────────────────────────────────────────────────────────────

#[test]
fn point_round_trip() {
    let cases = [
        dol_core::Point::try_new(0.0, 0.0).unwrap(),
        dol_core::Point::try_new(1.5, -2.25).unwrap(),
        dol_core::Point::try_new(f64::MAX, f64::MIN).unwrap(),
    ];
    for v in &cases {
        assert_eq!(v, &rt(v));
    }
}

#[test]
fn line_round_trip() {
    let v = dol_core::geo::Line::try_new(1.0, 2.0, 3.0).unwrap();
    assert_eq!(v, rt(&v));
}

#[test]
fn segment_round_trip() {
    let v = dol_core::geo::Segment::new(
        dol_core::Point::try_new(0.0, 0.0).unwrap(),
        dol_core::Point::try_new(1.0, 1.0).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn rect_round_trip() {
    let v = dol_core::geo::Rect::new(
        dol_core::Point::try_new(0.0, 0.0).unwrap(),
        dol_core::Point::try_new(10.0, 20.0).unwrap(),
    );
    assert_eq!(v, rt(&v));
}

#[test]
fn circle_round_trip() {
    let v = dol_core::geo::Circle::try_new(
        dol_core::Point::try_new(5.0, 5.0).unwrap(),
        2.5,
    )
    .unwrap();
    assert_eq!(v, rt(&v));
}

// ─── Reader-level guarantees ────────────────────────────────────────────────

#[test]
fn invalid_date_is_rejected() {
    // Hand-crafted bytes for `Date { year: 2026, month: 13, day: 1 }` —
    // valid postcard, invalid Date (month out of range).
    let bytes = postcard::to_allocvec(&(2026_i32, 13_u8, 1_u8)).unwrap();
    let mut reader = Reader::new(&bytes);
    let mut budget = Budget::new(Limits::host());
    let r = dol_core::Date::decode(&mut reader, &mut budget);
    assert!(r.is_err(), "Date::decode must reject invalid calendar fields");
}

#[test]
fn invalid_decimal_scale_is_rejected() {
    // Scale 100 violates Decimal::MAX_SCALE (= 38).
    let bytes = postcard::to_allocvec(&(0_i128, 100_u32)).unwrap();
    let mut reader = Reader::new(&bytes);
    let mut budget = Budget::new(Limits::host());
    let r = dol_core::Decimal::decode(&mut reader, &mut budget);
    assert!(r.is_err(), "Decimal::decode must reject out-of-range scale");
}

#[test]
fn bit_string_byte_count_mismatch_is_rejected() {
    // 17 bits requires 3 bytes; pass only 2 to provoke validation failure.
    let bytes = postcard::to_allocvec(&(17_u32, vec![0u8, 0u8])).unwrap();
    let mut reader = Reader::new(&bytes);
    let mut budget = Budget::new(Limits::host());
    let r = dol_core::BitString::decode(&mut reader, &mut budget);
    assert!(
        r.is_err(),
        "BitString::decode must reject byte count != ceil(len/8)"
    );
}

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
    let v =
        dol_core::geo::Circle::try_new(dol_core::Point::try_new(5.0, 5.0).unwrap(), 2.5).unwrap();
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
    assert!(
        r.is_err(),
        "Date::decode must reject invalid calendar fields"
    );
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

// ─── Span and SpanTable (always-on, postcard-compatible) ─────────────────────

#[test]
fn span_round_trip() {
    for v in [
        dol_core::Span::NONE,
        dol_core::Span::new(dol_core::FileId(0), 0, 0),
        dol_core::Span::new(dol_core::FileId(7), 100, 25),
        dol_core::Span::new(dol_core::FileId(0xFFFE), 0x00FF_FFFF, 0x00FF_FFFF),
    ] {
        assert_eq!(v, rt(&v));
    }
}

#[test]
fn span_table_round_trip() {
    // Empty table.
    let empty = dol_core::span::SpanTable::new();
    let decoded: dol_core::span::SpanTable = {
        let bytes = postcard::to_allocvec(&empty).expect("postcard encode SpanTable");
        let mut reader = Reader::new(&bytes);
        let mut budget = Budget::new(Limits::host());
        dol_core::span::SpanTable::decode(&mut reader, &mut budget).expect("decode empty SpanTable")
    };
    assert_eq!(decoded.len(), 0);

    // Table with a few spans.
    let mut table = dol_core::span::SpanTable::new();
    table.push(dol_core::Span::NONE);
    table.push(dol_core::Span::new(dol_core::FileId(1), 10, 20));
    table.push(dol_core::Span::new(dol_core::FileId(2), 30, 5));
    let bytes = postcard::to_allocvec(&table).expect("postcard encode SpanTable");
    let mut reader = Reader::new(&bytes);
    let mut budget = Budget::new(Limits::host());
    let decoded =
        dol_core::span::SpanTable::decode(&mut reader, &mut budget).expect("decode SpanTable");
    assert!(reader.is_exhausted());
    assert_eq!(decoded.len(), 3);
    assert!(decoded.get(0).is_none());
    assert_eq!(decoded.get(1).file(), dol_core::FileId(1));
    assert_eq!(decoded.get(1).start(), 10);
    assert_eq!(decoded.get(1).length(), 20);
    assert_eq!(decoded.get(2).file(), dol_core::FileId(2));
}

// ─── Geo compound leaves (postcard-compatible) ────────────────────────────────

#[test]
fn path_round_trip() {
    let open = dol_core::geo::Path::new(
        false,
        vec![
            dol_core::Point::try_new(0.0, 0.0).unwrap(),
            dol_core::Point::try_new(1.0, 1.0).unwrap(),
        ],
    );
    let closed = dol_core::geo::Path::new(
        true,
        vec![
            dol_core::Point::try_new(0.0, 0.0).unwrap(),
            dol_core::Point::try_new(2.0, 0.0).unwrap(),
            dol_core::Point::try_new(1.0, 1.0).unwrap(),
        ],
    );
    assert_eq!(open, rt(&open));
    assert_eq!(closed, rt(&closed));
}

#[test]
fn polygon_round_trip() {
    let poly = dol_core::geo::Polygon::new(vec![
        dol_core::Point::try_new(0.0, 0.0).unwrap(),
        dol_core::Point::try_new(1.0, 0.0).unwrap(),
        dol_core::Point::try_new(0.5, 1.0).unwrap(),
    ]);
    assert_eq!(poly, rt(&poly));
}

// ─── Network leaves: self-roundtrip (custom encoding, NOT postcard parity) ───

#[cfg(feature = "network")]
mod network_roundtrip {
    use super::*;
    use dol_wire::encoder::encode_to_vec;

    fn rt_custom<T>(value: &T) -> T
    where
        T: dol_wire::Encode + dol_wire::Decode + core::fmt::Debug + PartialEq,
    {
        let mut budget = Budget::new(Limits::host());
        let bytes = encode_to_vec(value, &mut budget).expect("Encode");
        let mut reader = Reader::new(&bytes);
        let mut dec = Budget::new(Limits::host());
        let decoded = T::decode(&mut reader, &mut dec)
            .unwrap_or_else(|e| panic!("decode of {value:?} failed: {e}"));
        assert!(reader.is_exhausted());
        decoded
    }

    #[test]
    fn ipaddr_v4_round_trip() {
        let v4 = dol_core::IpAddr::v4(192, 168, 0, 1);
        assert_eq!(v4, rt_custom(&v4));
    }

    #[test]
    fn ipaddr_v6_round_trip() {
        let v6 = dol_core::IpAddr::v6([0u8; 16]);
        assert_eq!(v6, rt_custom(&v6));
    }

    #[test]
    fn macaddr_eui48_round_trip() {
        let mac = dol_core::MacAddr::eui48([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);
        assert_eq!(mac, rt_custom(&mac));
    }

    #[test]
    fn macaddr_eui64_round_trip() {
        let mac = dol_core::MacAddr::eui64([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x70]);
        assert_eq!(mac, rt_custom(&mac));
    }
}

// ─── DataType self-roundtrip ──────────────────────────────────────────────────
//
// DataType uses stable custom discriminants (not postcard-compatible), so
// we test self-roundtrip via Encode → Decode only.

mod data_type_roundtrip {
    use super::*;
    use dol_core::DataType;
    use dol_wire::encoder::encode_to_vec;

    fn rt_dt(value: &DataType) -> DataType {
        let mut budget = Budget::new(Limits::host());
        let bytes = encode_to_vec(value, &mut budget).expect("Encode DataType");
        let mut reader = Reader::new(&bytes);
        let mut dec = Budget::new(Limits::host());
        let decoded = DataType::decode(&mut reader, &mut dec)
            .unwrap_or_else(|e| panic!("decode of {value:?} failed: {e}"));
        assert!(reader.is_exhausted());
        decoded
    }

    #[test]
    fn primitives_round_trip() {
        for v in [
            DataType::Null,
            DataType::Bool,
            DataType::Int8,
            DataType::Int16,
            DataType::Int32,
            DataType::Int64,
            DataType::Int128,
            DataType::UInt8,
            DataType::UInt16,
            DataType::UInt32,
            DataType::UInt64,
            DataType::UInt128,
            DataType::Float32,
            DataType::Float64,
            DataType::Json,
            DataType::Xml,
            DataType::Uuid,
        ] {
            assert_eq!(v, rt_dt(&v));
        }
    }

    #[test]
    fn string_with_options_round_trip() {
        let cases = [
            DataType::String {
                max_len: None,
                fixed: false,
            },
            DataType::String {
                max_len: Some(255),
                fixed: true,
            },
            DataType::Bytes {
                max_len: None,
                fixed: false,
            },
            DataType::Bytes {
                max_len: Some(1024),
                fixed: false,
            },
            DataType::BitString {
                max_len: Some(8),
                fixed: true,
            },
        ];
        for v in &cases {
            assert_eq!(v, &rt_dt(v));
        }
    }

    #[test]
    fn composite_round_trip() {
        let dt = DataType::Array(Box::new(DataType::Int32));
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::Set(Box::new(DataType::String {
            max_len: None,
            fixed: false,
        }));
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::Map {
            value: Box::new(DataType::Bool),
        };
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::Tuple(vec![DataType::Int32, DataType::Bool]);
        assert_eq!(dt, rt_dt(&dt));
    }

    #[test]
    fn struct_and_enum_round_trip() {
        let dt = DataType::Struct(vec![
            dol_core::StructField::new("id", DataType::Int64, false),
            dol_core::StructField::new(
                "name",
                DataType::String {
                    max_len: Some(100),
                    fixed: false,
                },
                true,
            ),
        ]);
        assert_eq!(dt, rt_dt(&dt));

        let dt = DataType::Enum(vec![Box::from("Foo"), Box::from("Bar"), Box::from("Baz")]);
        assert_eq!(dt, rt_dt(&dt));
    }

    #[test]
    fn typeref_and_extension_round_trip() {
        let dt = DataType::TypeRef(Box::from("my_type"));
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::Extension {
            name: Box::from("vector"),
            params: vec![DataType::Float32],
        };
        assert_eq!(dt, rt_dt(&dt));
    }

    #[cfg(feature = "numeric")]
    #[test]
    fn decimal_type_round_trip() {
        let dt = DataType::Decimal {
            precision: Some(10),
            scale: Some(2),
        };
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::Decimal {
            precision: None,
            scale: None,
        };
        assert_eq!(dt, rt_dt(&dt));
    }

    #[cfg(feature = "geo")]
    #[test]
    fn geo_types_round_trip() {
        for v in [
            DataType::Point,
            DataType::Line,
            DataType::LineSegment,
            DataType::Rect,
            DataType::Circle,
            DataType::Path,
            DataType::Polygon,
        ] {
            assert_eq!(v, rt_dt(&v));
        }
    }

    #[cfg(feature = "network")]
    #[test]
    fn network_types_round_trip() {
        for v in [DataType::IpAddr, DataType::MacAddr] {
            assert_eq!(v, rt_dt(&v));
        }
        let dt = DataType::IpNetwork { prefix_len: 24 };
        assert_eq!(dt, rt_dt(&dt));
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn datetime_types_round_trip() {
        for v in [DataType::Date, DataType::Interval] {
            assert_eq!(v, rt_dt(&v));
        }
        let dt = DataType::Time { precision: 6 };
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::DateTime { precision: 3 };
        assert_eq!(dt, rt_dt(&dt));
        let dt = DataType::OffsetDateTime { precision: 0 };
        assert_eq!(dt, rt_dt(&dt));
    }
}

// ─── Value self-roundtrip ─────────────────────────────────────────────────────

mod value_roundtrip {
    use super::*;
    use core::ops::Bound;
    use dol_core::{Value, ValueRange};
    use dol_wire::encoder::encode_to_vec;

    fn rt_val(value: &Value) -> Value {
        let mut budget = Budget::new(Limits::host());
        let bytes = encode_to_vec(value, &mut budget).expect("Encode Value");
        let mut reader = Reader::new(&bytes);
        let mut dec = Budget::new(Limits::host());
        let decoded = Value::decode(&mut reader, &mut dec)
            .unwrap_or_else(|e| panic!("decode of {value:?} failed: {e}"));
        assert!(reader.is_exhausted());
        decoded
    }

    #[test]
    fn scalars_round_trip() {
        for v in [
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Int8(-1),
            Value::Int32(42),
            Value::Int64(i64::MIN),
            Value::UInt8(255),
            Value::UInt64(u64::MAX),
            Value::Float32(1.5),
            Value::Float64(f64::INFINITY),
        ] {
            assert_eq!(v, rt_val(&v));
        }
    }

    #[test]
    fn strings_and_bytes_round_trip() {
        for v in [
            Value::String(Box::from("hello")),
            Value::Json(Box::from("{}")),
            Value::Xml(Box::from("<a/>")),
            Value::Enum(Box::from("Red")),
            Value::Bytes(Box::from([0u8, 1, 2].as_slice())),
            Value::Uuid([0u8; 16]),
        ] {
            assert_eq!(v, rt_val(&v));
        }
    }

    #[test]
    fn large_integers_round_trip() {
        let v = Value::Int128(Box::new(i128::MIN));
        assert_eq!(v, rt_val(&v));
        let v = Value::UInt128(Box::new(u128::MAX));
        assert_eq!(v, rt_val(&v));
    }

    #[test]
    fn composite_values_round_trip() {
        let arr = Value::Array(Box::from([Value::Int32(1), Value::Int32(2)].as_slice()));
        assert_eq!(arr, rt_val(&arr));

        let map = Value::Map(Box::from(
            [(Box::from("key"), Value::Bool(true))].as_slice(),
        ));
        assert_eq!(map, rt_val(&map));

        let range = Value::Range(Box::new(ValueRange {
            start: Bound::Included(Box::new(Value::Int32(1))),
            end: Bound::Excluded(Box::new(Value::Int32(10))),
        }));
        assert_eq!(range, rt_val(&range));
    }

    #[test]
    fn extension_value_round_trip() {
        let ext = Value::Extension(Box::new((
            Box::from("myvec"),
            Box::from([1u8, 2, 3].as_slice()),
        )));
        assert_eq!(ext, rt_val(&ext));
    }

    #[cfg(feature = "network")]
    #[test]
    fn network_values_round_trip() {
        let v = Value::Inet(dol_core::IpAddr::v4(10, 0, 0, 1));
        assert_eq!(v, rt_val(&v));
        let v = Value::MacAddr(dol_core::MacAddr::eui48([0xAA; 6]));
        assert_eq!(v, rt_val(&v));
    }

    #[cfg(feature = "geo")]
    #[test]
    fn geo_values_round_trip() {
        let v = Value::Point(dol_core::Point::try_new(1.0, 2.0).unwrap());
        assert_eq!(v, rt_val(&v));
        let v = Value::Path(Box::new(dol_core::geo::Path::new(
            false,
            vec![dol_core::Point::try_new(0.0, 0.0).unwrap()],
        )));
        assert_eq!(v, rt_val(&v));
    }
}

// ─── Literal<'static> and LiteralRange<'static> self-roundtrip ───────────────
//
// Literal uses stable custom discriminants (not postcard-compatible), so
// we test self-roundtrip via Encode → Decode only.

mod literal_roundtrip {
    use super::*;
    use core::ops::Bound;
    use dol_core::literal::{Literal, LiteralRange};
    use dol_wire::encoder::encode_to_vec;

    fn rt_lit(value: &Literal<'static>) -> Literal<'static> {
        let mut budget = Budget::new(Limits::host());
        let bytes = encode_to_vec(value, &mut budget).expect("Encode Literal");
        let mut reader = Reader::new(&bytes);
        let mut dec = Budget::new(Limits::host());
        let decoded = Literal::decode(&mut reader, &mut dec)
            .unwrap_or_else(|e| panic!("decode of {value:?} failed: {e}"));
        assert!(reader.is_exhausted());
        decoded
    }

    #[test]
    fn scalars_round_trip() {
        for v in [
            Literal::Null,
            Literal::Bool(true),
            Literal::Bool(false),
            Literal::Int8(-1),
            Literal::Int32(42),
            Literal::Int64(i64::MIN),
            Literal::UInt8(255),
            Literal::UInt64(u64::MAX),
            Literal::Float32(1.5_f32),
            Literal::Float64(f64::INFINITY),
        ] {
            assert_eq!(v, rt_lit(&v));
        }
    }

    #[test]
    fn strings_and_bytes_round_trip() {
        use std::borrow::Cow;
        for v in [
            Literal::String(Cow::Borrowed("hello")),
            Literal::Json(Cow::Borrowed("{}")),
            Literal::Xml(Cow::Borrowed("<a/>")),
            Literal::Enum(Cow::Borrowed("Red")),
            Literal::Bytes(Cow::Borrowed(b"data".as_slice())),
            Literal::Uuid([1u8; 16]),
        ] {
            // Decode always produces owned Cow, so compare via into_static
            let encoded_v = v.clone().into_static();
            let decoded = rt_lit(&encoded_v);
            assert_eq!(encoded_v, decoded);
        }
    }

    #[test]
    fn composite_literals_round_trip() {
        use std::borrow::Cow;
        let arr = Literal::Array(Box::from([Literal::Int32(1), Literal::Int32(2)].as_slice()));
        assert_eq!(arr, rt_lit(&arr));

        let map = Literal::Map(Box::from(
            [(Cow::Borrowed("x"), Literal::Bool(true))].as_slice(),
        ));
        let map_static = map.clone().into_static();
        assert_eq!(map_static, rt_lit(&map_static));

        let range = Literal::Range(Box::new(LiteralRange {
            start: Bound::Included(Box::new(Literal::Int32(1))),
            end: Bound::Excluded(Box::new(Literal::Int32(10))),
        }));
        assert_eq!(range, rt_lit(&range));
    }

    #[test]
    fn extension_literal_round_trip() {
        let ext = Literal::Extension(Box::new((
            Box::from("myvec"),
            Box::from([1u8, 2, 3].as_slice()),
        )));
        assert_eq!(ext, rt_lit(&ext));
    }

    #[test]
    fn literal_range_unbounded() {
        let r: LiteralRange<'static> = LiteralRange::unbounded();
        let mut budget = Budget::new(Limits::host());
        let bytes = encode_to_vec(&r, &mut budget).expect("Encode LiteralRange");
        let mut reader = Reader::new(&bytes);
        let mut dec = Budget::new(Limits::host());
        let decoded = LiteralRange::decode(&mut reader, &mut dec).expect("Decode LiteralRange");
        assert_eq!(decoded.start, Bound::Unbounded);
        assert_eq!(decoded.end, Bound::Unbounded);
        assert!(reader.is_exhausted());
    }

    #[cfg(feature = "geo")]
    #[test]
    fn geo_literal_round_trip() {
        let v = Literal::Point(dol_core::Point::try_new(1.0, 2.0).unwrap());
        assert_eq!(v, rt_lit(&v));
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn datetime_literal_round_trip() {
        let v = Literal::Date(dol_core::Date::try_new(2026, 5, 6).unwrap());
        assert_eq!(v, rt_lit(&v));
    }

    #[cfg(feature = "network")]
    #[test]
    fn network_literal_round_trip() {
        let v = Literal::Inet(dol_core::IpAddr::v4(10, 0, 0, 1));
        assert_eq!(v, rt_lit(&v));
    }
}

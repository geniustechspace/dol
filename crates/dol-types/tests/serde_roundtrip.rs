//! Round-trip serde tests for `dol-types`.
//!
//! Verify that representative values of every public top-level type survive
//! a JSON encode → decode cycle structurally. These tests back the
//! universal-`serde` claim made by the workspace.

#![cfg(feature = "serde")]

use std::borrow::Cow;

use dol_types::{
    BitString, DataType, Date, DateTime, Decimal, Interval, IpAddr, Literal, MacAddr, Offset,
    Point, StructField, Time, TimestampTz, Value,
};

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn data_type_round_trip() {
    let cases = [
        DataType::Null,
        DataType::Bool,
        DataType::Int32,
        DataType::Int64,
        DataType::Float64,
        DataType::Decimal {
            precision: Some(18),
            scale: Some(4),
        },
        DataType::String {
            max_len: Some(255),
            fixed: false,
        },
        DataType::Json,
        DataType::Bytes {
            max_len: None,
            fixed: false,
        },
        DataType::Uuid,
        DataType::Date,
        DataType::Time { precision: 6 },
        DataType::DateTime { precision: 6 },
        DataType::OffsetDateTime { precision: 6 },
        DataType::Interval,
        DataType::IpAddr,
    ];
    for ty in &cases {
        assert_eq!(ty, &round_trip(ty));
    }
}

#[test]
fn data_type_composite_round_trip() {
    let composites = vec![
        DataType::Array(Box::new(DataType::Int32)),
        DataType::Set(Box::new(DataType::Uuid)),
        DataType::Tuple(vec![DataType::Int32, DataType::Bool]),
        DataType::Struct(vec![
            StructField::new("id", DataType::Uuid, false),
            StructField::new(
                "label",
                DataType::String {
                    max_len: Some(64),
                    fixed: false,
                },
                true,
            ),
        ]),
        DataType::Enum(vec!["active".into(), "inactive".into()]),
        DataType::TypeRef("user_status".into()),
    ];
    for ty in &composites {
        assert_eq!(ty, &round_trip(ty));
    }
}

#[test]
fn value_round_trip_primitives() {
    let cases: Vec<Value> = vec![
        Value::Null,
        Value::Bool(true),
        Value::Int8(-7),
        Value::Int16(-32_000),
        Value::Int32(-1_234_567),
        Value::Int64(i64::MIN + 1),
        Value::UInt32(42),
        Value::UInt64(u64::MAX),
        Value::Float32(1.5),
        Value::Float64(-2.25),
        Value::String("hello".into()),
        Value::Json(r#"{"a":1}"#.into()),
        Value::Xml("<x/>".into()),
        Value::Enum("active".into()),
        Value::Bytes(vec![0u8, 1, 2, 3].into_boxed_slice()),
        Value::Uuid([0u8; 16]),
    ];
    for v in &cases {
        assert_eq!(v, &round_trip(v));
    }
}

#[test]
fn value_round_trip_temporal_and_network() {
    let date = Date::try_new(2026, 4, 25).expect("valid date");
    let time = Time::try_new(12, 34, 56, 789).expect("valid time");
    let dt = DateTime::new(date, time);
    let tstz = TimestampTz::new(dt, Offset::UTC);
    let interval = Interval::new(1, 2, 3);
    let dec = Decimal::try_new(12_345, 2).expect("valid decimal");
    let bs = BitString::zeroes(13);

    let cases: Vec<Value> = vec![
        Value::Date(date),
        Value::Time(time),
        Value::DateTime(dt),
        Value::TimestampTz(Box::new(tstz)),
        Value::Interval(Box::new(interval)),
        Value::Decimal(Box::new(dec)),
        Value::Inet(IpAddr::v4(127, 0, 0, 1)),
        Value::MacAddr(MacAddr::eui48([0u8; 6])),
        Value::MacAddr(MacAddr::eui64([0u8; 8])),
        Value::Point(Point::new_unchecked(1.0, 2.0)),
        Value::BitString(Box::new(bs)),
    ];
    for v in &cases {
        assert_eq!(v, &round_trip(v));
    }
}

#[test]
fn ip_addr_serialises_as_untagged_octet_array() {
    // V4 -> 4-byte JSON array, V6 -> 16-byte JSON array, with no enum tag.
    let v4 = IpAddr::v4(192, 168, 0, 1);
    assert_eq!(serde_json::to_string(&v4).unwrap(), "[192,168,0,1]");

    let v6 = IpAddr::v6([
        0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01,
    ]);
    assert_eq!(
        serde_json::to_string(&v6).unwrap(),
        "[32,1,13,184,0,0,0,0,0,0,0,0,0,0,0,1]"
    );

    // Round-trip both directions: bare arrays decode back to the right variant.
    let parsed_v4: IpAddr = serde_json::from_str("[10,0,0,1]").unwrap();
    assert_eq!(parsed_v4, IpAddr::v4(10, 0, 0, 1));
    let parsed_v6: IpAddr =
        serde_json::from_str("[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]").unwrap();
    let mut expected_v6 = [0u8; 16];
    expected_v6[15] = 1;
    assert_eq!(parsed_v6, IpAddr::V6(expected_v6));
}

#[test]
fn mac_addr_serialises_as_untagged_octet_array() {
    // EUI-48 -> 6-byte JSON array, EUI-64 -> 8-byte JSON array, no enum tag.
    let eui48 = MacAddr::eui48([0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]);
    assert_eq!(serde_json::to_string(&eui48).unwrap(), "[0,26,43,60,77,94]");

    let eui64 = MacAddr::eui64([0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x80]);
    assert_eq!(
        serde_json::to_string(&eui64).unwrap(),
        "[0,26,43,60,77,94,111,128]"
    );

    let parsed_48: MacAddr = serde_json::from_str("[1,2,3,4,5,6]").unwrap();
    assert_eq!(parsed_48, MacAddr::eui48([1, 2, 3, 4, 5, 6]));
    let parsed_64: MacAddr = serde_json::from_str("[1,2,3,4,5,6,7,8]").unwrap();
    assert_eq!(parsed_64, MacAddr::eui64([1, 2, 3, 4, 5, 6, 7, 8]));
}

#[test]
fn literal_round_trip_owned() {
    // `Literal<'static>` is the canonical owned literal form. Use owned
    // `Cow::Owned` payloads so the value is genuinely `'static`.
    let lits: Vec<Literal<'static>> = vec![
        Literal::Null,
        Literal::Bool(false),
        Literal::Int32(7),
        Literal::Int64(1_000_000),
        Literal::Float64(2.5),
        Literal::String(Cow::Owned("dol".to_string())),
        Literal::Bytes(Cow::Owned(vec![1u8, 2, 3])),
        Literal::Uuid([1u8; 16]),
        Literal::Array(vec![Literal::Int32(1), Literal::Int32(2)].into_boxed_slice()),
        Literal::Tuple(vec![Literal::Bool(true), Literal::Int32(9)].into_boxed_slice()),
    ];
    for l in &lits {
        assert_eq!(l, &round_trip(l));
    }
}

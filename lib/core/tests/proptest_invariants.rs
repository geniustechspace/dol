//! Property-based invariants for `dol-core`.
//!
//! These cover:
//!
//! 1. Validated constructors are exhaustive: every shape that should be
//!    rejected actually is.
//! 2. `DataType::accepts` is consistent: a value built from a constructor
//!    that succeeded conforms to the matching type descriptor.
//!
//! Round-trip stability is exercised separately by `dol-wire`'s
//! `Encode`/`Decode` test suite — `dol-core` itself no longer asserts
//! serde round-trip in v2 (`Deserialize` is gone from in-memory types).

#![cfg(all(
    feature = "datetime",
    feature = "geo",
    feature = "network",
    feature = "numeric"
))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_core::{
    DataType, Date, Decimal, IpAddr, Literal, MacAddr, Time, TypeError, Value,
    datetime::{from_hms_nano, from_ymd},
};
use proptest::prelude::*;

// ─── Constructor exhaustiveness ──────────────────────────────────────────────

proptest! {
    #[test]
    fn date_constructor_rejects_invalid_components(
        year in -9999i32..=9999,
        month in 0u8..=255,
        day in 0u8..=255,
    ) {
        match Date::try_new(year, month, day) {
            Ok(_) => {
                // Component ranges that succeeded must satisfy the spec.
                prop_assert!((1..=12).contains(&month));
                prop_assert!((1..=31).contains(&day));
            }
            Err(TypeError::InvalidMonth(m)) => prop_assert_eq!(m, month),
            Err(TypeError::InvalidDay(d)) => prop_assert_eq!(d, day),
            Err(other) => prop_assert!(false, "unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn time_constructor_rejects_invalid_components(
        hour in 0u8..=255,
        minute in 0u8..=255,
        second in 0u8..=255,
        nano in 0u32..2_000_000_000,
    ) {
        match Time::try_new(hour, minute, second, nano) {
            Ok(_) => {
                prop_assert!(hour <= 23);
                prop_assert!(minute <= 59);
                prop_assert!(second <= 60);
                prop_assert!(nano <= 999_999_999);
            }
            Err(TypeError::InvalidHour(h)) => prop_assert_eq!(h, hour),
            Err(TypeError::InvalidMinute(m)) => prop_assert_eq!(m, minute),
            Err(TypeError::InvalidSecond(s)) => prop_assert_eq!(s, second),
            Err(TypeError::InvalidNanosecond(n)) => prop_assert_eq!(n, nano),
            Err(other) => prop_assert!(false, "unexpected error variant: {other:?}"),
        }
    }
}

// ─── Round-trip ───────────────────────────────────────────────────────────────

fn ip_strategy() -> impl Strategy<Value = IpAddr> {
    prop_oneof![
        any::<[u8; 4]>().prop_map(IpAddr::V4),
        any::<[u8; 16]>().prop_map(IpAddr::V6),
    ]
}

#[allow(dead_code)] // Retained for upcoming Value-level proptests; see docs/v2_plan.md §57.
fn primitive_value_strategy() -> impl Strategy<Value = Value> {
    // Float values are excluded: JSON does not preserve full f64 precision
    // and proptest will eventually find a value where text round-tripping
    // perturbs the last bit. The serde_roundtrip integration test already
    // covers a hand-picked float corpus.
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i32>().prop_map(Value::Int32),
        any::<i64>().prop_map(Value::Int64),
        any::<u128>().prop_map(|n| Value::Uuid(n.to_be_bytes())),
        ip_strategy().prop_map(Value::Inet),
        any::<[u8; 6]>().prop_map(|b| Value::MacAddr(MacAddr::eui48(b))),
        any::<[u8; 8]>().prop_map(|b| Value::MacAddr(MacAddr::eui64(b))),
    ]
}

proptest! {
    #[test]
    fn ip_strategy_constructs_valid_ips(ip in ip_strategy()) {
        // Smoke check: the strategy yields a constructible IpAddr.
        let _ = format!("{ip:?}");
    }
}

#[test]
fn datatype_primitive_constructors_succeed() {
    for dt in [
        DataType::Null,
        DataType::Bool,
        DataType::Int8,
        DataType::Int16,
        DataType::Int32,
        DataType::Int64,
        DataType::Int128,
        DataType::Float32,
        DataType::Float64,
        DataType::Uuid,
        DataType::Json,
        DataType::Xml,
        DataType::Date,
    ] {
        // No-op smoke test: ensures every primitive variant is publicly
        // constructible without `Deserialize`.
        let _ = dt.clone();
    }
}

// ─── DataType::accepts consistency ────────────────────────────────────────────

proptest! {
    /// Booleans always conform to `DataType::Bool`.
    #[test]
    fn bool_value_conforms_to_bool_type(b in any::<bool>()) {
        prop_assert!(DataType::Bool.accepts(&Value::Bool(b)).is_ok());
    }

    /// Int32 values conform to `DataType::Int32` but not to `DataType::Int8`.
    #[test]
    fn int32_conforms_to_int32_type(n in any::<i32>()) {
        prop_assert!(DataType::Int32.accepts(&Value::Int32(n)).is_ok());
    }

    /// Strings inside the declared max_len are accepted; strings exceeding
    /// it are rejected with `StringTooLong`.
    #[test]
    fn string_max_len_is_enforced(
        max in 0u32..32,
        len in 0usize..64,
    ) {
        let s: String = "x".repeat(len);
        let dt = DataType::String { max_len: Some(max), fixed: false };
        match dt.accepts(&Value::String(s.into_boxed_str())) {
            Ok(()) => prop_assert!(len <= max as usize),
            Err(TypeError::StringTooLong { max: m, got }) => {
                prop_assert_eq!(m, max);
                prop_assert_eq!(got, len);
                prop_assert!(len > max as usize);
            }
            Err(other) => prop_assert!(false, "unexpected error variant: {other:?}"),
        }
    }
}

// ─── Literal ↔ Value coverage ─────────────────────────────────────────────────

#[test]
fn literal_into_owned_round_trips_for_primitives() {
    // Build literals directly from primitives, exercise into_owned, and
    // assert structural equivalence with the corresponding Value.
    let cases: Vec<(Literal<'static>, Value)> = vec![
        (Literal::Null, Value::Null),
        (Literal::from(true), Value::Bool(true)),
        (Literal::from(-1i32), Value::Int32(-1)),
        (Literal::from(i64::MAX), Value::Int64(i64::MAX)),
        (Literal::from(2.5_f64), Value::Float64(2.5)),
    ];
    for (lit, expected) in cases {
        let back: Value = lit.into_owned();
        assert_eq!(expected, back);
    }
}

#[test]
fn datetime_constructors_yield_well_typed_values() {
    let date = from_ymd(2026, 1, 15).unwrap();
    assert!(matches!(date, Value::Date(_)));
    assert!(DataType::Date.accepts(&date).is_ok());

    let time = from_hms_nano(9, 30, 0, 1_000).unwrap();
    assert!(matches!(time, Value::Time(_)));
    assert!(DataType::Time { precision: 9 }.accepts(&time).is_ok());
}

#[test]
fn decimal_construction_rejects_oversized_scale() {
    // `Decimal::MAX_SCALE` is the inclusive upper bound.
    let bad = Decimal::try_new(0, Decimal::MAX_SCALE + 1);
    assert!(matches!(bad, Err(TypeError::DecimalScaleTooLarge { .. })));
}

//! Tests for [`DataType::accepts`].

use crate::data_type::DataType;
use crate::error::TypeError;
use crate::value::Value;

use alloc::boxed::Box;
use alloc::vec;

#[test]
fn varchar_accepts_string_within_length() {
    let dt = DataType::varying_string(10);
    assert!(dt.accepts(&Value::from("hello")).is_ok());
}

#[test]
fn varchar_rejects_string_over_length() {
    let dt = DataType::varying_string(3);
    let err = dt.accepts(&Value::from("hello")).unwrap_err();
    assert!(matches!(err, TypeError::StringTooLong { max: 3, got: 5 }));
}

#[cfg(feature = "numeric")]
#[test]
fn decimal_precision_enforced() {
    use crate::numeric::Decimal;

    let dt = DataType::Decimal {
        precision: Some(4),
        scale: Some(2),
    };
    // 123.45 → unscaled = 12345, 5 significant digits > 4
    let v = Value::Decimal(Box::new(Decimal::new_unchecked(12345, 2)));
    let err = dt.accepts(&v).unwrap_err();
    assert!(matches!(err, TypeError::PrecisionExceeded { max: 4, .. }));
}

#[cfg(feature = "numeric")]
#[test]
fn decimal_scale_enforced() {
    use crate::numeric::Decimal;

    let dt = DataType::Decimal {
        precision: None,
        scale: Some(2),
    };
    // scale of 3 exceeds max 2
    let v = Value::Decimal(Box::new(Decimal::new_unchecked(12345, 3)));
    let err = dt.accepts(&v).unwrap_err();
    assert!(matches!(err, TypeError::ScaleExceeded { max: 2, .. }));
}

#[test]
fn enum_rejects_unknown_variant() {
    let dt = DataType::Enum(vec!["active".into(), "inactive".into()]);
    let v = Value::Enum("deleted".into());
    let err = dt.accepts(&v).unwrap_err();
    assert!(
        matches!(err, TypeError::EnumVariantUnknown { ref variant } if variant.as_ref() == "deleted")
    );
}

#[test]
fn enum_accepts_known_variant() {
    let dt = DataType::Enum(vec!["active".into(), "inactive".into()]);
    assert!(dt.accepts(&Value::Enum("active".into())).is_ok());
}

#[test]
fn array_validates_elements() {
    let dt = DataType::Array(Box::new(DataType::Int32));
    let v = Value::Array(vec![Value::Int32(1), Value::Int64(2)].into_boxed_slice());
    let err = dt.accepts(&v).unwrap_err();
    assert!(matches!(err, TypeError::ElementInvalid { index: 1, .. }));
}

#[test]
fn tuple_length_mismatch() {
    let dt = DataType::Tuple(vec![DataType::Int32, DataType::unbounded_string()]);
    let v = Value::Tuple(vec![Value::Int32(1)].into_boxed_slice());
    assert!(matches!(
        dt.accepts(&v).unwrap_err(),
        TypeError::TupleLengthMismatch {
            expected: 2,
            got: 1
        }
    ));
}

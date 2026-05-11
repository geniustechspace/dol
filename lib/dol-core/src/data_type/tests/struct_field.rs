//! Tests for [`super::super::StructField`] and `DataType::Struct` conformance.

use crate::data_type::{DataType, StructField};
use crate::error::TypeError;
use crate::value::Value;

use alloc::vec;

#[test]
fn struct_missing_required_field() {
    let dt = DataType::Struct(vec![
        StructField::new("id", DataType::Int32, false),
        StructField::new("name", DataType::unbounded_string(), false),
    ]);
    // Value has only "id"
    let v = Value::Struct(vec![("id".into(), Value::Int32(1))].into_boxed_slice());
    let err = dt.accepts(&v).unwrap_err();
    assert!(matches!(err, TypeError::StructFieldMissing(ref f) if f.as_ref() == "name"));
}

#[test]
fn struct_unexpected_field_rejected() {
    let dt = DataType::Struct(vec![StructField::new("id", DataType::Int32, false)]);
    let v = Value::Struct(
        vec![
            ("id".into(), Value::Int32(1)),
            ("unknown".into(), Value::Bool(true)),
        ]
        .into_boxed_slice(),
    );
    let err = dt.accepts(&v).unwrap_err();
    assert!(matches!(err, TypeError::StructUnexpectedField(ref f) if f.as_ref() == "unknown"));
}

//! Tests for `From<X> for Value` conversions.

use crate::value::Value;

#[test]
fn value_from_str_and_byte_slice() {
    assert_eq!(Value::from("hello").as_str(), Some("hello"));
    assert_eq!(
        Value::from(&[1u8, 2, 3][..]).as_bytes(),
        Some(&[1u8, 2, 3][..])
    );
}

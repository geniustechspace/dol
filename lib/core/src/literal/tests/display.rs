//! Display tests for [`Literal`].

use crate::literal::Literal;
use alloc::borrow::Cow;
use alloc::string::ToString;
use alloc::vec;

#[test]
fn xml_is_distinct_from_string() {
    let s = Literal::string_borrowed("<a/>");
    let x = Literal::xml_borrowed("<a/>");
    assert_ne!(core::mem::discriminant(&s), core::mem::discriminant(&x));
    assert_eq!(x.as_str(), Some("<a/>"));
}

#[test]
fn set_is_distinct_from_array() {
    let arr = Literal::array(vec![Literal::from(1i32)]);
    let set = Literal::set(vec![Literal::from(1i32)]);
    assert_ne!(core::mem::discriminant(&arr), core::mem::discriminant(&set));
    assert_eq!(set.to_string(), "set[1i32]");
}

#[test]
fn tuple_display() {
    let t = Literal::tuple(vec![Literal::from(1i32), Literal::from("x")]);
    assert_eq!(t.to_string(), "tuple[1i32, \"x\"]");
}

#[test]
fn struct_is_distinct_from_map() {
    let m = Literal::map(vec![(Cow::Borrowed("k"), Literal::from(1i32))]);
    let s = Literal::struct_value(vec![(Cow::Borrowed("k"), Literal::from(1i32))]);
    assert_ne!(core::mem::discriminant(&m), core::mem::discriminant(&s));
    assert!(s.to_string().starts_with("struct{"));
}

#[test]
fn uuid_display_standard_format() {
    let bytes: [u8; 16] = [
        0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44, 0x00,
        0x00,
    ];
    assert_eq!(
        Literal::from(bytes).to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}

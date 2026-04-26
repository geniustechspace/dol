//! Tests for [`Literal`] ↔ [`Value`] conversions.

use crate::literal::Literal;
use crate::value::Value;
use alloc::vec;
use alloc::vec::Vec;

#[test]
fn enum_variant_round_trips() {
    let lit = Literal::enum_variant_borrowed("active");
    let val: Value = lit.into();
    assert!(matches!(val, Value::Enum(ref s) if s.as_ref() == "active"));
}

#[test]
fn all_new_variants_convert_to_value() {
    #[allow(unused_mut)]
    let mut cases: Vec<Literal<'_>> = vec![
        Literal::xml_borrowed("<root/>"),
        Literal::enum_variant_borrowed("active"),
    ];
    #[cfg(feature = "network")]
    {
        cases.push(Literal::macaddr_eui48([0; 6]));
        cases.push(Literal::macaddr_eui64([0; 8]));
        cases.push(Literal::inet_v4(127, 0, 0, 1));
    }
    #[cfg(feature = "geo")]
    {
        use crate::geo::Point;
        cases.push(Literal::from(Point::new_unchecked(0.0, 0.0)));
    }
    for lit in cases {
        let _: Value = lit.into();
    }
}

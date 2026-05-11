//! Size budget assertions for [`Value`] and [`Literal`].

use crate::literal::Literal;
use crate::value::Value;
use core::mem::size_of;

#[test]
fn value_size_is_exactly_24_bytes() {
    assert_eq!(
        size_of::<Value>(),
        24,
        "Value size changed; check for new large unboxed variants (current: {} bytes)",
        size_of::<Value>()
    );
}

#[test]
fn literal_size_is_exactly_32_bytes() {
    assert_eq!(
        size_of::<Literal<'static>>(),
        32,
        "Literal size changed; check for new large unboxed variants (current: {} bytes)",
        size_of::<Literal<'static>>()
    );
}

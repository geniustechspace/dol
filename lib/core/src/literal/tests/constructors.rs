//! Tests for [`super::super::Literal`] constructors (validated and infallible).

#[cfg(feature = "geo")]
#[test]
fn point_rejects_nan() {
    use crate::literal::Literal;
    assert!(Literal::point(f64::NAN, 0.0).is_err());
}

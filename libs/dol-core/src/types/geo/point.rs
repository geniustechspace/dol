use core::fmt;
use super::super::error::TypeError;

/// A 2D point `(x, y)`.
///
/// # Size
///
/// 16 bytes — fits directly as a `Value` variant payload without boxing.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn try_new(x: f64, y: f64) -> Result<Self, TypeError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(TypeError::NonFiniteCoordinate);
        }
        Ok(Self { x, y })
    }

    pub const fn new_unchecked(x: f64, y: f64) -> Self { Self { x, y } }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

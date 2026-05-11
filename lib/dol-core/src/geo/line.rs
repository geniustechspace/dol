use super::{super::error::TypeError, Point};
use core::fmt;

/// An infinite 2D line defined by `ax + by + c = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Line {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Line {
    pub fn try_new(a: f64, b: f64, c: f64) -> Result<Self, TypeError> {
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            return Err(TypeError::NonFiniteCoordinate);
        }
        Ok(Self { a, b, c })
    }

    pub const fn new_unchecked(a: f64, b: f64, c: f64) -> Self {
        Self { a, b, c }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{},{},{}}}", self.a, self.b, self.c)
    }
}

/// A finite 2D line segment defined by two endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Segment {
    pub start: Point,
    pub end: Point,
}

impl Segment {
    pub const fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{},{}]", self.start, self.end)
    }
}

use super::{super::error::TypeError, Point};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;

/// An axis-aligned 2D rectangle defined by two corners.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    pub low: Point,
    pub high: Point,
}

impl Rect {
    pub const fn new(low: Point, high: Point) -> Self {
        Self { low, high }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.low, self.high)
    }
}

/// A 2D circle with a center point and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

impl Circle {
    pub fn try_new(center: Point, radius: f64) -> Result<Self, TypeError> {
        if !radius.is_finite() || radius < 0.0 {
            return Err(TypeError::NonFiniteCoordinate);
        }
        Ok(Self { center, radius })
    }

    pub const fn new_unchecked(center: Point, radius: f64) -> Self {
        Self { center, radius }
    }
}

impl fmt::Display for Circle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{},{}>", self.center, self.radius)
    }
}

/// A 2D path: an ordered sequence of points, open or closed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Path {
    pub closed: bool,
    pub points: Box<[Point]>,
}

impl Path {
    pub fn new(closed: bool, points: Vec<Point>) -> Self {
        Self {
            closed,
            points: points.into_boxed_slice(),
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (open, close) = if self.closed { ('(', ')') } else { ('[', ']') };
        write!(f, "{open}")?;
        for (i, p) in self.points.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{p}")?;
        }
        write!(f, "{close}")
    }
}

/// A 2D polygon: an implicitly-closed ordered sequence of points.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polygon {
    pub points: Box<[Point]>,
}

impl Polygon {
    pub fn new(points: Vec<Point>) -> Self {
        Self {
            points: points.into_boxed_slice(),
        }
    }
}

impl fmt::Display for Polygon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, p) in self.points.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{p}")?;
        }
        write!(f, ")")
    }
}

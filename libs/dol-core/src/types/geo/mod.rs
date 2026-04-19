//! 2D geometric primitives — algebraic types from PostgreSQL's geometry family.
//!
//! These are algebraic 2D shapes (`Point`, `Line`, `Rect`, etc.), **not**
//! geodesic types. There are no coordinate reference systems, projections,
//! or WGS-84 here.
//!
//! # Ergonomic factories
//!
//! ```rust
//! use dol_core::types::geo;
//!
//! let p   = geo::point(1.0, 2.0).unwrap();             // Value::Point
//! let c   = geo::circle(1.0, 2.0, 5.0).unwrap();       // Value::Circle
//! ```

pub mod point;
pub mod line;
pub mod shape;

pub use point::Point;
pub use line::{Line, Segment};
pub use shape::{Circle, Path, Polygon, Rect};

use super::value::Value;
use super::error::TypeError;

// ─── Factory functions ────────────────────────────────────────────────────────

/// Constructs a `Value::Point` from `(x, y)` coordinates.
pub fn point(x: f64, y: f64) -> Result<Value, TypeError> {
    Point::try_new(x, y).map(Value::Point)
}

/// Constructs a `Value::Line` from `(a, b, c)` coefficients of `ax+by+c=0`.
pub fn line(a: f64, b: f64, c: f64) -> Result<Value, TypeError> {
    Line::try_new(a, b, c).map(|l| Value::Line(Box::new(l)))
}

/// Constructs a `Value::Segment` from two endpoint pairs.
pub fn segment(x1: f64, y1: f64, x2: f64, y2: f64) -> Result<Value, TypeError> {
    let start = Point::try_new(x1, y1)?;
    let end   = Point::try_new(x2, y2)?;
    Ok(Value::Segment(Box::new(Segment::new(start, end))))
}

/// Constructs a `Value::Rect` from two corner pairs (low, high).
pub fn rect(lx: f64, ly: f64, hx: f64, hy: f64) -> Result<Value, TypeError> {
    let low  = Point::try_new(lx, ly)?;
    let high = Point::try_new(hx, hy)?;
    Ok(Value::Rect(Box::new(Rect::new(low, high))))
}

/// Constructs a `Value::Circle` from center `(cx, cy)` and `radius`.
pub fn circle(cx: f64, cy: f64, radius: f64) -> Result<Value, TypeError> {
    let center = Point::try_new(cx, cy)?;
    Circle::try_new(center, radius).map(|c| Value::Circle(Box::new(c)))
}

/// Constructs a `Value::Path` from a list of `(x, y)` pairs.
pub fn path(closed: bool, coords: Vec<(f64, f64)>) -> Result<Value, TypeError> {
    let points = coords.into_iter()
        .map(|(x, y)| Point::try_new(x, y))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::Path(Box::new(Path::new(closed, points))))
}

/// Constructs a `Value::Polygon` from a list of `(x, y)` pairs.
pub fn polygon(coords: Vec<(f64, f64)>) -> Result<Value, TypeError> {
    let points = coords.into_iter()
        .map(|(x, y)| Point::try_new(x, y))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::Polygon(Box::new(Polygon::new(points))))
}

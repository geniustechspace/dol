//! Runtime [`Value`] enum.

use alloc::boxed::Box;

use super::ValueRange;
use crate::binary::BitString;
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;

/// An owned runtime value for the full DOL type system.
///
/// This is what flows through the query engine at runtime: the output of
/// decoding, the input to binding, and the unit of storage and transport.
///
/// # Size
///
/// `size_of::<Value>() == 24` on 64-bit targets. Large payload types are
/// heap-boxed to preserve this budget.
///
/// # Float semantics
///
/// `Float32` and `Float64` variants may hold `NaN` or infinity. Use the
/// [`crate::Literal::float32`]/[`crate::Literal::float64`] fallible constructors
/// to enforce finite-only values at the literal layer.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Value {
    // ── Primitive ──
    Null,
    Bool(bool),

    // ── Text ──
    String(Box<str>),
    Json(Box<str>),
    Xml(Box<str>),
    /// A named enum variant. The type's variant list lives in [`crate::DataType::Enum`].
    Enum(Box<str>),

    // ── Binary ──
    Bytes(Box<[u8]>),
    Uuid([u8; 16]),
    BitString(Box<BitString>),

    // ── Integer ──
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(Box<i128>),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    UInt128(Box<u128>),

    // ── Float ──
    Float32(f32),
    Float64(f64),

    // ── Decimal ──
    #[cfg(feature = "numeric")]
    Decimal(Box<Decimal>),

    // ── Network ──
    #[cfg(feature = "network")]
    Inet(IpAddr),
    #[cfg(feature = "network")]
    MacAddr(MacAddr),

    // ── Temporal ──
    #[cfg(feature = "datetime")]
    Date(Date),
    #[cfg(feature = "datetime")]
    Time(Time),
    #[cfg(feature = "datetime")]
    DateTime(DateTime),
    #[cfg(feature = "datetime")]
    TimestampTz(Box<TimestampTz>),
    #[cfg(feature = "datetime")]
    Interval(Box<Interval>),

    // ── Geometric ──
    // Point is 16 bytes — fits unboxed.
    #[cfg(feature = "geo")]
    Point(Point),
    // All other geometry types are ≥ 24 bytes — boxed to stay within budget.
    #[cfg(feature = "geo")]
    Line(Box<Line>),
    #[cfg(feature = "geo")]
    Segment(Box<Segment>),
    #[cfg(feature = "geo")]
    Rect(Box<Rect>),
    #[cfg(feature = "geo")]
    Circle(Box<Circle>),
    #[cfg(feature = "geo")]
    Path(Box<Path>),
    #[cfg(feature = "geo")]
    Polygon(Box<Polygon>),

    // ── Composite ──
    /// Ordered, homogeneous-by-convention sequence.
    Array(Box<[Value]>),
    /// Unordered, unique-by-convention collection.
    Set(Box<[Value]>),
    /// Fixed-length, positional tuple.
    Tuple(Box<[Value]>),
    /// Dynamic string-keyed map (document/hash semantics).
    Map(Box<[(Box<str>, Value)]>),
    /// Schema-typed named-field record.
    Struct(Box<[(Box<str>, Value)]>),
    /// Bounded range.
    Range(Box<ValueRange>),

    // ── Extension ──
    /// A domain-specific extension value not covered by the well-known variants.
    /// Boxed to keep `Value` at 24 bytes on 64-bit targets.
    Extension(Box<(Box<str>, Box<[u8]>)>),
}

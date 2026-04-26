//! Borrowed-or-owned literal AST nodes.

use alloc::borrow::Cow;
use alloc::boxed::Box;

use super::LiteralRange;
use crate::binary::BitString;
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;

/// A literal constant in a DOL expression.
///
/// Optimized for zero-copy AST construction: strings and bytes are
/// `Cow<'a, _>`, scalars are inline, and large composite types are
/// heap-boxed to keep the enum at 32 bytes.
///
/// # Size
///
/// `size_of::<Literal<'static>>() == 32` on 64-bit targets.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Literal<'a> {
    // ── Primitive ──
    Null,
    Bool(bool),

    // ── Text ──
    String(Cow<'a, str>),
    Json(Cow<'a, str>),
    Xml(Cow<'a, str>),
    /// A named enum variant.
    Enum(Cow<'a, str>),

    // ── Binary ──
    Bytes(Cow<'a, [u8]>),
    Uuid([u8; 16]),
    BitString(Box<BitString>),

    // ── Integer ──
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    UInt128(u128),

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
    #[cfg(feature = "geo")]
    Point(Point),
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
    Array(Box<[Literal<'a>]>),
    Set(Box<[Literal<'a>]>),
    Tuple(Box<[Literal<'a>]>),
    Map(Box<[(Cow<'a, str>, Literal<'a>)]>),
    Struct(Box<[(Cow<'a, str>, Literal<'a>)]>),
    Range(Box<LiteralRange<'a>>),

    // ── Extension ──
    /// A domain-specific extension value not covered by the well-known variants.
    /// Boxed to keep `Literal` at 32 bytes on 64-bit targets.
    Extension(Box<(Box<str>, Box<[u8]>)>),
}

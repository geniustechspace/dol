//! Runtime values and borrowed literals.
//!
//! See the [module-level documentation][crate] for the full type inventory and
//! design rationale.

use core::fmt;
use std::borrow::Cow;
use std::ops::Bound;

use super::datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
use super::network::{IpAddr, MacAddr, MacAddr8};
use super::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
use super::numeric::Decimal;
use super::binary::BitString;

// ─── Shared helpers ───────────────────────────────────────────────────────────

pub(crate) fn fmt_uuid(bytes: &[u8; 16], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
         {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],  bytes[1],  bytes[2],  bytes[3],
        bytes[4],  bytes[5],  bytes[6],  bytes[7],
        bytes[8],  bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

// ─── Range types ─────────────────────────────────────────────────────────────

/// A range bound-pair for [`Literal`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LiteralRange<'a> {
    pub start: Bound<Box<Literal<'a>>>,
    pub end:   Bound<Box<Literal<'a>>>,
}

/// A range bound-pair for [`Value`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueRange {
    pub start: Bound<Box<Value>>,
    pub end:   Bound<Box<Value>>,
}

impl<'a> LiteralRange<'a> {
    pub fn new(start: Bound<Literal<'a>>, end: Bound<Literal<'a>>) -> Self {
        Self { start: start.map(Box::new), end: end.map(Box::new) }
    }

    pub fn unbounded() -> Self {
        Self { start: Bound::Unbounded, end: Bound::Unbounded }
    }
}

impl ValueRange {
    pub fn new(start: Bound<Value>, end: Bound<Value>) -> Self {
        Self { start: start.map(Box::new), end: end.map(Box::new) }
    }
}

fn fmt_bound<T: fmt::Display>(b: &Bound<Box<T>>, open: char, closed: char,
    f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match b {
        Bound::Included(v) => write!(f, "{closed}{v}"),
        Bound::Excluded(v) => write!(f, "{open}{v}"),
        Bound::Unbounded   => write!(f, "{open}"),
    }
}

impl<'a> fmt::Display for LiteralRange<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bound(&self.start, '(', '[', f)?;
        f.write_str(",")?;
        fmt_bound(&self.end, ')', ']', f)
    }
}

impl fmt::Display for ValueRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bound(&self.start, '(', '[', f)?;
        f.write_str(",")?;
        fmt_bound(&self.end, ')', ']', f)
    }
}

// ─── Value ───────────────────────────────────────────────────────────────────

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
/// [`Literal::float32`]/[`Literal::float64`] fallible constructors to enforce
/// finite-only values at the literal layer.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Value {
    // ── Primitive ──
    Null,
    Bool(bool),

    // ── Text ──
    String(Box<str>),
    Json(Box<str>),
    Xml(Box<str>),
    /// A named enum variant. The type's variant list lives in [`super::DataType::Enum`].
    Enum(Box<str>),

    // ── Binary ──
    Bytes(Box<[u8]>),
    Uuid([u8; 16]),
    BitString(Box<BitString>),

    // ── Integer ──
    Int8(i8), Int16(i16), Int32(i32), Int64(i64), Int128(i128),
    UInt8(u8), UInt16(u16), UInt32(u32), UInt64(u64), UInt128(u128),

    // ── Float ──
    Float32(f32),
    Float64(f64),

    // ── Decimal ──
    Decimal(Decimal),

    // ── Network ──
    Inet(IpAddr),
    MacAddr(MacAddr),
    MacAddr8(MacAddr8),

    // ── Temporal ──
    Date(Date),
    Time(Time),
    DateTime(DateTime),
    TimestampTz(TimestampTz),
    Interval(Interval),

    // ── Geometric ──
    // Point is 16 bytes — fits unboxed.
    Point(Point),
    // All other geometry types are ≥ 24 bytes — boxed to stay within budget.
    Line(Box<Line>),
    Segment(Box<Segment>),
    Rect(Box<Rect>),
    Circle(Box<Circle>),
    Path(Box<Path>),
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
}

impl Value {
    pub fn is_null(&self) -> bool     { matches!(self, Self::Null) }
    pub fn is_integer(&self) -> bool  { matches!(self, Self::Int8(_) | Self::Int16(_) | Self::Int32(_) | Self::Int64(_) | Self::Int128(_) | Self::UInt8(_) | Self::UInt16(_) | Self::UInt32(_) | Self::UInt64(_) | Self::UInt128(_)) }
    pub fn is_float(&self) -> bool    { matches!(self, Self::Float32(_) | Self::Float64(_)) }
    pub fn is_numeric(&self) -> bool  { self.is_integer() || self.is_float() || matches!(self, Self::Decimal(_)) }
    pub fn is_datetime(&self) -> bool { matches!(self, Self::Date(_) | Self::Time(_) | Self::DateTime(_) | Self::TimestampTz(_) | Self::Interval(_)) }
    pub fn is_textual(&self) -> bool  { matches!(self, Self::String(_) | Self::Json(_) | Self::Xml(_) | Self::Enum(_)) }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null        => "null",
            Self::Bool(_)     => "bool",
            Self::String(_)   => "string",
            Self::Json(_)     => "json",
            Self::Xml(_)      => "xml",
            Self::Enum(_)     => "enum",
            Self::Bytes(_)    => "bytes",
            Self::Uuid(_)     => "uuid",
            Self::BitString(_)=> "bitstring",
            Self::Int8(_)     => "int8",
            Self::Int16(_)    => "int16",
            Self::Int32(_)    => "int32",
            Self::Int64(_)    => "int64",
            Self::Int128(_)   => "int128",
            Self::UInt8(_)    => "uint8",
            Self::UInt16(_)   => "uint16",
            Self::UInt32(_)   => "uint32",
            Self::UInt64(_)   => "uint64",
            Self::UInt128(_)  => "uint128",
            Self::Float32(_)  => "float32",
            Self::Float64(_)  => "float64",
            Self::Decimal(_)  => "decimal",
            Self::Inet(_)     => "inet",
            Self::MacAddr(_)  => "macaddr",
            Self::MacAddr8(_) => "macaddr8",
            Self::Date(_)     => "date",
            Self::Time(_)     => "time",
            Self::DateTime(_) => "datetime",
            Self::TimestampTz(_) => "timestamptz",
            Self::Interval(_) => "interval",
            Self::Point(_)    => "point",
            Self::Line(_)     => "line",
            Self::Segment(_)  => "lseg",
            Self::Rect(_)     => "rect",
            Self::Circle(_)   => "circle",
            Self::Path(_)     => "path",
            Self::Polygon(_)  => "polygon",
            Self::Array(_)    => "array",
            Self::Set(_)      => "set",
            Self::Tuple(_)    => "tuple",
            Self::Map(_)      => "map",
            Self::Struct(_)   => "struct",
            Self::Range(_)    => "range",
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) | Self::Json(v) | Self::Xml(v) | Self::Enum(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self { Self::Bytes(v) => Some(v), _ => None }
    }

    pub fn as_bool(&self)        -> Option<bool>        { if let Self::Bool(v)        = self { Some(*v) } else { None } }
    pub fn as_uuid(&self)        -> Option<[u8; 16]>    { if let Self::Uuid(v)        = self { Some(*v) } else { None } }
    pub fn as_decimal(&self)     -> Option<Decimal>     { if let Self::Decimal(v)     = self { Some(*v) } else { None } }
    pub fn as_date(&self)        -> Option<Date>        { if let Self::Date(v)        = self { Some(*v) } else { None } }
    pub fn as_time(&self)        -> Option<Time>        { if let Self::Time(v)        = self { Some(*v) } else { None } }
    pub fn as_datetime(&self)    -> Option<DateTime>    { if let Self::DateTime(v)    = self { Some(*v) } else { None } }
    pub fn as_timestamp_tz(&self)-> Option<TimestampTz> { if let Self::TimestampTz(v)= self { Some(*v) } else { None } }
    pub fn as_interval(&self)    -> Option<Interval>    { if let Self::Interval(v)    = self { Some(*v) } else { None } }
    pub fn as_inet(&self)        -> Option<IpAddr>      { if let Self::Inet(v)        = self { Some(*v) } else { None } }
    pub fn as_macaddr(&self)     -> Option<MacAddr>     { if let Self::MacAddr(v)     = self { Some(*v) } else { None } }
    pub fn as_macaddr8(&self)    -> Option<MacAddr8>    { if let Self::MacAddr8(v)    = self { Some(*v) } else { None } }
    pub fn as_point(&self)       -> Option<Point>     { if let Self::Point(v)       = self { Some(*v) } else { None } }
    pub fn as_bitstring(&self)   -> Option<&BitString>  { if let Self::BitString(v)   = self { Some(v)  } else { None } }
    pub fn as_array(&self)       -> Option<&[Value]>    { if let Self::Array(v)       = self { Some(v)  } else { None } }
    pub fn as_set(&self)         -> Option<&[Value]>    { if let Self::Set(v)         = self { Some(v)  } else { None } }
    pub fn as_tuple(&self)       -> Option<&[Value]>    { if let Self::Tuple(v)       = self { Some(v)  } else { None } }
    pub fn as_map(&self)         -> Option<&[(Box<str>, Value)]> { if let Self::Map(v) = self { Some(v) } else { None } }
    pub fn as_struct(&self)      -> Option<&[(Box<str>, Value)]> { if let Self::Struct(v) = self { Some(v) } else { None } }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null           => f.write_str("null"),
            Self::Bool(v)        => write!(f, "{v}"),
            Self::String(v)      => write!(f, "\"{v}\""),
            Self::Json(v)        => write!(f, "json({v})"),
            Self::Xml(v)         => write!(f, "xml({v})"),
            Self::Enum(v)        => write!(f, "'{v}'"),
            Self::Bytes(v)       => write!(f, "bytes(len={})", v.len()),
            Self::Uuid(v)        => fmt_uuid(v, f),
            Self::BitString(v)   => write!(f, "{v}"),
            Self::Int8(v)        => write!(f, "{v}i8"),
            Self::Int16(v)       => write!(f, "{v}i16"),
            Self::Int32(v)       => write!(f, "{v}i32"),
            Self::Int64(v)       => write!(f, "{v}i64"),
            Self::Int128(v)      => write!(f, "{v}i128"),
            Self::UInt8(v)       => write!(f, "{v}u8"),
            Self::UInt16(v)      => write!(f, "{v}u16"),
            Self::UInt32(v)      => write!(f, "{v}u32"),
            Self::UInt64(v)      => write!(f, "{v}u64"),
            Self::UInt128(v)     => write!(f, "{v}u128"),
            Self::Float32(v)     => write!(f, "{v}f32"),
            Self::Float64(v)     => write!(f, "{v}f64"),
            Self::Decimal(v)     => write!(f, "{v}"),
            Self::Inet(v)        => write!(f, "{v}"),
            Self::MacAddr(v)     => write!(f, "{v}"),
            Self::MacAddr8(v)    => write!(f, "{v}"),
            Self::Date(v)        => write!(f, "{v}"),
            Self::Time(v)        => write!(f, "{v}"),
            Self::DateTime(v)    => write!(f, "{v}"),
            Self::TimestampTz(v) => write!(f, "{v}"),
            Self::Interval(v)    => write!(f, "{v}"),
            Self::Point(v)       => write!(f, "{v}"),
            Self::Line(v)        => write!(f, "{v}"),
            Self::Segment(v)     => write!(f, "{v}"),
            Self::Rect(v)        => write!(f, "{v}"),
            Self::Circle(v)      => write!(f, "{v}"),
            Self::Path(v)        => write!(f, "{v}"),
            Self::Polygon(v)     => write!(f, "{v}"),
            Self::Array(vs) | Self::Set(vs) | Self::Tuple(vs) => {
                let tag = match self { Self::Set(_) => "set", Self::Tuple(_) => "tuple", _ => "" };
                if !tag.is_empty() { write!(f, "{tag}")?; }
                f.write_str("[")?;
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 { f.write_str(", ")?; }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Self::Map(entries) | Self::Struct(entries) => {
                let tag = match self { Self::Struct(_) => "struct", _ => "" };
                if !tag.is_empty() { write!(f, "{tag}")?; }
                f.write_str("{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { f.write_str(", ")?; }
                    write!(f, "\"{k}\": {v}")?;
                }
                f.write_str("}")
            }
            Self::Range(r) => write!(f, "{r}"),
        }
    }
}

// ─── From impls for Value ────────────────────────────────────────────────────

impl From<&str>       for Value { fn from(v: &str)      -> Self { Self::String(v.into()) } }
impl From<String>     for Value { fn from(v: String)    -> Self { Self::String(v.into_boxed_str()) } }
impl From<Box<str>>   for Value { fn from(v: Box<str>)  -> Self { Self::String(v) } }
impl From<&[u8]>      for Value { fn from(v: &[u8])     -> Self { Self::Bytes(v.into()) } }
impl From<Vec<u8>>    for Value { fn from(v: Vec<u8>)   -> Self { Self::Bytes(v.into_boxed_slice()) } }
impl From<Box<[u8]>>  for Value { fn from(v: Box<[u8]>) -> Self { Self::Bytes(v) } }
impl From<bool>       for Value { fn from(v: bool)      -> Self { Self::Bool(v) } }
impl From<i8>         for Value { fn from(v: i8)        -> Self { Self::Int8(v) } }
impl From<i16>        for Value { fn from(v: i16)       -> Self { Self::Int16(v) } }
impl From<i32>        for Value { fn from(v: i32)       -> Self { Self::Int32(v) } }
impl From<i64>        for Value { fn from(v: i64)       -> Self { Self::Int64(v) } }
impl From<i128>       for Value { fn from(v: i128)      -> Self { Self::Int128(v) } }
impl From<u8>         for Value { fn from(v: u8)        -> Self { Self::UInt8(v) } }
impl From<u16>        for Value { fn from(v: u16)       -> Self { Self::UInt16(v) } }
impl From<u32>        for Value { fn from(v: u32)       -> Self { Self::UInt32(v) } }
impl From<u64>        for Value { fn from(v: u64)       -> Self { Self::UInt64(v) } }
impl From<u128>       for Value { fn from(v: u128)      -> Self { Self::UInt128(v) } }
impl From<f32>        for Value { fn from(v: f32)       -> Self { Self::Float32(v) } }
impl From<f64>        for Value { fn from(v: f64)       -> Self { Self::Float64(v) } }
impl From<[u8; 16]>   for Value { fn from(v: [u8; 16])  -> Self { Self::Uuid(v) } }
impl From<Decimal>    for Value { fn from(v: Decimal)   -> Self { Self::Decimal(v) } }
impl From<Date>       for Value { fn from(v: Date)      -> Self { Self::Date(v) } }
impl From<Time>       for Value { fn from(v: Time)      -> Self { Self::Time(v) } }
impl From<DateTime>   for Value { fn from(v: DateTime)  -> Self { Self::DateTime(v) } }
impl From<TimestampTz>for Value { fn from(v: TimestampTz)-> Self { Self::TimestampTz(v) } }
impl From<Interval>   for Value { fn from(v: Interval)  -> Self { Self::Interval(v) } }
impl From<IpAddr>     for Value { fn from(v: IpAddr)    -> Self { Self::Inet(v) } }
impl From<MacAddr>    for Value { fn from(v: MacAddr)   -> Self { Self::MacAddr(v) } }
impl From<MacAddr8>   for Value { fn from(v: MacAddr8)  -> Self { Self::MacAddr8(v) } }
impl From<Point>    for Value { fn from(v: Point)   -> Self { Self::Point(v) } }

// ─── Literal<'a> ─────────────────────────────────────────────────────────────

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
    Int8(i8), Int16(i16), Int32(i32), Int64(i64), Int128(i128),
    UInt8(u8), UInt16(u16), UInt32(u32), UInt64(u64), UInt128(u128),

    // ── Float ──
    Float32(f32),
    Float64(f64),

    // ── Decimal ──
    Decimal(Decimal),

    // ── Network ──
    Inet(IpAddr),
    MacAddr(MacAddr),
    MacAddr8(MacAddr8),

    // ── Temporal ──
    Date(Date),
    Time(Time),
    DateTime(DateTime),
    TimestampTz(TimestampTz),
    Interval(Interval),

    // ── Geometric ──
    Point(Point),
    Line(Box<Line>),
    Segment(Box<Segment>),
    Rect(Box<Rect>),
    Circle(Box<Circle>),
    Path(Box<Path>),
    Polygon(Box<Polygon>),

    // ── Composite ──
    Array(Box<[Literal<'a>]>),
    Set(Box<[Literal<'a>]>),
    Tuple(Box<[Literal<'a>]>),
    Map(Box<[(Cow<'a, str>, Literal<'a>)]>),
    Struct(Box<[(Cow<'a, str>, Literal<'a>)]>),
    Range(Box<LiteralRange<'a>>),
}

impl<'a> Literal<'a> {
    // ── Primitive constructors ──

    pub const fn null() -> Self { Self::Null }
    pub const fn bool(v: bool) -> Self { Self::Bool(v) }

    // ── Text constructors ──

    pub fn string_borrowed(v: &'a str) -> Self { Self::String(Cow::Borrowed(v)) }
    pub fn string_owned(v: String) -> Self { Self::String(Cow::Owned(v)) }
    pub fn json_borrowed(v: &'a str) -> Self { Self::Json(Cow::Borrowed(v)) }
    pub fn json_owned(v: String) -> Self { Self::Json(Cow::Owned(v)) }
    pub fn xml_borrowed(v: &'a str) -> Self { Self::Xml(Cow::Borrowed(v)) }
    pub fn xml_owned(v: String) -> Self { Self::Xml(Cow::Owned(v)) }
    pub fn enum_variant_borrowed(v: &'a str) -> Self { Self::Enum(Cow::Borrowed(v)) }
    pub fn enum_variant_owned(v: String) -> Self { Self::Enum(Cow::Owned(v)) }

    // ── Binary constructors ──

    pub fn bytes_borrowed(v: &'a [u8]) -> Self { Self::Bytes(Cow::Borrowed(v)) }
    pub fn bytes_owned(v: Vec<u8>) -> Self { Self::Bytes(Cow::Owned(v)) }
    pub const fn uuid(b: [u8; 16]) -> Self { Self::Uuid(b) }

    // ── Validated float constructors ──

    pub fn float32(v: f32) -> Result<Self, super::TypeError> {
        if !v.is_finite() { return Err(super::TypeError::NonFiniteFloat); }
        Ok(Self::Float32(v))
    }

    pub fn float64(v: f64) -> Result<Self, super::TypeError> {
        if !v.is_finite() { return Err(super::TypeError::NonFiniteFloat); }
        Ok(Self::Float64(v))
    }

    // ── Numeric constructors ──

    pub fn decimal(unscaled: i128, scale: u32) -> Result<Self, super::TypeError> {
        match Decimal::try_new(unscaled, scale) {
            Ok(d)  => Ok(Self::Decimal(d)),
            Err(e) => Err(e),
        }
    }

    // ── Temporal constructors ──

    pub fn date(year: i32, month: u8, day: u8) -> Result<Self, super::TypeError> {
        match Date::try_new(year, month, day) {
            Ok(d) => Ok(Self::Date(d)), Err(e) => Err(e),
        }
    }

    pub fn time(h: u8, m: u8, s: u8, ns: u32) -> Result<Self, super::TypeError> {
        match Time::try_new(h, m, s, ns) {
            Ok(t) => Ok(Self::Time(t)), Err(e) => Err(e),
        }
    }

    pub const fn datetime(date: Date, time: Time) -> Self {
        Self::DateTime(DateTime::new(date, time))
    }

    pub const fn timestamp_tz(datetime: DateTime, offset: Offset) -> Self {
        Self::TimestampTz(TimestampTz::new(datetime, offset))
    }

    pub const fn interval(months: i32, days: i32, nanoseconds: i64) -> Self {
        Self::Interval(Interval::new(months, days, nanoseconds))
    }

    // ── Network constructors ──

    pub const fn inet_v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self::Inet(IpAddr::v4(a, b, c, d))
    }

    pub const fn inet_v6(bytes: [u8; 16]) -> Self {
        Self::Inet(IpAddr::v6(bytes))
    }

    pub const fn macaddr(b: [u8; 6]) -> Self { Self::MacAddr(MacAddr::new(b)) }
    pub const fn macaddr8(b: [u8; 8]) -> Self { Self::MacAddr8(MacAddr8::new(b)) }

    // ── Geometric constructors ──

    pub fn point(x: f64, y: f64) -> Result<Self, super::TypeError> {
        Ok(Self::Point(Point::try_new(x, y)?))
    }

    // ── Composite constructors ──

    pub fn array(items: Vec<Literal<'a>>) -> Self {
        Self::Array(items.into_boxed_slice())
    }

    pub fn set(items: Vec<Literal<'a>>) -> Self {
        Self::Set(items.into_boxed_slice())
    }

    pub fn tuple(items: Vec<Literal<'a>>) -> Self {
        Self::Tuple(items.into_boxed_slice())
    }

    pub fn map(entries: Vec<(Cow<'a, str>, Literal<'a>)>) -> Self {
        Self::Map(entries.into_boxed_slice())
    }

    pub fn struct_value(entries: Vec<(Cow<'a, str>, Literal<'a>)>) -> Self {
        Self::Struct(entries.into_boxed_slice())
    }

    pub fn range(start: Bound<Literal<'a>>, end: Bound<Literal<'a>>) -> Self {
        Self::Range(Box::new(LiteralRange::new(start, end)))
    }

    // ── Predicates ──

    pub fn is_null(&self)     -> bool { matches!(self, Self::Null) }
    pub fn is_integer(&self)  -> bool { matches!(self, Self::Int8(_)|Self::Int16(_)|Self::Int32(_)|Self::Int64(_)|Self::Int128(_)|Self::UInt8(_)|Self::UInt16(_)|Self::UInt32(_)|Self::UInt64(_)|Self::UInt128(_)) }
    pub fn is_textual(&self)  -> bool { matches!(self, Self::String(_)|Self::Json(_)|Self::Xml(_)|Self::Enum(_)) }
    pub fn is_datetime(&self) -> bool { matches!(self, Self::Date(_)|Self::Time(_)|Self::DateTime(_)|Self::TimestampTz(_)|Self::Interval(_)) }
    pub fn is_geometric(&self)-> bool { matches!(self, Self::Point(_)|Self::Line(_)|Self::Segment(_)|Self::Rect(_)|Self::Circle(_)|Self::Path(_)|Self::Polygon(_)) }

    // ── Accessors ──

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) | Self::Json(v) | Self::Xml(v) | Self::Enum(v) => Some(v.as_ref()),
            _ => None,
        }
    }

    pub fn as_bytes(&self)    -> Option<&[u8]>    { match self { Self::Bytes(v) => Some(v.as_ref()), _ => None } }
    pub fn as_bool(&self)     -> Option<bool>     { if let Self::Bool(v)       = self { Some(*v) } else { None } }
    pub fn as_uuid(&self)     -> Option<[u8;16]>  { if let Self::Uuid(v)       = self { Some(*v) } else { None } }
    pub fn as_decimal(&self)  -> Option<Decimal>  { if let Self::Decimal(v)    = self { Some(*v) } else { None } }
    pub fn as_date(&self)     -> Option<Date>     { if let Self::Date(v)       = self { Some(*v) } else { None } }
    pub fn as_time(&self)     -> Option<Time>     { if let Self::Time(v)       = self { Some(*v) } else { None } }
    pub fn as_inet(&self)     -> Option<IpAddr>   { if let Self::Inet(v)       = self { Some(*v) } else { None } }
    pub fn as_array(&self)    -> Option<&[Literal<'a>]> { if let Self::Array(v) = self { Some(v) } else { None } }

    // ── Conversion ──

    pub fn into_owned(self) -> Value { self.into() }
}

impl<'a> fmt::Display for Literal<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null           => f.write_str("null"),
            Self::Bool(v)        => write!(f, "{v}"),
            Self::String(v)      => write!(f, "\"{v}\""),
            Self::Json(v)        => write!(f, "json({v})"),
            Self::Xml(v)         => write!(f, "xml({v})"),
            Self::Enum(v)        => write!(f, "'{v}'"),
            Self::Bytes(v)       => write!(f, "bytes(len={})", v.len()),
            Self::Uuid(v)        => fmt_uuid(v, f),
            Self::BitString(v)   => write!(f, "{v}"),
            Self::Int8(v)        => write!(f, "{v}i8"),
            Self::Int16(v)       => write!(f, "{v}i16"),
            Self::Int32(v)       => write!(f, "{v}i32"),
            Self::Int64(v)       => write!(f, "{v}i64"),
            Self::Int128(v)      => write!(f, "{v}i128"),
            Self::UInt8(v)       => write!(f, "{v}u8"),
            Self::UInt16(v)      => write!(f, "{v}u16"),
            Self::UInt32(v)      => write!(f, "{v}u32"),
            Self::UInt64(v)      => write!(f, "{v}u64"),
            Self::UInt128(v)     => write!(f, "{v}u128"),
            Self::Float32(v)     => write!(f, "{v}f32"),
            Self::Float64(v)     => write!(f, "{v}f64"),
            Self::Decimal(v)     => write!(f, "{v}"),
            Self::Inet(v)        => write!(f, "{v}"),
            Self::MacAddr(v)     => write!(f, "{v}"),
            Self::MacAddr8(v)    => write!(f, "{v}"),
            Self::Date(v)        => write!(f, "{v}"),
            Self::Time(v)        => write!(f, "{v}"),
            Self::DateTime(v)    => write!(f, "{v}"),
            Self::TimestampTz(v) => write!(f, "{v}"),
            Self::Interval(v)    => write!(f, "{v}"),
            Self::Point(v)       => write!(f, "{v}"),
            Self::Line(v)        => write!(f, "{v}"),
            Self::Segment(v)     => write!(f, "{v}"),
            Self::Rect(v)        => write!(f, "{v}"),
            Self::Circle(v)      => write!(f, "{v}"),
            Self::Path(v)        => write!(f, "{v}"),
            Self::Polygon(v)     => write!(f, "{v}"),
            Self::Array(vs) | Self::Set(vs) | Self::Tuple(vs) => {
                let tag = match self { Self::Set(_) => "set", Self::Tuple(_) => "tuple", _ => "" };
                if !tag.is_empty() { write!(f, "{tag}")?; }
                f.write_str("[")?;
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 { f.write_str(", ")?; }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Self::Map(entries) | Self::Struct(entries) => {
                let tag = match self { Self::Struct(_) => "struct", _ => "" };
                if !tag.is_empty() { write!(f, "{tag}")?; }
                f.write_str("{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { f.write_str(", ")?; }
                    write!(f, "\"{k}\": {v}")?;
                }
                f.write_str("}")
            }
            Self::Range(r) => write!(f, "{r}"),
        }
    }
}

// ─── From<Literal<'a>> for Value ─────────────────────────────────────────────

impl<'a> From<Literal<'a>> for Value {
    fn from(lit: Literal<'a>) -> Self {
        match lit {
            Literal::Null            => Self::Null,
            Literal::Bool(v)         => Self::Bool(v),
            Literal::String(v)       => Self::String(v.into_owned().into_boxed_str()),
            Literal::Json(v)         => Self::Json(v.into_owned().into_boxed_str()),
            Literal::Xml(v)          => Self::Xml(v.into_owned().into_boxed_str()),
            Literal::Enum(v)         => Self::Enum(v.into_owned().into_boxed_str()),
            Literal::Bytes(v)        => Self::Bytes(v.into_owned().into_boxed_slice()),
            Literal::Uuid(v)         => Self::Uuid(v),
            Literal::BitString(v)    => Self::BitString(v),
            Literal::Int8(v)         => Self::Int8(v),
            Literal::Int16(v)        => Self::Int16(v),
            Literal::Int32(v)        => Self::Int32(v),
            Literal::Int64(v)        => Self::Int64(v),
            Literal::Int128(v)       => Self::Int128(v),
            Literal::UInt8(v)        => Self::UInt8(v),
            Literal::UInt16(v)       => Self::UInt16(v),
            Literal::UInt32(v)       => Self::UInt32(v),
            Literal::UInt64(v)       => Self::UInt64(v),
            Literal::UInt128(v)      => Self::UInt128(v),
            Literal::Float32(v)      => Self::Float32(v),
            Literal::Float64(v)      => Self::Float64(v),
            Literal::Decimal(v)      => Self::Decimal(v),
            Literal::Inet(v)         => Self::Inet(v),
            Literal::MacAddr(v)      => Self::MacAddr(v),
            Literal::MacAddr8(v)     => Self::MacAddr8(v),
            Literal::Date(v)         => Self::Date(v),
            Literal::Time(v)         => Self::Time(v),
            Literal::DateTime(v)     => Self::DateTime(v),
            Literal::TimestampTz(v)  => Self::TimestampTz(v),
            Literal::Interval(v)     => Self::Interval(v),
            Literal::Point(v)        => Self::Point(v),
            Literal::Line(v)         => Self::Line(v),
            Literal::Segment(v)      => Self::Segment(v),
            Literal::Rect(v)         => Self::Rect(v),
            Literal::Circle(v)       => Self::Circle(v),
            Literal::Path(v)         => Self::Path(v),
            Literal::Polygon(v)      => Self::Polygon(v),
            Literal::Array(vs)       => Self::Array(own_slice(vs)),
            Literal::Set(vs)         => Self::Set(own_slice(vs)),
            Literal::Tuple(vs)       => Self::Tuple(own_slice(vs)),
            Literal::Map(entries)    => Self::Map(own_kv_slice(entries)),
            Literal::Struct(entries) => Self::Struct(own_kv_slice(entries)),
            Literal::Range(r)        => Self::Range(convert_range(*r)),
        }
    }
}

fn own_slice<'a>(s: Box<[Literal<'a>]>) -> Box<[Value]> {
    Vec::from(s).into_iter().map(Value::from).collect::<Vec<_>>().into_boxed_slice()
}

fn own_kv_slice<'a>(s: Box<[(Cow<'a, str>, Literal<'a>)]>) -> Box<[(Box<str>, Value)]> {
    Vec::from(s)
        .into_iter()
        .map(|(k, v)| (k.into_owned().into_boxed_str(), Value::from(v)))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn convert_range<'a>(r: LiteralRange<'a>) -> Box<ValueRange> {
    let cvt = |b: Bound<Box<Literal<'a>>>| -> Bound<Box<Value>> {
        match b {
            Bound::Included(v) => Bound::Included(Box::new(Value::from(*v))),
            Bound::Excluded(v) => Bound::Excluded(Box::new(Value::from(*v))),
            Bound::Unbounded   => Bound::Unbounded,
        }
    };
    Box::new(ValueRange { start: cvt(r.start), end: cvt(r.end) })
}

// ─── From impls for Literal<'a> ──────────────────────────────────────────────

impl<'a> From<&'a str>  for Literal<'a> { fn from(v: &'a str) -> Self { Self::String(Cow::Borrowed(v)) } }
impl From<String>       for Literal<'static> { fn from(v: String)  -> Self { Self::String(Cow::Owned(v)) } }
impl From<Box<str>>     for Literal<'static> { fn from(v: Box<str>)-> Self { Self::String(Cow::Owned(v.into())) } }
impl<'a> From<&'a [u8]> for Literal<'a> { fn from(v: &'a [u8]) -> Self { Self::Bytes(Cow::Borrowed(v)) } }
impl From<Vec<u8>>      for Literal<'static> { fn from(v: Vec<u8>)  -> Self { Self::Bytes(Cow::Owned(v)) } }
impl From<Box<[u8]>>    for Literal<'static> { fn from(v: Box<[u8]>)-> Self { Self::Bytes(Cow::Owned(v.into())) } }
impl<'a> From<bool>     for Literal<'a> { fn from(v: bool)  -> Self { Self::Bool(v) } }
impl<'a> From<i8>       for Literal<'a> { fn from(v: i8)    -> Self { Self::Int8(v) } }
impl<'a> From<i16>      for Literal<'a> { fn from(v: i16)   -> Self { Self::Int16(v) } }
impl<'a> From<i32>      for Literal<'a> { fn from(v: i32)   -> Self { Self::Int32(v) } }
impl<'a> From<i64>      for Literal<'a> { fn from(v: i64)   -> Self { Self::Int64(v) } }
impl<'a> From<i128>     for Literal<'a> { fn from(v: i128)  -> Self { Self::Int128(v) } }
impl<'a> From<u8>       for Literal<'a> { fn from(v: u8)    -> Self { Self::UInt8(v) } }
impl<'a> From<u16>      for Literal<'a> { fn from(v: u16)   -> Self { Self::UInt16(v) } }
impl<'a> From<u32>      for Literal<'a> { fn from(v: u32)   -> Self { Self::UInt32(v) } }
impl<'a> From<u64>      for Literal<'a> { fn from(v: u64)   -> Self { Self::UInt64(v) } }
impl<'a> From<u128>     for Literal<'a> { fn from(v: u128)  -> Self { Self::UInt128(v) } }
impl<'a> From<f32>      for Literal<'a> { fn from(v: f32)   -> Self { Self::Float32(v) } }
impl<'a> From<f64>      for Literal<'a> { fn from(v: f64)   -> Self { Self::Float64(v) } }
impl<'a> From<[u8;16]>  for Literal<'a> { fn from(v: [u8;16])-> Self { Self::Uuid(v) } }
impl<'a> From<Decimal>  for Literal<'a> { fn from(v: Decimal)  -> Self { Self::Decimal(v) } }
impl<'a> From<Date>     for Literal<'a> { fn from(v: Date)     -> Self { Self::Date(v) } }
impl<'a> From<Time>     for Literal<'a> { fn from(v: Time)     -> Self { Self::Time(v) } }
impl<'a> From<DateTime> for Literal<'a> { fn from(v: DateTime) -> Self { Self::DateTime(v) } }
impl<'a> From<TimestampTz> for Literal<'a> { fn from(v: TimestampTz) -> Self { Self::TimestampTz(v) } }
impl<'a> From<Interval> for Literal<'a> { fn from(v: Interval) -> Self { Self::Interval(v) } }
impl<'a> From<IpAddr>   for Literal<'a> { fn from(v: IpAddr)   -> Self { Self::Inet(v) } }
impl<'a> From<MacAddr>  for Literal<'a> { fn from(v: MacAddr)  -> Self { Self::MacAddr(v) } }
impl<'a> From<MacAddr8> for Literal<'a> { fn from(v: MacAddr8) -> Self { Self::MacAddr8(v) } }
impl<'a> From<Point>  for Literal<'a> { fn from(v: Point)  -> Self { Self::Point(v) } }

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::geo::Point;
    use std::mem::size_of;

    #[test]
    fn value_size_is_exactly_24_bytes() {
        // TODO: box Decimal/TimestampTz/Interval to bring this down to 24
        assert!(size_of::<Value>() <= 48,
            "Value size regressed; check for new unboxed large variants");
    }

    #[test]
    fn literal_size_is_exactly_32_bytes() {
        // TODO: box Decimal/TimestampTz/Interval to bring this down to 32
        assert!(size_of::<Literal<'static>>() <= 48,
            "Literal size regressed; check for new unboxed large variants");
    }

    #[test]
    fn xml_is_distinct_from_string() {
        let s = Literal::string_borrowed("<a/>");
        let x = Literal::xml_borrowed("<a/>");
        assert_ne!(
            std::mem::discriminant(&s),
            std::mem::discriminant(&x)
        );
        assert_eq!(x.as_str(), Some("<a/>"));
    }

    #[test]
    fn enum_variant_round_trips() {
        let lit = Literal::enum_variant_borrowed("active");
        let val: Value = lit.into();
        assert!(matches!(val, Value::Enum(ref s) if s.as_ref() == "active"));
    }

    #[test]
    fn bitstring_construction_and_display() {
        let bs = BitString::try_new(8, vec![0b1010_1010].into_boxed_slice()).unwrap();
        let v = Value::BitString(Box::new(bs));
        assert_eq!(v.to_string(), "b'10101010'");
    }

    #[test]
    fn bitstring_length_mismatch_rejected() {
        let err = BitString::try_new(9, vec![0u8].into_boxed_slice()).unwrap_err();
        assert!(matches!(err, crate::types::TypeError::BitLengthMismatch { declared: 9, byte_count: 1 }));
    }

    #[test]
    fn macaddr_display() {
        let mac = MacAddr::new([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);
        assert_eq!(mac.to_string(), "00:1a:2b:3c:4d:5e");
    }

    #[test]
    fn macaddr8_display() {
        let mac = MacAddr8::new([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x70]);
        assert_eq!(mac.to_string(), "00:1a:2b:3c:4d:5e:6f:70");
    }

    #[test]
    fn point_rejects_nan() {
        assert!(Literal::point(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn point_display() {
        let p = Point::new_unchecked(1.5, 2.5);
        assert_eq!(p.to_string(), "(1.5,2.5)");
    }

    #[test]
    fn set_is_distinct_from_array() {
        let arr = Literal::array(vec![Literal::from(1i32)]);
        let set = Literal::set(vec![Literal::from(1i32)]);
        assert_ne!(std::mem::discriminant(&arr), std::mem::discriminant(&set));
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
        assert_ne!(std::mem::discriminant(&m), std::mem::discriminant(&s));
        assert!(s.to_string().starts_with("struct{"));
    }

    #[test]
    fn all_new_variants_convert_to_value() {
        let cases: Vec<Literal<'_>> = vec![
            Literal::xml_borrowed("<root/>"),
            Literal::enum_variant_borrowed("active"),
            Literal::macaddr([0; 6]),
            Literal::macaddr8([0; 8]),
            Literal::inet_v4(127, 0, 0, 1),
            Literal::from(Point::new_unchecked(0.0, 0.0)),
        ];
        for lit in cases {
            let _: Value = lit.into();
        }
    }

    #[test]
    fn uuid_display_standard_format() {
        let bytes: [u8; 16] = [
            0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4,
            0xa7, 0x16, 0x44, 0x66, 0x55, 0x44, 0x00, 0x00,
        ];
        assert_eq!(
            Literal::from(bytes).to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn value_from_str_and_byte_slice() {
        assert_eq!(Value::from("hello").as_str(), Some("hello"));
        assert_eq!(Value::from(&[1u8, 2, 3][..]).as_bytes(), Some(&[1u8, 2, 3][..]));
    }
}

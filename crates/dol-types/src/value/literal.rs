//! Literal AST nodes and their conversions.
//!
//! `Literal<'a>` is the borrowed-or-owned literal form used inside the
//! expression AST. It is kept in a sibling file to keep `value/mod.rs`
//! focused on the runtime [`Value`] enum and its `From` impls.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Bound;

use super::super::binary::BitString;
use super::super::datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
use super::super::error::TypeError;
use super::super::geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
use super::super::network::{IpAddr, MacAddr, MacAddr8};
use super::super::numeric::Decimal;
use super::{LiteralRange, Value, ValueRange, fmt_uuid};

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
    Decimal(Box<Decimal>),

    // ── Network ──
    Inet(IpAddr),
    MacAddr(MacAddr),
    MacAddr8(MacAddr8),

    // ── Temporal ──
    Date(Date),
    Time(Time),
    DateTime(DateTime),
    TimestampTz(Box<TimestampTz>),
    Interval(Box<Interval>),

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

    // ── Extension ──
    /// A domain-specific extension value not covered by the well-known variants.
    /// Boxed to keep `Literal` at 32 bytes on 64-bit targets.
    Extension(Box<(Box<str>, Box<[u8]>)>),
}

impl<'a> Literal<'a> {
    // ── Primitive constructors ──

    pub const fn null() -> Self {
        Self::Null
    }
    pub const fn bool(v: bool) -> Self {
        Self::Bool(v)
    }

    // ── Text constructors ──

    pub fn string_borrowed(v: &'a str) -> Self {
        Self::String(Cow::Borrowed(v))
    }
    pub fn string_owned(v: String) -> Self {
        Self::String(Cow::Owned(v))
    }
    pub fn json_borrowed(v: &'a str) -> Self {
        Self::Json(Cow::Borrowed(v))
    }
    pub fn json_owned(v: String) -> Self {
        Self::Json(Cow::Owned(v))
    }
    pub fn xml_borrowed(v: &'a str) -> Self {
        Self::Xml(Cow::Borrowed(v))
    }
    pub fn xml_owned(v: String) -> Self {
        Self::Xml(Cow::Owned(v))
    }
    pub fn enum_variant_borrowed(v: &'a str) -> Self {
        Self::Enum(Cow::Borrowed(v))
    }
    pub fn enum_variant_owned(v: String) -> Self {
        Self::Enum(Cow::Owned(v))
    }

    // ── Binary constructors ──

    pub fn bytes_borrowed(v: &'a [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(v))
    }
    pub fn bytes_owned(v: Vec<u8>) -> Self {
        Self::Bytes(Cow::Owned(v))
    }
    pub const fn uuid(b: [u8; 16]) -> Self {
        Self::Uuid(b)
    }

    // ── Validated float constructors ──

    pub fn float32(v: f32) -> Result<Self, TypeError> {
        if !v.is_finite() {
            return Err(TypeError::NonFiniteFloat);
        }
        Ok(Self::Float32(v))
    }

    pub fn float64(v: f64) -> Result<Self, TypeError> {
        if !v.is_finite() {
            return Err(TypeError::NonFiniteFloat);
        }
        Ok(Self::Float64(v))
    }

    // ── Numeric constructors ──

    pub fn decimal(unscaled: i128, scale: u32) -> Result<Self, TypeError> {
        match Decimal::try_new(unscaled, scale) {
            Ok(d) => Ok(Self::Decimal(Box::new(d))),
            Err(e) => Err(e),
        }
    }

    // ── Temporal constructors ──

    pub fn date(year: i32, month: u8, day: u8) -> Result<Self, TypeError> {
        match Date::try_new(year, month, day) {
            Ok(d) => Ok(Self::Date(d)),
            Err(e) => Err(e),
        }
    }

    pub fn time(h: u8, m: u8, s: u8, ns: u32) -> Result<Self, TypeError> {
        match Time::try_new(h, m, s, ns) {
            Ok(t) => Ok(Self::Time(t)),
            Err(e) => Err(e),
        }
    }

    pub const fn datetime(date: Date, time: Time) -> Self {
        Self::DateTime(DateTime::new(date, time))
    }

    pub fn timestamp_tz(datetime: DateTime, offset: Offset) -> Self {
        Self::TimestampTz(Box::new(TimestampTz::new(datetime, offset)))
    }

    pub fn interval(months: i32, days: i32, nanoseconds: i64) -> Self {
        Self::Interval(Box::new(Interval::new(months, days, nanoseconds)))
    }

    // ── Network constructors ──

    pub const fn inet_v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self::Inet(IpAddr::v4(a, b, c, d))
    }

    pub const fn inet_v6(bytes: [u8; 16]) -> Self {
        Self::Inet(IpAddr::v6(bytes))
    }

    pub const fn macaddr(b: [u8; 6]) -> Self {
        Self::MacAddr(MacAddr::new(b))
    }
    pub const fn macaddr8(b: [u8; 8]) -> Self {
        Self::MacAddr8(MacAddr8::new(b))
    }

    // ── Geometric constructors ──

    pub fn point(x: f64, y: f64) -> Result<Self, TypeError> {
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

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int8(_)
                | Self::Int16(_)
                | Self::Int32(_)
                | Self::Int64(_)
                | Self::Int128(_)
                | Self::UInt8(_)
                | Self::UInt16(_)
                | Self::UInt32(_)
                | Self::UInt64(_)
                | Self::UInt128(_)
        )
    }
    pub fn is_textual(&self) -> bool {
        matches!(
            self,
            Self::String(_) | Self::Json(_) | Self::Xml(_) | Self::Enum(_)
        )
    }
    pub fn is_datetime(&self) -> bool {
        matches!(
            self,
            Self::Date(_)
                | Self::Time(_)
                | Self::DateTime(_)
                | Self::TimestampTz(_)
                | Self::Interval(_)
        )
    }
    pub fn is_geometric(&self) -> bool {
        matches!(
            self,
            Self::Point(_)
                | Self::Line(_)
                | Self::Segment(_)
                | Self::Rect(_)
                | Self::Circle(_)
                | Self::Path(_)
                | Self::Polygon(_)
        )
    }

    // ── Accessors ──

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) | Self::Json(v) | Self::Xml(v) | Self::Enum(v) => Some(v.as_ref()),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(v) => Some(v.as_ref()),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_uuid(&self) -> Option<[u8; 16]> {
        if let Self::Uuid(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_decimal(&self) -> Option<Decimal> {
        if let Self::Decimal(v) = self {
            Some(**v)
        } else {
            None
        }
    }
    pub fn as_date(&self) -> Option<Date> {
        if let Self::Date(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_time(&self) -> Option<Time> {
        if let Self::Time(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_inet(&self) -> Option<IpAddr> {
        if let Self::Inet(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&[Literal<'a>]> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }

    // ── Conversion ──

    pub fn into_owned(self) -> Value {
        self.into()
    }

    /// Convert any borrowed string data to owned, erasing the lifetime.
    ///
    /// Used when lowering `Expr<'a>` into the lifetime-free expression arena.
    pub fn into_static(self) -> Literal<'static> {
        match self {
            Self::Null => Literal::Null,
            Self::Bool(v) => Literal::Bool(v),

            Self::String(s) => Literal::String(Cow::Owned(s.into_owned())),
            Self::Json(s) => Literal::Json(Cow::Owned(s.into_owned())),
            Self::Xml(s) => Literal::Xml(Cow::Owned(s.into_owned())),
            Self::Enum(s) => Literal::Enum(Cow::Owned(s.into_owned())),

            Self::Bytes(b) => Literal::Bytes(Cow::Owned(b.into_owned())),
            Self::Uuid(b) => Literal::Uuid(b),
            Self::BitString(b) => Literal::BitString(b),

            Self::Int8(v) => Literal::Int8(v),
            Self::Int16(v) => Literal::Int16(v),
            Self::Int32(v) => Literal::Int32(v),
            Self::Int64(v) => Literal::Int64(v),
            Self::Int128(v) => Literal::Int128(v),
            Self::UInt8(v) => Literal::UInt8(v),
            Self::UInt16(v) => Literal::UInt16(v),
            Self::UInt32(v) => Literal::UInt32(v),
            Self::UInt64(v) => Literal::UInt64(v),
            Self::UInt128(v) => Literal::UInt128(v),

            Self::Float32(v) => Literal::Float32(v),
            Self::Float64(v) => Literal::Float64(v),

            Self::Decimal(d) => Literal::Decimal(d),
            Self::Inet(v) => Literal::Inet(v),
            Self::MacAddr(v) => Literal::MacAddr(v),
            Self::MacAddr8(v) => Literal::MacAddr8(v),
            Self::Date(v) => Literal::Date(v),
            Self::Time(v) => Literal::Time(v),
            Self::DateTime(v) => Literal::DateTime(v),
            Self::TimestampTz(v) => Literal::TimestampTz(v),
            Self::Interval(v) => Literal::Interval(v),

            Self::Point(v) => Literal::Point(v),
            Self::Line(v) => Literal::Line(v),
            Self::Segment(v) => Literal::Segment(v),
            Self::Rect(v) => Literal::Rect(v),
            Self::Circle(v) => Literal::Circle(v),
            Self::Path(v) => Literal::Path(v),
            Self::Polygon(v) => Literal::Polygon(v),

            Self::Array(a) => Literal::Array(
                Vec::from(a)
                    .into_iter()
                    .map(Literal::into_static)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            Self::Set(a) => Literal::Set(
                Vec::from(a)
                    .into_iter()
                    .map(Literal::into_static)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            Self::Tuple(a) => Literal::Tuple(
                Vec::from(a)
                    .into_iter()
                    .map(Literal::into_static)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            Self::Map(m) => Literal::Map(
                Vec::from(m)
                    .into_iter()
                    .map(|(k, v)| (Cow::Owned(k.into_owned()), v.into_static()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            Self::Struct(m) => Literal::Struct(
                Vec::from(m)
                    .into_iter()
                    .map(|(k, v)| (Cow::Owned(k.into_owned()), v.into_static()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            Self::Range(r) => Literal::Range(Box::new((*r).into_static())),
            Self::Extension(e) => Literal::Extension(e),
        }
    }
}

impl<'a> fmt::Display for Literal<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "\"{v}\""),
            Self::Json(v) => write!(f, "json({v})"),
            Self::Xml(v) => write!(f, "xml({v})"),
            Self::Enum(v) => write!(f, "'{v}'"),
            Self::Bytes(v) => write!(f, "bytes(len={})", v.len()),
            Self::Uuid(v) => fmt_uuid(v, f),
            Self::BitString(v) => write!(f, "{v}"),
            Self::Int8(v) => write!(f, "{v}i8"),
            Self::Int16(v) => write!(f, "{v}i16"),
            Self::Int32(v) => write!(f, "{v}i32"),
            Self::Int64(v) => write!(f, "{v}i64"),
            Self::Int128(v) => write!(f, "{v}i128"),
            Self::UInt8(v) => write!(f, "{v}u8"),
            Self::UInt16(v) => write!(f, "{v}u16"),
            Self::UInt32(v) => write!(f, "{v}u32"),
            Self::UInt64(v) => write!(f, "{v}u64"),
            Self::UInt128(v) => write!(f, "{v}u128"),
            Self::Float32(v) => write!(f, "{v}f32"),
            Self::Float64(v) => write!(f, "{v}f64"),
            Self::Decimal(v) => write!(f, "{v}"),
            Self::Inet(v) => write!(f, "{v}"),
            Self::MacAddr(v) => write!(f, "{v}"),
            Self::MacAddr8(v) => write!(f, "{v}"),
            Self::Date(v) => write!(f, "{v}"),
            Self::Time(v) => write!(f, "{v}"),
            Self::DateTime(v) => write!(f, "{v}"),
            Self::TimestampTz(v) => write!(f, "{v}"),
            Self::Interval(v) => write!(f, "{v}"),
            Self::Point(v) => write!(f, "{v}"),
            Self::Line(v) => write!(f, "{v}"),
            Self::Segment(v) => write!(f, "{v}"),
            Self::Rect(v) => write!(f, "{v}"),
            Self::Circle(v) => write!(f, "{v}"),
            Self::Path(v) => write!(f, "{v}"),
            Self::Polygon(v) => write!(f, "{v}"),
            Self::Array(vs) | Self::Set(vs) | Self::Tuple(vs) => {
                let tag = match self {
                    Self::Set(_) => "set",
                    Self::Tuple(_) => "tuple",
                    _ => "",
                };
                if !tag.is_empty() {
                    write!(f, "{tag}")?;
                }
                f.write_str("[")?;
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Self::Map(entries) | Self::Struct(entries) => {
                let tag = match self {
                    Self::Struct(_) => "struct",
                    _ => "",
                };
                if !tag.is_empty() {
                    write!(f, "{tag}")?;
                }
                f.write_str("{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "\"{k}\": {v}")?;
                }
                f.write_str("}")
            }
            Self::Range(r) => write!(f, "{r}"),
            Self::Extension(inner) => {
                write!(f, "ext:{}(len={})", inner.0, inner.1.len())
            }
        }
    }
}

// ─── From<Literal<'a>> for Value ─────────────────────────────────────────────

impl<'a> From<Literal<'a>> for Value {
    fn from(lit: Literal<'a>) -> Self {
        match lit {
            Literal::Null => Self::Null,
            Literal::Bool(v) => Self::Bool(v),
            Literal::String(v) => Self::String(v.into_owned().into_boxed_str()),
            Literal::Json(v) => Self::Json(v.into_owned().into_boxed_str()),
            Literal::Xml(v) => Self::Xml(v.into_owned().into_boxed_str()),
            Literal::Enum(v) => Self::Enum(v.into_owned().into_boxed_str()),
            Literal::Bytes(v) => Self::Bytes(v.into_owned().into_boxed_slice()),
            Literal::Uuid(v) => Self::Uuid(v),
            Literal::BitString(v) => Self::BitString(v),
            Literal::Int8(v) => Self::Int8(v),
            Literal::Int16(v) => Self::Int16(v),
            Literal::Int32(v) => Self::Int32(v),
            Literal::Int64(v) => Self::Int64(v),
            Literal::Int128(v) => Self::Int128(Box::new(v)),
            Literal::UInt8(v) => Self::UInt8(v),
            Literal::UInt16(v) => Self::UInt16(v),
            Literal::UInt32(v) => Self::UInt32(v),
            Literal::UInt64(v) => Self::UInt64(v),
            Literal::UInt128(v) => Self::UInt128(Box::new(v)),
            Literal::Float32(v) => Self::Float32(v),
            Literal::Float64(v) => Self::Float64(v),
            Literal::Decimal(v) => Self::Decimal(v),
            Literal::Inet(v) => Self::Inet(v),
            Literal::MacAddr(v) => Self::MacAddr(v),
            Literal::MacAddr8(v) => Self::MacAddr8(v),
            Literal::Date(v) => Self::Date(v),
            Literal::Time(v) => Self::Time(v),
            Literal::DateTime(v) => Self::DateTime(v),
            Literal::TimestampTz(v) => Self::TimestampTz(v),
            Literal::Interval(v) => Self::Interval(v),
            Literal::Point(v) => Self::Point(v),
            Literal::Line(v) => Self::Line(v),
            Literal::Segment(v) => Self::Segment(v),
            Literal::Rect(v) => Self::Rect(v),
            Literal::Circle(v) => Self::Circle(v),
            Literal::Path(v) => Self::Path(v),
            Literal::Polygon(v) => Self::Polygon(v),
            Literal::Array(vs) => Self::Array(own_slice(vs)),
            Literal::Set(vs) => Self::Set(own_slice(vs)),
            Literal::Tuple(vs) => Self::Tuple(own_slice(vs)),
            Literal::Map(entries) => Self::Map(own_kv_slice(entries)),
            Literal::Struct(entries) => Self::Struct(own_kv_slice(entries)),
            Literal::Range(r) => Self::Range(convert_range(*r)),
            Literal::Extension(inner) => Self::Extension(inner),
        }
    }
}

fn own_slice<'a>(s: Box<[Literal<'a>]>) -> Box<[Value]> {
    Vec::from(s)
        .into_iter()
        .map(Value::from)
        .collect::<Vec<_>>()
        .into_boxed_slice()
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
            Bound::Unbounded => Bound::Unbounded,
        }
    };
    Box::new(ValueRange {
        start: cvt(r.start),
        end: cvt(r.end),
    })
}

// ─── From impls for Literal<'a> ──────────────────────────────────────────────

impl<'a> From<&'a str> for Literal<'a> {
    fn from(v: &'a str) -> Self {
        Self::String(Cow::Borrowed(v))
    }
}
impl From<String> for Literal<'static> {
    fn from(v: String) -> Self {
        Self::String(Cow::Owned(v))
    }
}
impl From<Box<str>> for Literal<'static> {
    fn from(v: Box<str>) -> Self {
        Self::String(Cow::Owned(v.into()))
    }
}
impl<'a> From<&'a [u8]> for Literal<'a> {
    fn from(v: &'a [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(v))
    }
}
impl From<Vec<u8>> for Literal<'static> {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(Cow::Owned(v))
    }
}
impl From<Box<[u8]>> for Literal<'static> {
    fn from(v: Box<[u8]>) -> Self {
        Self::Bytes(Cow::Owned(v.into()))
    }
}
impl<'a> From<bool> for Literal<'a> {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl<'a> From<i8> for Literal<'a> {
    fn from(v: i8) -> Self {
        Self::Int8(v)
    }
}
impl<'a> From<i16> for Literal<'a> {
    fn from(v: i16) -> Self {
        Self::Int16(v)
    }
}
impl<'a> From<i32> for Literal<'a> {
    fn from(v: i32) -> Self {
        Self::Int32(v)
    }
}
impl<'a> From<i64> for Literal<'a> {
    fn from(v: i64) -> Self {
        Self::Int64(v)
    }
}
impl<'a> From<i128> for Literal<'a> {
    fn from(v: i128) -> Self {
        Self::Int128(v)
    }
}
impl<'a> From<u8> for Literal<'a> {
    fn from(v: u8) -> Self {
        Self::UInt8(v)
    }
}
impl<'a> From<u16> for Literal<'a> {
    fn from(v: u16) -> Self {
        Self::UInt16(v)
    }
}
impl<'a> From<u32> for Literal<'a> {
    fn from(v: u32) -> Self {
        Self::UInt32(v)
    }
}
impl<'a> From<u64> for Literal<'a> {
    fn from(v: u64) -> Self {
        Self::UInt64(v)
    }
}
impl<'a> From<u128> for Literal<'a> {
    fn from(v: u128) -> Self {
        Self::UInt128(v)
    }
}
impl<'a> From<f32> for Literal<'a> {
    fn from(v: f32) -> Self {
        Self::Float32(v)
    }
}
impl<'a> From<f64> for Literal<'a> {
    fn from(v: f64) -> Self {
        Self::Float64(v)
    }
}
impl<'a> From<[u8; 16]> for Literal<'a> {
    fn from(v: [u8; 16]) -> Self {
        Self::Uuid(v)
    }
}
impl<'a> From<Decimal> for Literal<'a> {
    fn from(v: Decimal) -> Self {
        Self::Decimal(Box::new(v))
    }
}
impl<'a> From<Date> for Literal<'a> {
    fn from(v: Date) -> Self {
        Self::Date(v)
    }
}
impl<'a> From<Time> for Literal<'a> {
    fn from(v: Time) -> Self {
        Self::Time(v)
    }
}
impl<'a> From<DateTime> for Literal<'a> {
    fn from(v: DateTime) -> Self {
        Self::DateTime(v)
    }
}
impl<'a> From<TimestampTz> for Literal<'a> {
    fn from(v: TimestampTz) -> Self {
        Self::TimestampTz(Box::new(v))
    }
}
impl<'a> From<Interval> for Literal<'a> {
    fn from(v: Interval) -> Self {
        Self::Interval(Box::new(v))
    }
}
impl<'a> From<IpAddr> for Literal<'a> {
    fn from(v: IpAddr) -> Self {
        Self::Inet(v)
    }
}
impl<'a> From<MacAddr> for Literal<'a> {
    fn from(v: MacAddr) -> Self {
        Self::MacAddr(v)
    }
}
impl<'a> From<MacAddr8> for Literal<'a> {
    fn from(v: MacAddr8) -> Self {
        Self::MacAddr8(v)
    }
}
impl<'a> From<Point> for Literal<'a> {
    fn from(v: Point) -> Self {
        Self::Point(v)
    }
}

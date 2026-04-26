//! [`Literal`] constructors and predicates/accessors.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Bound;

use super::{Literal, LiteralRange};
use crate::error::TypeError;
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::Point;
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;

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

    #[cfg(feature = "numeric")]
    pub fn decimal(unscaled: i128, scale: u32) -> Result<Self, TypeError> {
        match Decimal::try_new(unscaled, scale) {
            Ok(d) => Ok(Self::Decimal(Box::new(d))),
            Err(e) => Err(e),
        }
    }

    // ── Temporal constructors ──

    #[cfg(feature = "datetime")]
    pub fn date(year: i32, month: u8, day: u8) -> Result<Self, TypeError> {
        match Date::try_new(year, month, day) {
            Ok(d) => Ok(Self::Date(d)),
            Err(e) => Err(e),
        }
    }

    #[cfg(feature = "datetime")]
    pub fn time(h: u8, m: u8, s: u8, ns: u32) -> Result<Self, TypeError> {
        match Time::try_new(h, m, s, ns) {
            Ok(t) => Ok(Self::Time(t)),
            Err(e) => Err(e),
        }
    }

    #[cfg(feature = "datetime")]
    pub const fn datetime(date: Date, time: Time) -> Self {
        Self::DateTime(DateTime::new(date, time))
    }

    #[cfg(feature = "datetime")]
    pub fn timestamp_tz(datetime: DateTime, offset: Offset) -> Self {
        Self::TimestampTz(Box::new(TimestampTz::new(datetime, offset)))
    }

    #[cfg(feature = "datetime")]
    pub fn interval(months: i32, days: i32, nanoseconds: i64) -> Self {
        Self::Interval(Box::new(Interval::new(months, days, nanoseconds)))
    }

    // ── Network constructors ──

    #[cfg(feature = "network")]
    pub const fn inet_v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self::Inet(IpAddr::v4(a, b, c, d))
    }

    #[cfg(feature = "network")]
    pub const fn inet_v6(bytes: [u8; 16]) -> Self {
        Self::Inet(IpAddr::v6(bytes))
    }

    #[cfg(feature = "network")]
    pub const fn macaddr_eui48(b: [u8; 6]) -> Self {
        Self::MacAddr(MacAddr::eui48(b))
    }
    #[cfg(feature = "network")]
    pub const fn macaddr_eui64(b: [u8; 8]) -> Self {
        Self::MacAddr(MacAddr::eui64(b))
    }

    // ── Geometric constructors ──

    #[cfg(feature = "geo")]
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
    #[cfg(feature = "datetime")]
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
    #[cfg(not(feature = "datetime"))]
    pub fn is_datetime(&self) -> bool {
        false
    }
    #[cfg(feature = "geo")]
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
    #[cfg(not(feature = "geo"))]
    pub fn is_geometric(&self) -> bool {
        false
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
    #[cfg(feature = "numeric")]
    pub fn as_decimal(&self) -> Option<Decimal> {
        if let Self::Decimal(v) = self {
            Some(**v)
        } else {
            None
        }
    }
    #[cfg(feature = "datetime")]
    pub fn as_date(&self) -> Option<Date> {
        if let Self::Date(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    #[cfg(feature = "datetime")]
    pub fn as_time(&self) -> Option<Time> {
        if let Self::Time(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    #[cfg(feature = "network")]
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
}

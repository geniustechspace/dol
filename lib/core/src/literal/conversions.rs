//! Conversions involving [`Literal`]:
//! - [`Literal::into_owned`] / [`Literal::into_static`] — lifetime erasure.
//! - `From<Literal<'_>> for Value` — owned upgrade.
//! - `From<X> for Literal<'_>` — primitive payload constructors.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Bound;

use super::{Literal, LiteralRange};
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::Point;
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;
use crate::value::{Value, ValueRange};

impl<'a> Literal<'a> {
    /// Promote this borrowed literal into an owned [`Value`].
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

            #[cfg(feature = "numeric")]
            Self::Decimal(d) => Literal::Decimal(d),
            #[cfg(feature = "network")]
            Self::Inet(v) => Literal::Inet(v),
            #[cfg(feature = "network")]
            Self::MacAddr(v) => Literal::MacAddr(v),
            #[cfg(feature = "datetime")]
            Self::Date(v) => Literal::Date(v),
            #[cfg(feature = "datetime")]
            Self::Time(v) => Literal::Time(v),
            #[cfg(feature = "datetime")]
            Self::DateTime(v) => Literal::DateTime(v),
            #[cfg(feature = "datetime")]
            Self::TimestampTz(v) => Literal::TimestampTz(v),
            #[cfg(feature = "datetime")]
            Self::Interval(v) => Literal::Interval(v),

            #[cfg(feature = "geo")]
            Self::Point(v) => Literal::Point(v),
            #[cfg(feature = "geo")]
            Self::Line(v) => Literal::Line(v),
            #[cfg(feature = "geo")]
            Self::Segment(v) => Literal::Segment(v),
            #[cfg(feature = "geo")]
            Self::Rect(v) => Literal::Rect(v),
            #[cfg(feature = "geo")]
            Self::Circle(v) => Literal::Circle(v),
            #[cfg(feature = "geo")]
            Self::Path(v) => Literal::Path(v),
            #[cfg(feature = "geo")]
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
            #[cfg(feature = "numeric")]
            Literal::Decimal(v) => Self::Decimal(v),
            #[cfg(feature = "network")]
            Literal::Inet(v) => Self::Inet(v),
            #[cfg(feature = "network")]
            Literal::MacAddr(v) => Self::MacAddr(v),
            #[cfg(feature = "datetime")]
            Literal::Date(v) => Self::Date(v),
            #[cfg(feature = "datetime")]
            Literal::Time(v) => Self::Time(v),
            #[cfg(feature = "datetime")]
            Literal::DateTime(v) => Self::DateTime(v),
            #[cfg(feature = "datetime")]
            Literal::TimestampTz(v) => Self::TimestampTz(v),
            #[cfg(feature = "datetime")]
            Literal::Interval(v) => Self::Interval(v),
            #[cfg(feature = "geo")]
            Literal::Point(v) => Self::Point(v),
            #[cfg(feature = "geo")]
            Literal::Line(v) => Self::Line(v),
            #[cfg(feature = "geo")]
            Literal::Segment(v) => Self::Segment(v),
            #[cfg(feature = "geo")]
            Literal::Rect(v) => Self::Rect(v),
            #[cfg(feature = "geo")]
            Literal::Circle(v) => Self::Circle(v),
            #[cfg(feature = "geo")]
            Literal::Path(v) => Self::Path(v),
            #[cfg(feature = "geo")]
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
#[cfg(feature = "numeric")]
impl<'a> From<Decimal> for Literal<'a> {
    fn from(v: Decimal) -> Self {
        Self::Decimal(Box::new(v))
    }
}
#[cfg(feature = "datetime")]
impl<'a> From<Date> for Literal<'a> {
    fn from(v: Date) -> Self {
        Self::Date(v)
    }
}
#[cfg(feature = "datetime")]
impl<'a> From<Time> for Literal<'a> {
    fn from(v: Time) -> Self {
        Self::Time(v)
    }
}
#[cfg(feature = "datetime")]
impl<'a> From<DateTime> for Literal<'a> {
    fn from(v: DateTime) -> Self {
        Self::DateTime(v)
    }
}
#[cfg(feature = "datetime")]
impl<'a> From<TimestampTz> for Literal<'a> {
    fn from(v: TimestampTz) -> Self {
        Self::TimestampTz(Box::new(v))
    }
}
#[cfg(feature = "datetime")]
impl<'a> From<Interval> for Literal<'a> {
    fn from(v: Interval) -> Self {
        Self::Interval(Box::new(v))
    }
}
#[cfg(feature = "network")]
impl<'a> From<IpAddr> for Literal<'a> {
    fn from(v: IpAddr) -> Self {
        Self::Inet(v)
    }
}
#[cfg(feature = "network")]
impl<'a> From<MacAddr> for Literal<'a> {
    fn from(v: MacAddr) -> Self {
        Self::MacAddr(v)
    }
}
#[cfg(feature = "geo")]
impl<'a> From<Point> for Literal<'a> {
    fn from(v: Point) -> Self {
        Self::Point(v)
    }
}

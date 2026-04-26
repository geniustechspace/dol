//! `impl From<X> for Value` conversions for primitive and validated payloads.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::Value;
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::Point;
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.into())
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(v.into_boxed_str())
    }
}
impl From<Box<str>> for Value {
    fn from(v: Box<str>) -> Self {
        Self::String(v)
    }
}
impl From<&[u8]> for Value {
    fn from(v: &[u8]) -> Self {
        Self::Bytes(v.into())
    }
}
impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(v.into_boxed_slice())
    }
}
impl From<Box<[u8]>> for Value {
    fn from(v: Box<[u8]>) -> Self {
        Self::Bytes(v)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Self::Int8(v)
    }
}
impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Self::Int16(v)
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Int32(v)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Int64(v)
    }
}
impl From<i128> for Value {
    fn from(v: i128) -> Self {
        Self::Int128(Box::new(v))
    }
}
impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Self::UInt8(v)
    }
}
impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Self::UInt16(v)
    }
}
impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Self::UInt32(v)
    }
}
impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Self::UInt64(v)
    }
}
impl From<u128> for Value {
    fn from(v: u128) -> Self {
        Self::UInt128(Box::new(v))
    }
}
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::Float32(v)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Float64(v)
    }
}
impl From<[u8; 16]> for Value {
    fn from(v: [u8; 16]) -> Self {
        Self::Uuid(v)
    }
}
#[cfg(feature = "numeric")]
impl From<Decimal> for Value {
    fn from(v: Decimal) -> Self {
        Self::Decimal(Box::new(v))
    }
}
#[cfg(feature = "datetime")]
impl From<Date> for Value {
    fn from(v: Date) -> Self {
        Self::Date(v)
    }
}
#[cfg(feature = "datetime")]
impl From<Time> for Value {
    fn from(v: Time) -> Self {
        Self::Time(v)
    }
}
#[cfg(feature = "datetime")]
impl From<DateTime> for Value {
    fn from(v: DateTime) -> Self {
        Self::DateTime(v)
    }
}
#[cfg(feature = "datetime")]
impl From<TimestampTz> for Value {
    fn from(v: TimestampTz) -> Self {
        Self::TimestampTz(Box::new(v))
    }
}
#[cfg(feature = "datetime")]
impl From<Interval> for Value {
    fn from(v: Interval) -> Self {
        Self::Interval(Box::new(v))
    }
}
#[cfg(feature = "network")]
impl From<IpAddr> for Value {
    fn from(v: IpAddr) -> Self {
        Self::Inet(v)
    }
}
#[cfg(feature = "network")]
impl From<MacAddr> for Value {
    fn from(v: MacAddr) -> Self {
        Self::MacAddr(v)
    }
}
#[cfg(feature = "geo")]
impl From<Point> for Value {
    fn from(v: Point) -> Self {
        Self::Point(v)
    }
}

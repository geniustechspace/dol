//! Typed accessors for [`Value`] (`as_str`, `as_bytes`, `as_bool`, …).

use alloc::boxed::Box;

use super::Value;
use crate::binary::BitString;
#[cfg(feature = "datetime")]
use crate::datetime::{Date, DateTime, Interval, Time, TimestampTz};
#[cfg(feature = "geo")]
use crate::geo::Point;
#[cfg(feature = "network")]
use crate::network::{IpAddr, MacAddr};
#[cfg(feature = "numeric")]
use crate::numeric::Decimal;

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) | Self::Json(v) | Self::Xml(v) | Self::Enum(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(v) => Some(v),
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
    #[cfg(feature = "datetime")]
    pub fn as_datetime(&self) -> Option<DateTime> {
        if let Self::DateTime(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    #[cfg(feature = "datetime")]
    pub fn as_timestamp_tz(&self) -> Option<TimestampTz> {
        if let Self::TimestampTz(v) = self {
            Some(**v)
        } else {
            None
        }
    }
    #[cfg(feature = "datetime")]
    pub fn as_interval(&self) -> Option<Interval> {
        if let Self::Interval(v) = self {
            Some(**v)
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
    #[cfg(feature = "network")]
    pub fn as_macaddr(&self) -> Option<MacAddr> {
        if let Self::MacAddr(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    #[cfg(feature = "geo")]
    pub fn as_point(&self) -> Option<Point> {
        if let Self::Point(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_bitstring(&self) -> Option<&BitString> {
        if let Self::BitString(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_set(&self) -> Option<&[Value]> {
        if let Self::Set(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_tuple(&self) -> Option<&[Value]> {
        if let Self::Tuple(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_map(&self) -> Option<&[(Box<str>, Value)]> {
        if let Self::Map(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_struct(&self) -> Option<&[(Box<str>, Value)]> {
        if let Self::Struct(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

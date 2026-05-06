use super::{super::error::TypeError, DateTime};
use core::fmt;

/// A fixed UTC offset in whole seconds. Valid range: `−86_399..=86_399`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Offset(i32);

impl Offset {
    pub const UTC: Self = Self(0);

    pub const fn try_from_seconds(s: i32) -> Result<Self, TypeError> {
        if s < -86_399 || s > 86_399 {
            return Err(TypeError::TimezoneOffsetOutOfRange(s));
        }
        Ok(Self(s))
    }

    pub const fn try_from_hm(sign: i8, h: u8, m: u8) -> Result<Self, TypeError> {
        Self::try_from_seconds((sign as i32) * ((h as i32) * 3600 + (m as i32) * 60))
    }

    pub const fn new_unchecked(seconds: i32) -> Self {
        Self(seconds)
    }
    pub const fn as_seconds(self) -> i32 {
        self.0
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            return f.write_str("Z");
        }
        let sign = if self.0 < 0 { '-' } else { '+' };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
    }
}

/// Datetime with a fixed UTC offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TimestampTz {
    pub datetime: DateTime,
    pub offset: Offset,
}

impl TimestampTz {
    pub const fn new(datetime: DateTime, offset: Offset) -> Self {
        Self { datetime, offset }
    }

    pub const fn utc(datetime: DateTime) -> Self {
        Self {
            datetime,
            offset: Offset::UTC,
        }
    }
}

impl fmt::Display for TimestampTz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.datetime, self.offset)
    }
}

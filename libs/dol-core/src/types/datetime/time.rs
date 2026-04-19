use core::fmt;
use super::super::error::TypeError;

/// A time of day (no date, no timezone). Nanosecond precision.
/// Second may be `60` for leap seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    /// `0–60`; 60 is reserved for leap seconds.
    pub second: u8,
    /// `0–999_999_999`
    pub nanosecond: u32,
}

impl Time {
    pub const MIDNIGHT: Self = Self { hour: 0, minute: 0, second: 0, nanosecond: 0 };

    pub const fn try_new(h: u8, m: u8, s: u8, ns: u32) -> Result<Self, TypeError> {
        if h  > 23            { return Err(TypeError::InvalidHour(h)); }
        if m  > 59            { return Err(TypeError::InvalidMinute(m)); }
        if s  > 60            { return Err(TypeError::InvalidSecond(s)); }
        if ns > 999_999_999   { return Err(TypeError::InvalidNanosecond(ns)); }
        Ok(Self { hour: h, minute: m, second: s, nanosecond: ns })
    }

    pub const fn new_unchecked(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Self {
        Self { hour, minute, second, nanosecond }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        if self.nanosecond > 0 {
            let s = format!("{:09}", self.nanosecond);
            write!(f, ".{}", s.trim_end_matches('0'))?;
        }
        Ok(())
    }
}

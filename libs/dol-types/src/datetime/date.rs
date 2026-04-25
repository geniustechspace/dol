use super::super::error::TypeError;
use core::fmt;

/// A calendar date (no time, no timezone). Proleptic Gregorian; negative years
/// are BCE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Date {
    pub year: i32,
    /// `1–12`
    pub month: u8,
    /// `1–31` (range-checked, not calendar-exact)
    pub day: u8,
}

impl Date {
    pub const fn try_new(year: i32, month: u8, day: u8) -> Result<Self, TypeError> {
        if month < 1 || month > 12 {
            return Err(TypeError::InvalidMonth(month));
        }
        if day < 1 || day > 31 {
            return Err(TypeError::InvalidDay(day));
        }
        Ok(Self { year, month, day })
    }

    pub const fn new_unchecked(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.year < 0 {
            write!(f, "-{:04}-{:02}-{:02}", -self.year, self.month, self.day)
        } else {
            write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
        }
    }
}

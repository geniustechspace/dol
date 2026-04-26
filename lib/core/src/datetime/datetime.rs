use super::{Date, Time};
use core::fmt;

/// Naive datetime: date + time, no timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl DateTime {
    pub const fn new(date: Date, time: Time) -> Self {
        Self { date, time }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

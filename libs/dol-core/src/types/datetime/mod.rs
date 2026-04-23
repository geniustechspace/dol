//! Date and time types — mirrors Python's `datetime` module.
//!
//! This module is the single home for every date/time type in DOL:
//! [`Date`], [`Time`], [`DateTime`], [`TimestampTz`], [`Offset`], and
//! [`Interval`].
//!
//! # Ergonomic factories
//!
//! Module-level functions construct [`Value`] directly so callers never
//! need to manually wrap primitives:
//!
//! ```rust
//! use dol_core::types::datetime;
//!
//! let d = datetime::from_ymd(2024, 1, 15).unwrap();   // Value::Date
//! let t = datetime::from_hms(9, 30, 0).unwrap();      // Value::Time
//! let iv = datetime::interval_days(30);                // Value::Interval
//! let iv2 = datetime::interval_months(3);              // Value::Interval
//! ```

pub mod date;
#[allow(clippy::module_inception)]
pub mod datetime;
pub mod interval;
pub mod time;
pub mod timestamp;

pub use date::Date;
pub use datetime::DateTime;
pub use interval::Interval;
pub use time::Time;
pub use timestamp::{Offset, TimestampTz};

use super::error::TypeError;
use super::value::Value;

// ─── Factory functions ────────────────────────────────────────────────────────

/// Returns today's UTC date as `Value::Date`.
pub fn today() -> Value {
    let (year, month, day) = utc_date_parts();
    Value::Date(Date::new_unchecked(year, month, day))
}

/// Returns the current UTC datetime (no timezone) as `Value::DateTime`.
pub fn now() -> Value {
    let (year, month, day, hour, minute, second, nano) = utc_datetime_parts();
    let date = Date::new_unchecked(year, month, day);
    let time = Time::new_unchecked(hour, minute, second, nano);
    Value::DateTime(DateTime::new(date, time))
}

/// Returns the current UTC datetime with UTC offset as `Value::TimestampTz`.
pub fn now_tz() -> Value {
    let (year, month, day, hour, minute, second, nano) = utc_datetime_parts();
    let date = Date::new_unchecked(year, month, day);
    let time = Time::new_unchecked(hour, minute, second, nano);
    let dt = DateTime::new(date, time);
    Value::TimestampTz(Box::new(TimestampTz::utc(dt)))
}

/// Constructs a `Value::Date` from year, month, day components.
pub fn from_ymd(year: i32, month: u8, day: u8) -> Result<Value, TypeError> {
    Date::try_new(year, month, day).map(Value::Date)
}

/// Constructs a `Value::Time` from hour, minute, second components (nanoseconds = 0).
pub fn from_hms(hour: u8, minute: u8, second: u8) -> Result<Value, TypeError> {
    Time::try_new(hour, minute, second, 0).map(Value::Time)
}

/// Constructs a `Value::Time` from hour, minute, second, and nanosecond components.
pub fn from_hms_nano(hour: u8, minute: u8, second: u8, nano: u32) -> Result<Value, TypeError> {
    Time::try_new(hour, minute, second, nano).map(Value::Time)
}

/// Constructs a `Value::Interval` spanning the given number of days.
pub fn interval_days(days: i32) -> Value {
    Value::Interval(Box::new(Interval::from_days(days)))
}

/// Constructs a `Value::Interval` spanning the given number of months.
pub fn interval_months(months: i32) -> Value {
    Value::Interval(Box::new(Interval::from_months(months)))
}

/// Constructs a `Value::Interval` spanning the given number of nanoseconds.
pub fn interval_nanos(nanos: i64) -> Value {
    Value::Interval(Box::new(Interval::from_nanos(nanos)))
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Splits a count of days since the Unix epoch (1970-01-01) into
/// `(year, month, day)` using the proleptic Gregorian civil calendar.
///
/// Algorithm by Howard Hinnant (public domain).
fn days_to_ymd(z: i64) -> (i32, u8, u8) {
    let z = z + 719_468;
    let era: i64 = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u8, d as u8)
}

fn utc_date_parts() -> (i32, u8, u8) {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch");
    let days = (duration.as_secs() / 86_400) as i64;
    days_to_ymd(days)
}

fn utc_datetime_parts() -> (i32, u8, u8, u8, u8, u8, u32) {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch");
    let total_secs = duration.as_secs() as i64;
    let nanosecond = duration.subsec_nanos();
    let secs_of_day = (total_secs % 86_400) as u32;
    let days = total_secs / 86_400;
    let (year, month, day) = days_to_ymd(days);
    let hour = (secs_of_day / 3600) as u8;
    let minute = ((secs_of_day % 3600) / 60) as u8;
    let second = (secs_of_day % 60) as u8;
    (year, month, day, hour, minute, second, nanosecond)
}

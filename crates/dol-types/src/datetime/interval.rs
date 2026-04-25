use alloc::format;
use core::fmt;

/// Calendar/time interval: months + days + nanoseconds.
///
/// Three components are stored independently because month lengths vary.
/// This matches the PostgreSQL `INTERVAL`, ISO 8601 duration, and RFC 5545
/// DURATION representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Interval {
    pub months: i32,
    pub days: i32,
    pub nanoseconds: i64,
}

impl Interval {
    pub const ZERO: Self = Self {
        months: 0,
        days: 0,
        nanoseconds: 0,
    };

    pub const fn new(months: i32, days: i32, nanoseconds: i64) -> Self {
        Self {
            months,
            days,
            nanoseconds,
        }
    }

    pub const fn from_months(m: i32) -> Self {
        Self {
            months: m,
            ..Self::ZERO
        }
    }
    pub const fn from_days(d: i32) -> Self {
        Self {
            days: d,
            ..Self::ZERO
        }
    }
    pub const fn from_nanos(ns: i64) -> Self {
        Self {
            nanoseconds: ns,
            ..Self::ZERO
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Interval::ZERO {
            return f.write_str("PT0S");
        }
        f.write_str("P")?;
        if self.months != 0 {
            let (y, m) = (self.months / 12, self.months % 12);
            if y != 0 {
                write!(f, "{y}Y")?;
            }
            if m != 0 {
                write!(f, "{m}M")?;
            }
        }
        if self.days != 0 {
            write!(f, "{}D", self.days)?;
        }
        if self.nanoseconds != 0 {
            let ns = self.nanoseconds.abs();
            let sign = if self.nanoseconds < 0 { "-" } else { "" };
            let h = ns / 3_600_000_000_000_i64;
            let min = (ns % 3_600_000_000_000_i64) / 60_000_000_000_i64;
            let s = (ns % 60_000_000_000_i64) / 1_000_000_000_i64;
            let frac = ns % 1_000_000_000_i64;
            f.write_str("T")?;
            if h != 0 {
                write!(f, "{sign}{h}H")?;
            }
            if min != 0 {
                write!(f, "{sign}{min}M")?;
            }
            if s != 0 || frac != 0 {
                if frac != 0 {
                    write!(
                        f,
                        "{sign}{s}.{}S",
                        format!("{frac:09}").trim_end_matches('0')
                    )?;
                } else {
                    write!(f, "{sign}{s}S")?;
                }
            }
        }
        Ok(())
    }
}

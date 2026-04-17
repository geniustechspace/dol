//! Structural sub-types used by [`super::Value`], [`crate::Literal`], and
//! [`crate::DataType`].
//!
//! Each type here has a validity constraint and therefore provides:
//! - A `try_new` fallible constructor for runtime use.
//! - A `new_unchecked` infallible constructor for known-valid compile-time use.
//!
//! None of these types allocate unless they contain a slice or string payload.

use core::fmt;
use super::TypeError;

// ─── Decimal ─────────────────────────────────────────────────────────────────

/// Fixed-point decimal: `real = unscaled × 10^−scale`.
///
/// Scale is bounded to [`Decimal::MAX_SCALE`] (38), matching the maximum
/// precision of SQL `NUMERIC` and most fixed-point decimal standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Decimal {
    pub unscaled: i128,
    pub scale: u32,
}

impl Decimal {
    /// Maximum scale (38), matching SQL NUMERIC maximum precision.
    pub const MAX_SCALE: u32 = 38;

    pub const fn try_new(unscaled: i128, scale: u32) -> Result<Self, TypeError> {
        if scale > Self::MAX_SCALE {
            return Err(TypeError::DecimalScaleTooLarge { scale });
        }
        Ok(Self { unscaled, scale })
    }

    pub const fn new_unchecked(unscaled: i128, scale: u32) -> Self {
        Self { unscaled, scale }
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(f, "{}", self.unscaled);
        }
        let negative = self.unscaled < 0;
        let abs = self.unscaled.unsigned_abs();
        let scale = self.scale as usize;
        let mut digits = abs.to_string();
        if digits.len() <= scale {
            digits = format!("{:0>width$}", digits, width = scale + 1);
        }
        let split = digits.len() - scale;
        let (int_part, frac_part) = digits.split_at(split);
        if negative { write!(f, "-{int_part}.{frac_part}") }
        else        { write!(f, "{int_part}.{frac_part}") }
    }
}

// ─── Temporal ────────────────────────────────────────────────────────────────

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
        if month < 1 || month > 12 { return Err(TypeError::InvalidMonth(month)); }
        if day   < 1 || day   > 31 { return Err(TypeError::InvalidDay(day)); }
        Ok(Self { year, month, day })
    }

    pub const fn new_unchecked(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.year < 0 { write!(f, "-{:04}-{:02}-{:02}", -self.year, self.month, self.day) }
        else             { write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day) }
    }
}

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

/// A fixed UTC offset in whole seconds. Valid range: `−86_399..=86_399`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    pub const fn new_unchecked(seconds: i32) -> Self { Self(seconds) }
    pub const fn as_seconds(self) -> i32 { self.0 }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 { return f.write_str("Z"); }
        let sign = if self.0 < 0 { '-' } else { '+' };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
    }
}

/// Datetime with a fixed UTC offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimestampTz {
    pub datetime: DateTime,
    pub offset: Offset,
}

impl TimestampTz {
    pub const fn new(datetime: DateTime, offset: Offset) -> Self {
        Self { datetime, offset }
    }

    pub const fn utc(datetime: DateTime) -> Self {
        Self { datetime, offset: Offset::UTC }
    }
}

impl fmt::Display for TimestampTz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.datetime, self.offset)
    }
}

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
    pub const ZERO: Self = Self { months: 0, days: 0, nanoseconds: 0 };

    pub const fn new(months: i32, days: i32, nanoseconds: i64) -> Self {
        Self { months, days, nanoseconds }
    }

    pub const fn from_months(m: i32)  -> Self { Self { months: m, ..Self::ZERO } }
    pub const fn from_days(d: i32)    -> Self { Self { days: d, ..Self::ZERO } }
    pub const fn from_nanos(ns: i64)  -> Self { Self { nanoseconds: ns, ..Self::ZERO } }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Interval::ZERO { return f.write_str("PT0S"); }
        f.write_str("P")?;
        if self.months != 0 {
            let (y, m) = (self.months / 12, self.months % 12);
            if y != 0 { write!(f, "{y}Y")?; }
            if m != 0 { write!(f, "{m}M")?; }
        }
        if self.days != 0 { write!(f, "{}D", self.days)?; }
        if self.nanoseconds != 0 {
            let ns    = self.nanoseconds.abs();
            let sign  = if self.nanoseconds < 0 { "-" } else { "" };
            let h     = ns / 3_600_000_000_000_i64;
            let min   = (ns % 3_600_000_000_000_i64) / 60_000_000_000_i64;
            let s     = (ns % 60_000_000_000_i64) / 1_000_000_000_i64;
            let frac  = ns % 1_000_000_000_i64;
            f.write_str("T")?;
            if h    != 0 { write!(f, "{sign}{h}H")?; }
            if min  != 0 { write!(f, "{sign}{min}M")?; }
            if s != 0 || frac != 0 {
                if frac != 0 {
                    write!(f, "{sign}{s}.{}S", format!("{frac:09}").trim_end_matches('0'))?;
                } else {
                    write!(f, "{sign}{s}S")?;
                }
            }
        }
        Ok(())
    }
}

// ─── Network ─────────────────────────────────────────────────────────────────

/// An IP address literal.
///
/// Stored as a self-contained enum (not `std::net::IpAddr`) for consistent
/// size, alignment, and serde behaviour. `From` impls cover the stdlib types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IpAddr {
    V4([u8; 4]),
    V6([u8; 16]),
}

impl IpAddr {
    pub const fn v4(a: u8, b: u8, c: u8, d: u8) -> Self { Self::V4([a, b, c, d]) }
    pub const fn v6(bytes: [u8; 16]) -> Self { Self::V6(bytes) }
    pub const fn is_v4(&self) -> bool { matches!(self, Self::V4(_)) }
    pub const fn is_v6(&self) -> bool { matches!(self, Self::V6(_)) }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V4([a, b, c, d]) => write!(f, "{a}.{b}.{c}.{d}"),
            Self::V6(b) => {
                let w = |i: usize| u16::from_be_bytes([b[i * 2], b[i * 2 + 1]]);
                write!(f, "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                    w(0), w(1), w(2), w(3), w(4), w(5), w(6), w(7))
            }
        }
    }
}

impl From<[u8; 4]>  for IpAddr { fn from(b: [u8; 4])  -> Self { Self::V4(b) } }
impl From<[u8; 16]> for IpAddr { fn from(b: [u8; 16]) -> Self { Self::V6(b) } }
impl From<std::net::Ipv4Addr> for IpAddr {
    fn from(a: std::net::Ipv4Addr) -> Self { Self::V4(a.octets()) }
}
impl From<std::net::Ipv6Addr> for IpAddr {
    fn from(a: std::net::Ipv6Addr) -> Self { Self::V6(a.octets()) }
}
impl From<std::net::IpAddr> for IpAddr {
    fn from(a: std::net::IpAddr) -> Self {
        match a {
            std::net::IpAddr::V4(v) => v.into(),
            std::net::IpAddr::V6(v) => v.into(),
        }
    }
}

/// An Ethernet MAC address (EUI-48).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MacAddr(pub [u8; 6]);

impl MacAddr {
    pub const fn new(b: [u8; 6]) -> Self { Self(b) }
    pub const fn octets(self) -> [u8; 6] { self.0 }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g] = self.0;
        write!(f, "{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{g:02x}")
    }
}

/// An Ethernet MAC address (EUI-64 / MAC-48 extended).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MacAddr8(pub [u8; 8]);

impl MacAddr8 {
    pub const fn new(b: [u8; 8]) -> Self { Self(b) }
    pub const fn octets(self) -> [u8; 8] { self.0 }
}

impl fmt::Display for MacAddr8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g, h, i] = self.0;
        write!(f, "{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{g:02x}:{h:02x}:{i:02x}")
    }
}

// ─── BitString ───────────────────────────────────────────────────────────────

/// A packed bit string with an explicit bit count.
///
/// Maps to SQL `BIT(n)` / `VARBIT(n)`, PostgreSQL `bit` / `varbit`, and
/// binary protocol flags. The byte buffer always satisfies
/// `bytes.len() == len.div_ceil(8)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BitString {
    /// Bit count.
    pub len: u32,
    /// Packed bytes; high bit of `bytes[0]` is bit 0.
    pub bytes: Box<[u8]>,
}

impl BitString {
    /// Validates that `bytes.len() == len.div_ceil(8)`.
    pub fn try_new(len: u32, bytes: Box<[u8]>) -> Result<Self, TypeError> {
        let required = len.div_ceil(8) as usize;
        if bytes.len() != required {
            return Err(TypeError::BitLengthMismatch { declared: len, byte_count: bytes.len() });
        }
        Ok(Self { len, bytes })
    }

    pub fn new_unchecked(len: u32, bytes: Box<[u8]>) -> Self {
        Self { len, bytes }
    }

    /// Creates a zero-filled bit string.
    pub fn zeroes(len: u32) -> Self {
        let byte_count = len.div_ceil(8) as usize;
        Self { len, bytes: vec![0u8; byte_count].into_boxed_slice() }
    }
}

impl fmt::Display for BitString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("b'")?;
        for i in 0..self.len {
            let byte = self.bytes[(i / 8) as usize];
            let bit  = (byte >> (7 - (i % 8))) & 1;
            write!(f, "{bit}")?;
        }
        f.write_str("'")
    }
}

// ─── Geometry ────────────────────────────────────────────────────────────────
//
// Two-dimensional geometric primitives for spatial data operations.
// All coordinate types use f64 and reject non-finite values via `try_new`.
//
// Maps to: PostgreSQL native geometry types, PostGIS geometry/geography,
// MySQL spatial types, and GIS API payloads.

/// A 2D point `(x, y)`.
///
/// # Size
///
/// 16 bytes — fits directly as a `Value` variant payload without boxing.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn try_new(x: f64, y: f64) -> Result<Self, TypeError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(TypeError::NonFiniteCoordinate);
        }
        Ok(Self { x, y })
    }

    pub const fn new_unchecked(x: f64, y: f64) -> Self { Self { x, y } }
}

impl fmt::Display for Point2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

/// An infinite 2D line defined by `ax + by + c = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Line2D {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Line2D {
    pub fn try_new(a: f64, b: f64, c: f64) -> Result<Self, TypeError> {
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            return Err(TypeError::NonFiniteCoordinate);
        }
        Ok(Self { a, b, c })
    }

    pub const fn new_unchecked(a: f64, b: f64, c: f64) -> Self { Self { a, b, c } }
}

impl fmt::Display for Line2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{},{},{}}}", self.a, self.b, self.c)
    }
}

/// A finite 2D line segment defined by two endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Segment2D {
    pub start: Point2D,
    pub end: Point2D,
}

impl Segment2D {
    pub const fn new(start: Point2D, end: Point2D) -> Self { Self { start, end } }
}

impl fmt::Display for Segment2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{},{}]", self.start, self.end)
    }
}

/// An axis-aligned 2D rectangle defined by two corners.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect2D {
    pub low: Point2D,
    pub high: Point2D,
}

impl Rect2D {
    pub const fn new(low: Point2D, high: Point2D) -> Self { Self { low, high } }
}

impl fmt::Display for Rect2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.low, self.high)
    }
}

/// A 2D circle with a center point and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Circle2D {
    pub center: Point2D,
    pub radius: f64,
}

impl Circle2D {
    pub fn try_new(center: Point2D, radius: f64) -> Result<Self, TypeError> {
        if !radius.is_finite() || radius < 0.0 {
            return Err(TypeError::NonFiniteCoordinate);
        }
        Ok(Self { center, radius })
    }

    pub const fn new_unchecked(center: Point2D, radius: f64) -> Self {
        Self { center, radius }
    }
}

impl fmt::Display for Circle2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{},{}>", self.center, self.radius)
    }
}

/// A 2D path: an ordered sequence of points, open or closed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Path2D {
    pub closed: bool,
    pub points: Box<[Point2D]>,
}

impl Path2D {
    pub fn new(closed: bool, points: Vec<Point2D>) -> Self {
        Self { closed, points: points.into_boxed_slice() }
    }
}

impl fmt::Display for Path2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (open, close) = if self.closed { ('(', ')') } else { ('[', ']') };
        write!(f, "{open}")?;
        for (i, p) in self.points.iter().enumerate() {
            if i > 0 { write!(f, ",")?; }
            write!(f, "{p}")?;
        }
        write!(f, "{close}")
    }
}

/// A 2D polygon: an implicitly-closed ordered sequence of points.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polygon2D {
    pub points: Box<[Point2D]>,
}

impl Polygon2D {
    pub fn new(points: Vec<Point2D>) -> Self {
        Self { points: points.into_boxed_slice() }
    }
}

impl fmt::Display for Polygon2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, p) in self.points.iter().enumerate() {
            if i > 0 { write!(f, ",")?; }
            write!(f, "{p}")?;
        }
        write!(f, ")")
    }
}

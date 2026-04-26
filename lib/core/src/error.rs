//! Errors produced by type construction, validation, and conformance checks.

use alloc::boxed::Box;
use core::fmt;

/// All errors that can arise from the DOL type layer.
///
/// Covers two surfaces:
///
/// - **Value construction** — validation failures in `try_new` constructors
///   (invalid field ranges, non-finite floats, etc.)
/// - **Type conformance** — failures from [`crate::DataType::accepts`] when a
///   runtime [`super::Value`] does not conform to its declared type descriptor
///
/// The enum is `#[non_exhaustive]` so that adding new validation rules in
/// future releases is not a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeError {
    // ── Value construction ────────────────────────────────────────────────
    /// Month must be `1–12`.
    InvalidMonth(u8),
    /// Day must be `1–31`.
    InvalidDay(u8),
    /// Hour must be `0–23`.
    InvalidHour(u8),
    /// Minute must be `0–59`.
    InvalidMinute(u8),
    /// Second must be `0–60` (60 is reserved for leap seconds).
    InvalidSecond(u8),
    /// Nanosecond must be `0–999_999_999`.
    InvalidNanosecond(u32),
    /// Timezone offset must be in the range `−86_399..=86_399` seconds.
    TimezoneOffsetOutOfRange(i32),
    /// `Decimal` scale exceeded [`super::Decimal::MAX_SCALE`].
    DecimalScaleTooLarge { scale: u32 },
    /// A `f32` or `f64` value was `NaN` or infinite.
    NonFiniteFloat,
    /// The bit-string byte buffer does not hold the declared number of bits.
    ///
    /// `ceil(declared / 8)` bytes are required.
    BitLengthMismatch { declared: u32, byte_count: usize },
    /// A geometric coordinate was non-finite.
    NonFiniteCoordinate,

    // ── Type conformance ──────────────────────────────────────────────────
    /// A `null` value was provided for a non-nullable field.
    NullNotAllowed,

    /// The value's kind does not match the expected type's kind.
    KindMismatch {
        expected: &'static str,
        got: &'static str,
    },

    /// A `String` or `Json` value exceeded the maximum declared length (characters).
    StringTooLong { max: u32, got: usize },

    /// A `Bytes` value exceeded the maximum declared length (bytes).
    BytesTooLong { max: u32, got: usize },

    /// A `BitString` value exceeded the maximum declared length (bits).
    BitsTooLong { max: u32, got: u32 },

    /// A `BitString` value did not match the exact declared length (bits).
    BitsLengthMismatch { expected: u32, got: u32 },

    /// A `Decimal` value had more significant digits than `precision` allows.
    PrecisionExceeded { max: u8, got: u8 },

    /// A `Decimal` value had a larger scale than declared.
    ScaleExceeded { max: u8, got: u8 },

    /// A `Tuple` value had a different number of elements than the declared type.
    TupleLengthMismatch { expected: usize, got: usize },

    /// An element of an `Array`, `Set`, or `Tuple` failed validation.
    ElementInvalid {
        index: usize,
        source: Box<TypeError>,
    },

    /// A value for a `Map` key failed validation.
    MapValueInvalid {
        key: Box<str>,
        source: Box<TypeError>,
    },

    /// A required `Struct` field was absent from the value.
    StructFieldMissing(Box<str>),

    /// A `Struct` field's value failed validation against its declared type.
    StructFieldInvalid {
        field: Box<str>,
        source: Box<TypeError>,
    },

    /// A `Struct` value contained a field not declared in the type.
    StructUnexpectedField(Box<str>),

    /// An `Enum` value's variant name is not in the type's declared variant list.
    EnumVariantUnknown { variant: Box<str> },

    /// The bounds of a `Range` value have incompatible kinds.
    RangeBoundsMismatch,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Construction
            Self::InvalidMonth(m) => write!(f, "invalid month {m}: must be 1–12"),
            Self::InvalidDay(d) => write!(f, "invalid day {d}: must be 1–31"),
            Self::InvalidHour(h) => write!(f, "invalid hour {h}: must be 0–23"),
            Self::InvalidMinute(m) => write!(f, "invalid minute {m}: must be 0–59"),
            Self::InvalidSecond(s) => write!(f, "invalid second {s}: must be 0–60"),
            Self::InvalidNanosecond(n) => {
                write!(f, "invalid nanosecond {n}: must be 0–999_999_999")
            }
            Self::TimezoneOffsetOutOfRange(o) => {
                write!(f, "timezone offset {o}s out of ±86_399 range")
            }
            Self::DecimalScaleTooLarge { scale } => write!(
                f,
                "decimal scale {scale} exceeds maximum {}",
                super::Decimal::MAX_SCALE
            ),
            Self::NonFiniteFloat => write!(f, "float value must be finite (not NaN or infinite)"),
            Self::BitLengthMismatch {
                declared,
                byte_count,
            } => write!(
                f,
                "bit-string declared {declared} bits but buffer has {byte_count} bytes (need {})",
                declared.div_ceil(8)
            ),
            Self::NonFiniteCoordinate => write!(f, "geometric coordinate must be finite"),

            // Conformance
            Self::NullNotAllowed => write!(f, "null value is not allowed for a non-nullable type"),
            Self::KindMismatch { expected, got } => {
                write!(f, "type mismatch: expected {expected}, got {got}")
            }
            Self::StringTooLong { max, got } => {
                write!(f, "string length {got} exceeds maximum {max}")
            }
            Self::BytesTooLong { max, got } => write!(f, "byte length {got} exceeds maximum {max}"),
            Self::BitsTooLong { max, got } => {
                write!(f, "bit-string length {got} exceeds maximum {max}")
            }
            Self::BitsLengthMismatch { expected, got } => write!(
                f,
                "bit-string length {got} does not match declared {expected}"
            ),
            Self::PrecisionExceeded { max, got } => write!(
                f,
                "decimal has {got} significant digits but type allows {max}"
            ),
            Self::ScaleExceeded { max, got } => {
                write!(f, "decimal scale {got} exceeds declared {max}")
            }
            Self::TupleLengthMismatch { expected, got } => {
                write!(f, "tuple has {got} elements but type expects {expected}")
            }
            Self::ElementInvalid { index, source } => {
                write!(f, "element at index {index} is invalid: {source}")
            }
            Self::MapValueInvalid { key, source } => {
                write!(f, "map value for key \"{key}\" is invalid: {source}")
            }
            Self::StructFieldMissing(field) => {
                write!(f, "required struct field \"{field}\" is missing")
            }
            Self::StructFieldInvalid { field, source } => {
                write!(f, "struct field \"{field}\" is invalid: {source}")
            }
            Self::StructUnexpectedField(field) => {
                write!(f, "struct contains undeclared field \"{field}\"")
            }
            Self::EnumVariantUnknown { variant } => {
                write!(f, "enum variant \"{variant}\" is not declared in the type")
            }
            Self::RangeBoundsMismatch => write!(f, "range bounds have incompatible kinds"),
        }
    }
}

impl core::error::Error for TypeError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ElementInvalid { source, .. }
            | Self::MapValueInvalid { source, .. }
            | Self::StructFieldInvalid { source, .. } => Some(source),
            _ => None,
        }
    }
}

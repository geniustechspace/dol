//! Type descriptor: the [`DataType`] enum.

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::StructField;

/// A type descriptor for any value that flows through DOL.
///
/// # Composite types
///
/// Composite variants are heap-allocated (`Box<DataType>`, `Vec<StructField>`)
/// to keep `DataType` a reasonable size on the stack.
///
/// # Type references
///
/// [`DataType::TypeRef`] is a symbolic reference to a type defined elsewhere
/// in the schema (e.g., a user-defined SQL type, an Avro named type, a Protobuf
/// message). Resolution happens at the schema layer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum DataType {
    // ── Primitive ──
    Null,
    Bool,

    // ── Integer ──
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,

    // ── Float ──
    Float32,
    Float64,

    // ── Decimal ──
    /// Fixed-point numeric.
    ///
    /// - `precision`: total significant digits (None = unlimited).
    /// - `scale`: fractional digits (None = unlimited, or derived from precision).
    #[cfg(feature = "numeric")]
    Decimal {
        precision: Option<u8>,
        scale: Option<u8>,
    },

    // ── Text ──
    /// Character string with optional bounded length and fixed-vs-variable
    /// distinction. `max_len = None` means unbounded; `fixed = true` requires
    /// values to match `max_len` exactly (padding/validation is a backend
    /// concern).
    String {
        max_len: Option<u32>,
        fixed: bool,
    },
    /// JSON-typed text. Distinct from [`String`](Self::String) so backends
    /// apply the correct wire encoding.
    Json,
    /// XML-typed text.
    Xml,

    // ── Binary ──
    /// Byte string with optional bounded length and fixed-vs-variable
    /// distinction. `max_len = None` means unbounded; `fixed = true` requires
    /// values to match `max_len` exactly.
    Bytes {
        max_len: Option<u32>,
        fixed: bool,
    },
    /// UUID in 16-byte canonical form.
    Uuid,

    // ── Bit string ──
    /// Bit string with optional bounded length and fixed-vs-variable
    /// distinction. `max_len = None` means unbounded; `fixed = true` requires
    /// values to match `max_len` exactly.
    BitString {
        max_len: Option<u32>,
        fixed: bool,
    },

    // ── Temporal ──
    #[cfg(feature = "datetime")]
    Date,
    /// Time of day. `precision` is fractional seconds digits (`0–9`).
    #[cfg(feature = "datetime")]
    Time {
        precision: u8,
    },
    /// Naive datetime. `precision` is fractional seconds digits (`0–9`).
    #[cfg(feature = "datetime")]
    DateTime {
        precision: u8,
    },
    /// Datetime with timezone offset. `precision` is fractional seconds digits.
    #[cfg(feature = "datetime")]
    OffsetDateTime {
        precision: u8,
    },
    #[cfg(feature = "datetime")]
    Interval,

    // ── Network ──
    /// An IP host address (IPv4 or IPv6).
    #[cfg(feature = "network")]
    IpAddr,
    /// An IP network range with an explicit prefix length. Values are
    /// `IpAddr` (the host portion); the network mask is part of the *type*,
    /// not the value.
    #[cfg(feature = "network")]
    IpNetwork {
        prefix_len: u8,
    },
    /// An Ethernet MAC address. Matches both EUI-48 and EUI-64
    /// [`crate::Value::MacAddr`] payloads; [`Self::type_name`] returns
    /// `"macaddr"` for either width.
    #[cfg(feature = "network")]
    MacAddr,

    // ── Geometric ──
    #[cfg(feature = "geo")]
    Point,
    #[cfg(feature = "geo")]
    Line,
    #[cfg(feature = "geo")]
    LineSegment,
    #[cfg(feature = "geo")]
    Rect,
    #[cfg(feature = "geo")]
    Circle,
    #[cfg(feature = "geo")]
    Path,
    #[cfg(feature = "geo")]
    Polygon,

    // ── Composite ──
    /// Ordered, homogeneously typed sequence.
    Array(Box<DataType>),
    /// Unordered, homogeneously typed collection of unique values.
    Set(Box<DataType>),
    /// Key–value store: dynamic string keys, homogeneously typed values.
    Map {
        value: Box<DataType>,
    },
    /// Bounded range of an ordered type.
    Range(Box<DataType>),
    /// Fixed-length, heterogeneously typed sequence.
    Tuple(Vec<DataType>),
    /// Named, schema-typed fields. Order is significant.
    Struct(Vec<StructField>),
    /// An enumerated type with a fixed set of variant names.
    Enum(Vec<Box<str>>),

    // ── Meta ──
    /// A symbolic reference to a named type defined elsewhere (user-defined
    /// types, Avro named types, Protobuf message types, etc.).
    TypeRef(Box<str>),
    /// A domain-specific extension type not covered by the well-known variants.
    ///
    /// `name` identifies the type (e.g. `"vector"`, `"tsquery"`),
    /// `params` carries type parameters (e.g. element type, dimension).
    Extension {
        name: Box<str>,
        params: Vec<DataType>,
    },
}

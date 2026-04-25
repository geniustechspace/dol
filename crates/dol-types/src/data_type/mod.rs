//! Type descriptors: what shape and constraints a field or parameter holds.
//!
//! [`DataType`] is the type-descriptor counterpart to [`super::Value`]. It
//! answers **"what type is expected here?"** where `Value` answers
//! **"what data is actually here?"**.
//!
//! # Design
//!
//! - [`DataType`] mirrors every [`super::Value`] variant but adds constraints
//!   (e.g., `String { max_len, fixed }` adds a max-length and a fixed/variable
//!   flag).
//! - [`StructField`] is a named, typed field used inside `DataType::Struct`.
//! - [`DataType::accepts`] is the conformance bridge: it validates that a
//!   runtime [`super::Value`] satisfies the declared type.
//!
//! # Nullability
//!
//! Nullability is NOT embedded inside `DataType` (e.g., no
//! `DataType::Nullable` variant). It is a property of the *position* — the
//! field — not of the type itself. Embedding nullability would make
//! `DataType::Nullable(DataType::Nullable(...))` representable but meaningless.
//! Nullability belongs on the field definition (e.g., `StructField::nullable`).
//!
//! # No `Display`
//!
//! `DataType` deliberately does **not** implement [`core::fmt::Display`].
//! Backend-specific spellings (e.g. `VARCHAR(255)`, `String`, `text`) live
//! in the corresponding backend crate. For human-readable diagnostics, use
//! the auto-derived [`Debug`] impl or [`DataType::type_name`].

use super::{TypeError, Value};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ops::Bound;

// ─── StructField ─────────────────────────────────────────────────────────────

/// A named, typed field inside a [`DataType::Struct`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StructField {
    pub name: Box<str>,
    pub data_type: DataType,
    pub nullable: bool,
}

impl StructField {
    pub fn new(name: impl Into<Box<str>>, data_type: DataType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
        }
    }
}

// ─── DataType ────────────────────────────────────────────────────────────────

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    Date,
    /// Time of day. `precision` is fractional seconds digits (`0–9`).
    Time {
        precision: u8,
    },
    /// Naive datetime. `precision` is fractional seconds digits (`0–9`).
    DateTime {
        precision: u8,
    },
    /// Datetime with timezone offset. `precision` is fractional seconds digits.
    OffsetDateTime {
        precision: u8,
    },
    Interval,

    // ── Network ──
    /// An IP host address (IPv4 or IPv6).
    IpAddr,
    /// An IP network range with an explicit prefix length. Values are
    /// `IpAddr` (the host portion); the network mask is part of the *type*,
    /// not the value.
    IpNetwork {
        prefix_len: u8,
    },
    MacAddr,
    MacAddr8,

    // ── Geometric ──
    Point,
    Line,
    LineSegment,
    Rect,
    Circle,
    Path,
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

impl DataType {
    // ── Classification ────────────────────────────────────────────────────

    pub const fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::Int128
                | Self::UInt8
                | Self::UInt16
                | Self::UInt32
                | Self::UInt64
                | Self::UInt128
        )
    }

    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float32 | Self::Float64)
    }

    pub const fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float() || matches!(self, Self::Decimal { .. })
    }

    pub const fn is_textual(&self) -> bool {
        matches!(self, Self::String { .. } | Self::Json | Self::Xml)
    }

    pub const fn is_binary(&self) -> bool {
        matches!(
            self,
            Self::Bytes { .. } | Self::Uuid | Self::BitString { .. }
        )
    }

    pub const fn is_datetime(&self) -> bool {
        matches!(
            self,
            Self::Date
                | Self::Time { .. }
                | Self::DateTime { .. }
                | Self::OffsetDateTime { .. }
                | Self::Interval
        )
    }

    pub const fn is_geometric(&self) -> bool {
        matches!(
            self,
            Self::Point
                | Self::Line
                | Self::LineSegment
                | Self::Rect
                | Self::Circle
                | Self::Path
                | Self::Polygon
        )
    }

    pub const fn is_network(&self) -> bool {
        matches!(
            self,
            Self::IpAddr | Self::IpNetwork { .. } | Self::MacAddr | Self::MacAddr8
        )
    }

    pub const fn is_composite(&self) -> bool {
        matches!(
            self,
            Self::Array(_)
                | Self::Set(_)
                | Self::Map { .. }
                | Self::Range(_)
                | Self::Tuple(_)
                | Self::Struct(_)
                | Self::Enum(_)
        )
    }

    // ── Conformance ───────────────────────────────────────────────────────

    /// Returns `Ok(())` if `value` is a valid instance of this type, or a
    /// detailed [`TypeError`] describing the first violation.
    ///
    /// This does **not** handle nullability; the caller (e.g., a field
    /// definition) should check for null separately.
    pub fn accepts(&self, value: &Value) -> Result<(), TypeError> {
        use Value as V;

        macro_rules! kind_check {
            ($pat:pat) => {{
                if !matches!(value, $pat) {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                }
                Ok(())
            }};
        }

        match self {
            // Null accepts only Null — nullability is checked at the field level.
            Self::Null => kind_check!(V::Null),

            // Primitive pass-through
            Self::Bool => kind_check!(V::Bool(_)),
            Self::Int8 => kind_check!(V::Int8(_)),
            Self::Int16 => kind_check!(V::Int16(_)),
            Self::Int32 => kind_check!(V::Int32(_)),
            Self::Int64 => kind_check!(V::Int64(_)),
            Self::Int128 => kind_check!(V::Int128(_)),
            Self::UInt8 => kind_check!(V::UInt8(_)),
            Self::UInt16 => kind_check!(V::UInt16(_)),
            Self::UInt32 => kind_check!(V::UInt32(_)),
            Self::UInt64 => kind_check!(V::UInt64(_)),
            Self::UInt128 => kind_check!(V::UInt128(_)),
            Self::Float32 => kind_check!(V::Float32(_)),
            Self::Float64 => kind_check!(V::Float64(_)),

            // Decimal with optional precision/scale constraints
            Self::Decimal { precision, scale } => {
                let V::Decimal(d) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(max_scale) = scale {
                    if d.scale > *max_scale as u32 {
                        return Err(TypeError::ScaleExceeded {
                            max: *max_scale,
                            got: d.scale as u8,
                        });
                    }
                }
                if let Some(max_prec) = precision {
                    let digits = d.unscaled.unsigned_abs().to_string().len() as u8;
                    if digits > *max_prec {
                        return Err(TypeError::PrecisionExceeded {
                            max: *max_prec,
                            got: digits,
                        });
                    }
                }
                Ok(())
            }

            // Text types
            Self::String { max_len, fixed } => {
                let V::String(s) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(limit) = max_len {
                    let actual = s.chars().count();
                    if *fixed {
                        if actual > *limit as usize {
                            return Err(TypeError::StringTooLong {
                                max: *limit,
                                got: actual,
                            });
                        }
                    } else if actual > *limit as usize {
                        return Err(TypeError::StringTooLong {
                            max: *limit,
                            got: actual,
                        });
                    }
                }
                Ok(())
            }
            Self::Json => kind_check!(V::Json(_)),
            Self::Xml => kind_check!(V::Xml(_)),

            // Binary types
            Self::Bytes { max_len, fixed } => {
                let V::Bytes(b) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(limit) = max_len {
                    if *fixed {
                        if b.len() != *limit as usize {
                            return Err(TypeError::BytesTooLong {
                                max: *limit,
                                got: b.len(),
                            });
                        }
                    } else if b.len() > *limit as usize {
                        return Err(TypeError::BytesTooLong {
                            max: *limit,
                            got: b.len(),
                        });
                    }
                }
                Ok(())
            }
            Self::Uuid => kind_check!(V::Uuid(_)),

            // Bit strings
            Self::BitString { max_len, fixed } => {
                let V::BitString(bs) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(limit) = max_len {
                    if *fixed {
                        if bs.len != *limit {
                            return Err(TypeError::BitsLengthMismatch {
                                expected: *limit,
                                got: bs.len,
                            });
                        }
                    } else if bs.len > *limit {
                        return Err(TypeError::BitsTooLong {
                            max: *limit,
                            got: bs.len,
                        });
                    }
                }
                Ok(())
            }

            // Temporal
            Self::Date => kind_check!(V::Date(_)),
            Self::Time { .. } => kind_check!(V::Time(_)),
            Self::DateTime { .. } => kind_check!(V::DateTime(_)),
            Self::OffsetDateTime { .. } => kind_check!(V::TimestampTz(_)),
            Self::Interval => kind_check!(V::Interval(_)),

            // Network
            Self::IpAddr | Self::IpNetwork { .. } => {
                if !matches!(value, V::Inet(_)) {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                }
                Ok(())
            }
            Self::MacAddr => kind_check!(V::MacAddr(_)),
            Self::MacAddr8 => kind_check!(V::MacAddr8(_)),

            // Geometric
            Self::Point => kind_check!(V::Point(_)),
            Self::Line => kind_check!(V::Line(_)),
            Self::LineSegment => kind_check!(V::Segment(_)),
            Self::Rect => kind_check!(V::Rect(_)),
            Self::Circle => kind_check!(V::Circle(_)),
            Self::Path => kind_check!(V::Path(_)),
            Self::Polygon => kind_check!(V::Polygon(_)),

            // Composite: Array
            Self::Array(element_type) => {
                let V::Array(items) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                for (i, item) in items.iter().enumerate() {
                    element_type
                        .accepts(item)
                        .map_err(|e| TypeError::ElementInvalid {
                            index: i,
                            source: Box::new(e),
                        })?;
                }
                Ok(())
            }

            // Composite: Set
            Self::Set(element_type) => {
                let V::Set(items) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                for (i, item) in items.iter().enumerate() {
                    element_type
                        .accepts(item)
                        .map_err(|e| TypeError::ElementInvalid {
                            index: i,
                            source: Box::new(e),
                        })?;
                }
                Ok(())
            }

            // Composite: Map
            Self::Map { value: val_type } => {
                let V::Map(entries) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                for (k, v) in entries.iter() {
                    val_type
                        .accepts(v)
                        .map_err(|e| TypeError::MapValueInvalid {
                            key: k.clone(),
                            source: Box::new(e),
                        })?;
                }
                Ok(())
            }

            // Composite: Range
            Self::Range(element_type) => {
                let V::Range(r) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                let check_bound = |b: &Bound<Box<Value>>| -> Result<(), TypeError> {
                    match b {
                        Bound::Included(v) | Bound::Excluded(v) => element_type.accepts(v),
                        Bound::Unbounded => Ok(()),
                    }
                };
                check_bound(&r.start)?;
                check_bound(&r.end)?;
                Ok(())
            }

            // Composite: Tuple
            Self::Tuple(field_types) => {
                let V::Tuple(items) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if items.len() != field_types.len() {
                    return Err(TypeError::TupleLengthMismatch {
                        expected: field_types.len(),
                        got: items.len(),
                    });
                }
                for (i, (ft, v)) in field_types.iter().zip(items.iter()).enumerate() {
                    ft.accepts(v).map_err(|e| TypeError::ElementInvalid {
                        index: i,
                        source: Box::new(e),
                    })?;
                }
                Ok(())
            }

            // Composite: Struct
            Self::Struct(fields) => {
                let V::Struct(entries) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };

                // Check that every declared field is present and valid
                for field in fields {
                    match entries
                        .iter()
                        .find(|(k, _)| k.as_ref() == field.name.as_ref())
                    {
                        None => {
                            if !field.nullable {
                                return Err(TypeError::StructFieldMissing(field.name.clone()));
                            }
                        }
                        Some((_, v)) => {
                            if v.is_null() && !field.nullable {
                                return Err(TypeError::StructFieldInvalid {
                                    field: field.name.clone(),
                                    source: Box::new(TypeError::NullNotAllowed),
                                });
                            }
                            if !v.is_null() {
                                field.data_type.accepts(v).map_err(|e| {
                                    TypeError::StructFieldInvalid {
                                        field: field.name.clone(),
                                        source: Box::new(e),
                                    }
                                })?;
                            }
                        }
                    }
                }

                // Check that no undeclared fields are present (strict mode)
                for (k, _) in entries.iter() {
                    if !fields.iter().any(|f| f.name.as_ref() == k.as_ref()) {
                        return Err(TypeError::StructUnexpectedField(k.clone()));
                    }
                }

                Ok(())
            }

            // Composite: Enum
            Self::Enum(variants) => {
                let V::Enum(variant) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if !variants.iter().any(|v| v.as_ref() == variant.as_ref()) {
                    return Err(TypeError::EnumVariantUnknown {
                        variant: variant.clone(),
                    });
                }
                Ok(())
            }

            // TypeRef: cannot validate structurally; resolution is schema-layer concern
            Self::TypeRef(_) => Ok(()),

            // Extension: cannot validate structurally; semantics are domain-specific
            Self::Extension { .. } => Ok(()),
        }
    }

    // ── Display helpers ───────────────────────────────────────────────────

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Int8 => "int8",
            Self::Int16 => "int16",
            Self::Int32 => "int32",
            Self::Int64 => "int64",
            Self::Int128 => "int128",
            Self::UInt8 => "uint8",
            Self::UInt16 => "uint16",
            Self::UInt32 => "uint32",
            Self::UInt64 => "uint64",
            Self::UInt128 => "uint128",
            Self::Float32 => "float32",
            Self::Float64 => "float64",
            Self::Decimal { .. } => "decimal",
            Self::String { .. } => "string",
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Bytes { .. } => "bytes",
            Self::Uuid => "uuid",
            Self::BitString { .. } => "bitstring",
            Self::Date => "date",
            Self::Time { .. } => "time",
            Self::DateTime { .. } => "datetime",
            Self::OffsetDateTime { .. } => "offsetdatetime",
            Self::Interval => "interval",
            Self::IpAddr => "ipaddr",
            Self::IpNetwork { .. } => "ipnetwork",
            Self::MacAddr => "macaddr",
            Self::MacAddr8 => "macaddr8",
            Self::Point => "point",
            Self::Line => "line",
            Self::LineSegment => "lseg",
            Self::Rect => "rect",
            Self::Circle => "circle",
            Self::Path => "path",
            Self::Polygon => "polygon",
            Self::Array(_) => "array",
            Self::Set(_) => "set",
            Self::Map { .. } => "map",
            Self::Range(_) => "range",
            Self::Tuple(_) => "tuple",
            Self::Struct(_) => "struct",
            Self::Enum(_) => "enum",
            Self::TypeRef(_) => "typeref",
            Self::Extension { .. } => "extension",
        }
    }

    // ── Convenience constructors ──────────────────────────────────────────
    //
    // Store-neutral helpers that keep call sites concise without referencing
    // any specific backend's spelling.

    /// A bounded variable-length character string (`max_len` characters).
    pub const fn varying_string(max_len: u32) -> Self {
        Self::String {
            max_len: Some(max_len),
            fixed: false,
        }
    }

    /// A fixed-length character string (`len` characters).
    pub const fn fixed_string(len: u32) -> Self {
        Self::String {
            max_len: Some(len),
            fixed: true,
        }
    }

    /// An unbounded character string.
    pub const fn unbounded_string() -> Self {
        Self::String {
            max_len: None,
            fixed: false,
        }
    }

    /// A bounded variable-length byte string.
    pub const fn varying_bytes(max_len: u32) -> Self {
        Self::Bytes {
            max_len: Some(max_len),
            fixed: false,
        }
    }

    /// A fixed-length byte string.
    pub const fn fixed_bytes(len: u32) -> Self {
        Self::Bytes {
            max_len: Some(len),
            fixed: true,
        }
    }

    /// An unbounded byte string.
    pub const fn unbounded_bytes() -> Self {
        Self::Bytes {
            max_len: None,
            fixed: false,
        }
    }

    /// A bounded variable-length bit string.
    pub const fn varying_bits(max_len: u32) -> Self {
        Self::BitString {
            max_len: Some(max_len),
            fixed: false,
        }
    }

    /// A fixed-length bit string.
    pub const fn fixed_bits(len: u32) -> Self {
        Self::BitString {
            max_len: Some(len),
            fixed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use super::*;
    use crate::numeric::Decimal;

    #[test]
    fn varchar_accepts_string_within_length() {
        let dt = DataType::varying_string(10);
        assert!(dt.accepts(&Value::from("hello")).is_ok());
    }

    #[test]
    fn varchar_rejects_string_over_length() {
        let dt = DataType::varying_string(3);
        let err = dt.accepts(&Value::from("hello")).unwrap_err();
        assert!(matches!(err, TypeError::StringTooLong { max: 3, got: 5 }));
    }

    #[test]
    fn decimal_precision_enforced() {
        let dt = DataType::Decimal {
            precision: Some(4),
            scale: Some(2),
        };
        // 123.45 → unscaled = 12345, 5 significant digits > 4
        let v = Value::Decimal(Box::new(Decimal::new_unchecked(12345, 2)));
        let err = dt.accepts(&v).unwrap_err();
        assert!(matches!(err, TypeError::PrecisionExceeded { max: 4, .. }));
    }

    #[test]
    fn decimal_scale_enforced() {
        let dt = DataType::Decimal {
            precision: None,
            scale: Some(2),
        };
        // scale of 3 exceeds max 2
        let v = Value::Decimal(Box::new(Decimal::new_unchecked(12345, 3)));
        let err = dt.accepts(&v).unwrap_err();
        assert!(matches!(err, TypeError::ScaleExceeded { max: 2, .. }));
    }

    #[test]
    fn struct_missing_required_field() {
        let dt = DataType::Struct(vec![
            StructField::new("id", DataType::Int32, false),
            StructField::new("name", DataType::unbounded_string(), false),
        ]);
        // Value has only "id"
        let v = Value::Struct(vec![("id".into(), Value::Int32(1))].into_boxed_slice());
        let err = dt.accepts(&v).unwrap_err();
        assert!(matches!(err, TypeError::StructFieldMissing(ref f) if f.as_ref() == "name"));
    }

    #[test]
    fn struct_unexpected_field_rejected() {
        let dt = DataType::Struct(vec![StructField::new("id", DataType::Int32, false)]);
        let v = Value::Struct(
            vec![
                ("id".into(), Value::Int32(1)),
                ("unknown".into(), Value::Bool(true)),
            ]
            .into_boxed_slice(),
        );
        let err = dt.accepts(&v).unwrap_err();
        assert!(matches!(err, TypeError::StructUnexpectedField(ref f) if f.as_ref() == "unknown"));
    }

    #[test]
    fn enum_rejects_unknown_variant() {
        let dt = DataType::Enum(vec!["active".into(), "inactive".into()]);
        let v = Value::Enum("deleted".into());
        let err = dt.accepts(&v).unwrap_err();
        assert!(
            matches!(err, TypeError::EnumVariantUnknown { ref variant } if variant.as_ref() == "deleted")
        );
    }

    #[test]
    fn enum_accepts_known_variant() {
        let dt = DataType::Enum(vec!["active".into(), "inactive".into()]);
        assert!(dt.accepts(&Value::Enum("active".into())).is_ok());
    }

    #[test]
    fn array_validates_elements() {
        let dt = DataType::Array(Box::new(DataType::Int32));
        let v = Value::Array(vec![Value::Int32(1), Value::Int64(2)].into_boxed_slice());
        let err = dt.accepts(&v).unwrap_err();
        assert!(matches!(err, TypeError::ElementInvalid { index: 1, .. }));
    }

    #[test]
    fn tuple_length_mismatch() {
        let dt = DataType::Tuple(vec![DataType::Int32, DataType::unbounded_string()]);
        let v = Value::Tuple(vec![Value::Int32(1)].into_boxed_slice());
        assert!(matches!(
            dt.accepts(&v).unwrap_err(),
            TypeError::TupleLengthMismatch {
                expected: 2,
                got: 1
            }
        ));
    }
}

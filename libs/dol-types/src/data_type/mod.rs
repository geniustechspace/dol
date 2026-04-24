//! Type descriptors: what shape and constraints a field or parameter holds.
//!
//! [`DataType`] is the type-descriptor counterpart to [`super::Value`]. It
//! answers **"what type is expected here?"** where `Value` answers
//! **"what data is actually here?"**.
//!
//! # Design
//!
//! - [`DataType`] mirrors every [`super::Value`] variant but adds constraints
//!   (e.g., `Varchar(Option<u32>)` adds a max-length).
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
//! Submodules:
//! - [`display`] — `impl Display for DataType` (separate to keep this file focused).

mod display;

use super::{TypeError, Value};
use std::ops::Bound;

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
    /// Fixed-length character string: `CHAR(n)`.
    Char(u32),
    /// Variable-length character string: `VARCHAR(n)` or `TEXT` if `max` is `None`.
    Varchar(Option<u32>),
    /// Unbounded text. Alias for `Varchar(None)` with distinct backend semantics.
    Text,
    /// JSON-typed text. Distinct from `Varchar`/`Text` so backends apply the
    /// correct wire encoding.
    Json,
    /// XML-typed text.
    Xml,

    // ── Binary ──
    /// Fixed-length byte array: `BINARY(n)`.
    Binary(u32),
    /// Variable-length byte array: `VARBINARY(n)` or `BLOB` if `max` is `None`.
    Varbinary(Option<u32>),
    /// UUID in 16-byte canonical form.
    Uuid,

    // ── Bit string ──
    /// Fixed-length bit string: `BIT(n)`.
    Bit(u32),
    /// Variable-length bit string: `VARBIT(n)` or unlimited if `max` is `None`.
    Varbit(Option<u32>),

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
    TimestampTz {
        precision: u8,
    },
    Interval,

    // ── Network ──
    Inet,
    /// IP network in CIDR notation. Values are `IpAddr` (the host portion);
    /// the network mask is part of the *type*, not the value.
    Cidr {
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
        matches!(
            self,
            Self::Char(_) | Self::Varchar(_) | Self::Text | Self::Json | Self::Xml
        )
    }

    pub const fn is_binary(&self) -> bool {
        matches!(
            self,
            Self::Binary(_) | Self::Varbinary(_) | Self::Uuid | Self::Bit(_) | Self::Varbit(_)
        )
    }

    pub const fn is_datetime(&self) -> bool {
        matches!(
            self,
            Self::Date
                | Self::Time { .. }
                | Self::DateTime { .. }
                | Self::TimestampTz { .. }
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
            Self::Inet | Self::Cidr { .. } | Self::MacAddr | Self::MacAddr8
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
            Self::Char(len) => {
                let V::String(s) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                let actual = s.chars().count();
                if actual != *len as usize {
                    // CHAR requires exact length; backends pad, but we validate the max.
                    if actual > *len as usize {
                        return Err(TypeError::StringTooLong {
                            max: *len,
                            got: actual,
                        });
                    }
                }
                Ok(())
            }
            Self::Varchar(max) => {
                let V::String(s) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(max_len) = max {
                    let actual = s.chars().count();
                    if actual > *max_len as usize {
                        return Err(TypeError::StringTooLong {
                            max: *max_len,
                            got: actual,
                        });
                    }
                }
                Ok(())
            }
            Self::Text => kind_check!(V::String(_)),
            Self::Json => kind_check!(V::Json(_)),
            Self::Xml => kind_check!(V::Xml(_)),

            // Binary types
            Self::Binary(len) => {
                let V::Bytes(b) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if b.len() != *len as usize {
                    return Err(TypeError::BytesTooLong {
                        max: *len,
                        got: b.len(),
                    });
                }
                Ok(())
            }
            Self::Varbinary(max) => {
                let V::Bytes(b) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(max_len) = max {
                    if b.len() > *max_len as usize {
                        return Err(TypeError::BytesTooLong {
                            max: *max_len,
                            got: b.len(),
                        });
                    }
                }
                Ok(())
            }
            Self::Uuid => kind_check!(V::Uuid(_)),

            // Bit strings
            Self::Bit(len) => {
                let V::BitString(bs) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if bs.len != *len {
                    return Err(TypeError::BitsLengthMismatch {
                        expected: *len,
                        got: bs.len,
                    });
                }
                Ok(())
            }
            Self::Varbit(max) => {
                let V::BitString(bs) = value else {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                };
                if let Some(max_bits) = max {
                    if bs.len > *max_bits {
                        return Err(TypeError::BitsTooLong {
                            max: *max_bits,
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
            Self::TimestampTz { .. } => kind_check!(V::TimestampTz(_)),
            Self::Interval => kind_check!(V::Interval(_)),

            // Network
            Self::Inet | Self::Cidr { .. } => {
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
            Self::Char(_) => "char",
            Self::Varchar(_) => "varchar",
            Self::Text => "text",
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Binary(_) => "binary",
            Self::Varbinary(_) => "varbinary",
            Self::Uuid => "uuid",
            Self::Bit(_) => "bit",
            Self::Varbit(_) => "varbit",
            Self::Date => "date",
            Self::Time { .. } => "time",
            Self::DateTime { .. } => "datetime",
            Self::TimestampTz { .. } => "timestamptz",
            Self::Interval => "interval",
            Self::Inet => "inet",
            Self::Cidr { .. } => "cidr",
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
}


#[cfg(test)]
mod tests {
    use super::Value;
    use super::*;
    use crate::numeric::Decimal;

    #[test]
    fn varchar_accepts_string_within_length() {
        let dt = DataType::Varchar(Some(10));
        assert!(dt.accepts(&Value::from("hello")).is_ok());
    }

    #[test]
    fn varchar_rejects_string_over_length() {
        let dt = DataType::Varchar(Some(3));
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
            StructField::new("name", DataType::Varchar(None), false),
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
        let dt = DataType::Tuple(vec![DataType::Int32, DataType::Text]);
        let v = Value::Tuple(vec![Value::Int32(1)].into_boxed_slice());
        assert!(matches!(
            dt.accepts(&v).unwrap_err(),
            TypeError::TupleLengthMismatch {
                expected: 2,
                got: 1
            }
        ));
    }

    #[test]
    fn display_for_datatypes() {
        assert_eq!(DataType::Varchar(Some(255)).to_string(), "VARCHAR(255)");
        assert_eq!(DataType::Varchar(None).to_string(), "TEXT");
        assert_eq!(
            DataType::Array(Box::new(DataType::Int32)).to_string(),
            "INT32[]"
        );
        assert_eq!(
            DataType::Decimal {
                precision: Some(10),
                scale: Some(2)
            }
            .to_string(),
            "NUMERIC(10,2)"
        );
    }
}

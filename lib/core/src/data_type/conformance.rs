//! [`DataType::accepts`] — the conformance bridge between [`DataType`] and
//! [`Value`].

use alloc::boxed::Box;
#[cfg(feature = "numeric")]
use alloc::string::ToString;
use core::ops::Bound;

use super::DataType;
use crate::error::TypeError;
use crate::value::Value;

impl DataType {
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
            #[cfg(feature = "numeric")]
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
            #[cfg(feature = "datetime")]
            Self::Date => kind_check!(V::Date(_)),
            #[cfg(feature = "datetime")]
            Self::Time { .. } => kind_check!(V::Time(_)),
            #[cfg(feature = "datetime")]
            Self::DateTime { .. } => kind_check!(V::DateTime(_)),
            #[cfg(feature = "datetime")]
            Self::OffsetDateTime { .. } => kind_check!(V::TimestampTz(_)),
            #[cfg(feature = "datetime")]
            Self::Interval => kind_check!(V::Interval(_)),

            // Network
            #[cfg(feature = "network")]
            Self::IpAddr | Self::IpNetwork { .. } => {
                if !matches!(value, V::Inet(_)) {
                    return Err(TypeError::KindMismatch {
                        expected: self.type_name(),
                        got: value.type_name(),
                    });
                }
                Ok(())
            }
            #[cfg(feature = "network")]
            Self::MacAddr => kind_check!(V::MacAddr(_)),

            // Geometric
            #[cfg(feature = "geo")]
            Self::Point => kind_check!(V::Point(_)),
            #[cfg(feature = "geo")]
            Self::Line => kind_check!(V::Line(_)),
            #[cfg(feature = "geo")]
            Self::LineSegment => kind_check!(V::Segment(_)),
            #[cfg(feature = "geo")]
            Self::Rect => kind_check!(V::Rect(_)),
            #[cfg(feature = "geo")]
            Self::Circle => kind_check!(V::Circle(_)),
            #[cfg(feature = "geo")]
            Self::Path => kind_check!(V::Path(_)),
            #[cfg(feature = "geo")]
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
}

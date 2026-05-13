//! `impl fmt::Display for Value`.

use core::fmt;

use super::Value;
use crate::format::fmt_uuid;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "\"{v}\""),
            Self::Json(v) => write!(f, "json({v})"),
            Self::Xml(v) => write!(f, "xml({v})"),
            Self::Enum(v) => write!(f, "'{v}'"),
            Self::Bytes(v) => write!(f, "bytes(len={})", v.len()),
            Self::Uuid(v) => fmt_uuid(v, f),
            Self::BitString(v) => write!(f, "{v}"),
            Self::Int8(v) => write!(f, "{v}i8"),
            Self::Int16(v) => write!(f, "{v}i16"),
            Self::Int32(v) => write!(f, "{v}i32"),
            Self::Int64(v) => write!(f, "{v}i64"),
            Self::Int128(v) => write!(f, "{v}i128"),
            Self::UInt8(v) => write!(f, "{v}u8"),
            Self::UInt16(v) => write!(f, "{v}u16"),
            Self::UInt32(v) => write!(f, "{v}u32"),
            Self::UInt64(v) => write!(f, "{v}u64"),
            Self::UInt128(v) => write!(f, "{v}u128"),
            Self::Float32(v) => write!(f, "{v}f32"),
            Self::Float64(v) => write!(f, "{v}f64"),
            #[cfg(feature = "numeric")]
            Self::Decimal(v) => write!(f, "{v}"),
            #[cfg(feature = "network")]
            Self::Inet(v) => write!(f, "{v}"),
            #[cfg(feature = "network")]
            Self::MacAddr(v) => write!(f, "{v}"),
            #[cfg(feature = "datetime")]
            Self::Date(v) => write!(f, "{v}"),
            #[cfg(feature = "datetime")]
            Self::Time(v) => write!(f, "{v}"),
            #[cfg(feature = "datetime")]
            Self::DateTime(v) => write!(f, "{v}"),
            #[cfg(feature = "datetime")]
            Self::TimestampTz(v) => write!(f, "{v}"),
            #[cfg(feature = "datetime")]
            Self::Interval(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Point(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Line(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Segment(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Rect(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Circle(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Path(v) => write!(f, "{v}"),
            #[cfg(feature = "geo")]
            Self::Polygon(v) => write!(f, "{v}"),
            Self::Array(vs) | Self::Set(vs) | Self::Tuple(vs) => {
                let tag = match self {
                    Self::Set(_) => "set",
                    Self::Tuple(_) => "tuple",
                    _ => "",
                };
                if !tag.is_empty() {
                    write!(f, "{tag}")?;
                }
                f.write_str("[")?;
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Self::Map(entries) | Self::Struct(entries) => {
                let tag = match self {
                    Self::Struct(_) => "struct",
                    _ => "",
                };
                if !tag.is_empty() {
                    write!(f, "{tag}")?;
                }
                f.write_str("{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "\"{k}\": {v}")?;
                }
                f.write_str("}")
            }
            Self::Range(r) => write!(f, "{r}"),
            Self::Extension(inner) => {
                write!(f, "ext:{}(len={})", inner.0, inner.1.len())
            }
        }
    }
}

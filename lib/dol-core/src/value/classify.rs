//! Classification predicates and human-readable type names for [`Value`].

use super::Value;

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int8(_)
                | Self::Int16(_)
                | Self::Int32(_)
                | Self::Int64(_)
                | Self::Int128(_)
                | Self::UInt8(_)
                | Self::UInt16(_)
                | Self::UInt32(_)
                | Self::UInt64(_)
                | Self::UInt128(_)
        )
    }
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float32(_) | Self::Float64(_))
    }
    pub fn is_numeric(&self) -> bool {
        if self.is_integer() || self.is_float() {
            return true;
        }
        #[cfg(feature = "numeric")]
        {
            matches!(self, Self::Decimal(_))
        }
        #[cfg(not(feature = "numeric"))]
        {
            false
        }
    }
    #[cfg(feature = "datetime")]
    pub fn is_datetime(&self) -> bool {
        matches!(
            self,
            Self::Date(_)
                | Self::Time(_)
                | Self::DateTime(_)
                | Self::TimestampTz(_)
                | Self::Interval(_)
        )
    }
    #[cfg(not(feature = "datetime"))]
    pub fn is_datetime(&self) -> bool {
        false
    }
    pub fn is_textual(&self) -> bool {
        matches!(
            self,
            Self::String(_) | Self::Json(_) | Self::Xml(_) | Self::Enum(_)
        )
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::String(_) => "string",
            Self::Json(_) => "json",
            Self::Xml(_) => "xml",
            Self::Enum(_) => "enum",
            Self::Bytes(_) => "bytes",
            Self::Uuid(_) => "uuid",
            Self::BitString(_) => "bitstring",
            Self::Int8(_) => "int8",
            Self::Int16(_) => "int16",
            Self::Int32(_) => "int32",
            Self::Int64(_) => "int64",
            Self::Int128(_) => "int128",
            Self::UInt8(_) => "uint8",
            Self::UInt16(_) => "uint16",
            Self::UInt32(_) => "uint32",
            Self::UInt64(_) => "uint64",
            Self::UInt128(_) => "uint128",
            Self::Float32(_) => "float32",
            Self::Float64(_) => "float64",
            #[cfg(feature = "numeric")]
            Self::Decimal(_) => "decimal",
            #[cfg(feature = "network")]
            Self::Inet(_) => "inet",
            #[cfg(feature = "network")]
            Self::MacAddr(_) => "macaddr",
            #[cfg(feature = "datetime")]
            Self::Date(_) => "date",
            #[cfg(feature = "datetime")]
            Self::Time(_) => "time",
            #[cfg(feature = "datetime")]
            Self::DateTime(_) => "datetime",
            #[cfg(feature = "datetime")]
            Self::TimestampTz(_) => "timestamptz",
            #[cfg(feature = "datetime")]
            Self::Interval(_) => "interval",
            #[cfg(feature = "geo")]
            Self::Point(_) => "point",
            #[cfg(feature = "geo")]
            Self::Line(_) => "line",
            #[cfg(feature = "geo")]
            Self::Segment(_) => "lseg",
            #[cfg(feature = "geo")]
            Self::Rect(_) => "rect",
            #[cfg(feature = "geo")]
            Self::Circle(_) => "circle",
            #[cfg(feature = "geo")]
            Self::Path(_) => "path",
            #[cfg(feature = "geo")]
            Self::Polygon(_) => "polygon",
            Self::Array(_) => "array",
            Self::Set(_) => "set",
            Self::Tuple(_) => "tuple",
            Self::Map(_) => "map",
            Self::Struct(_) => "struct",
            Self::Range(_) => "range",
            Self::Extension(_) => "extension",
        }
    }
}

//! Classification predicates and human-readable type names for [`DataType`].

use super::DataType;

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
        if self.is_integer() || self.is_float() {
            return true;
        }
        #[cfg(feature = "numeric")]
        {
            matches!(self, Self::Decimal { .. })
        }
        #[cfg(not(feature = "numeric"))]
        {
            false
        }
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

    #[cfg(feature = "datetime")]
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

    #[cfg(not(feature = "datetime"))]
    pub const fn is_datetime(&self) -> bool {
        false
    }

    #[cfg(feature = "geo")]
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

    #[cfg(not(feature = "geo"))]
    pub const fn is_geometric(&self) -> bool {
        false
    }

    #[cfg(feature = "network")]
    pub const fn is_network(&self) -> bool {
        matches!(self, Self::IpAddr | Self::IpNetwork { .. } | Self::MacAddr)
    }

    #[cfg(not(feature = "network"))]
    pub const fn is_network(&self) -> bool {
        false
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
            #[cfg(feature = "numeric")]
            Self::Decimal { .. } => "decimal",
            Self::String { .. } => "string",
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Bytes { .. } => "bytes",
            Self::Uuid => "uuid",
            Self::BitString { .. } => "bitstring",
            #[cfg(feature = "datetime")]
            Self::Date => "date",
            #[cfg(feature = "datetime")]
            Self::Time { .. } => "time",
            #[cfg(feature = "datetime")]
            Self::DateTime { .. } => "datetime",
            #[cfg(feature = "datetime")]
            Self::OffsetDateTime { .. } => "offsetdatetime",
            #[cfg(feature = "datetime")]
            Self::Interval => "interval",
            #[cfg(feature = "network")]
            Self::IpAddr => "ipaddr",
            #[cfg(feature = "network")]
            Self::IpNetwork { .. } => "ipnetwork",
            #[cfg(feature = "network")]
            Self::MacAddr => "macaddr",
            #[cfg(feature = "geo")]
            Self::Point => "point",
            #[cfg(feature = "geo")]
            Self::Line => "line",
            #[cfg(feature = "geo")]
            Self::LineSegment => "lseg",
            #[cfg(feature = "geo")]
            Self::Rect => "rect",
            #[cfg(feature = "geo")]
            Self::Circle => "circle",
            #[cfg(feature = "geo")]
            Self::Path => "path",
            #[cfg(feature = "geo")]
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

//! Field type — backend-agnostic logical types.
//!
//! DOL uses neutral names that map to backend-specific physical types:
//! - `Text` → SQL `TEXT`, Document `string`, KV `string`
//! - `Char(n)` → SQL `CHAR(n)`, fixed-length string
//! - `Varchar(Some(n))` → SQL `VARCHAR(n)`, variable-length with max
//! - `Varchar(None)` → SQL `VARCHAR`, unbounded variable-length
//! - `Uuid` → SQL `UUID`, Document `string`, KV `string`
//! - `Timestamp` → SQL `TIMESTAMPTZ`, Document `date`, KV `string`
//! - etc.

use std::fmt;

/// Backend-agnostic logical type for a field.
///
/// The `Display` impl uses PostgreSQL names as defaults.
/// For dialect-specific physical type names, use the dialect `TypeMap::resolve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FieldType {
    // ── Core scalar ──
    Text,
    /// Fixed-length string — `CHAR(n)` in SQL.
    Char(u32),
    /// Variable-length string — `VARCHAR(n)` or `VARCHAR` in SQL.
    Varchar(Option<u32>),
    Int,
    SmallInt,
    BigInt,
    Float,
    Double,
    Decimal,
    Bool,
    Uuid,
    Bytes,
    Timestamp,
    Date,
    Time,
    Duration,
    Json,
    Inet,

    // ── Composite ──
    /// Structured data — maps to JSONB in SQL, embedded document in doc stores.
    Object,
    /// Text array — maps to TEXT[] in PostgreSQL.
    TextArray,

    // ── Storage / binary ──
    /// Large binary object — maps to BYTEA in SQL.
    Blob,
    /// File or object path — maps to TEXT in SQL.
    Path,

    // ── Capability types ──
    /// URL string — maps to TEXT in SQL.
    Url,
    /// External resource identifier — maps to TEXT in SQL.
    ResourceId,
    /// Version number — maps to INTEGER in SQL.
    Version,
    /// Entity tag for caching — maps to TEXT in SQL.
    Etag,
    /// MIME type string — maps to TEXT in SQL.
    Mime,

    // ── Auto-increment ──
    Serial,
    BigSerial,

    // ── Custom / domain-defined ──
    Custom(&'static str),
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Core scalar — PostgreSQL names as defaults
            Self::Text => write!(f, "TEXT"),
            Self::Char(n) => write!(f, "CHAR({})", n),
            Self::Varchar(Some(n)) => write!(f, "VARCHAR({})", n),
            Self::Varchar(None) => write!(f, "VARCHAR"),
            Self::Int => write!(f, "INTEGER"),
            Self::SmallInt => write!(f, "SMALLINT"),
            Self::BigInt => write!(f, "BIGINT"),
            Self::Float => write!(f, "REAL"),
            Self::Double => write!(f, "DOUBLE PRECISION"),
            Self::Decimal => write!(f, "NUMERIC"),
            Self::Bool => write!(f, "BOOLEAN"),
            Self::Uuid => write!(f, "UUID"),
            Self::Bytes => write!(f, "BYTEA"),
            Self::Timestamp => write!(f, "TIMESTAMPTZ"),
            Self::Date => write!(f, "DATE"),
            Self::Time => write!(f, "TIME"),
            Self::Duration => write!(f, "INTERVAL"),
            Self::Json => write!(f, "JSONB"),
            Self::Inet => write!(f, "INET"),
            // Composite
            Self::Object => write!(f, "JSONB"),
            Self::TextArray => write!(f, "TEXT[]"),
            // Storage / binary
            Self::Blob => write!(f, "BYTEA"),
            Self::Path => write!(f, "TEXT"),
            // Capability types — all TEXT in SQL
            Self::Url => write!(f, "TEXT"),
            Self::ResourceId => write!(f, "TEXT"),
            Self::Version => write!(f, "INTEGER"),
            Self::Etag => write!(f, "TEXT"),
            Self::Mime => write!(f, "TEXT"),
            // Auto-increment
            Self::Serial => write!(f, "SERIAL"),
            Self::BigSerial => write!(f, "BIGSERIAL"),
            // Custom
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

// ── Serde round-trip support ──────────────────────────────────────────────
//
// `FieldType` uses `&'static str` for the `Custom` variant so that it stays
// `Copy` and works in `const`/`static` contexts.  serde's derive macro can
// only *serialize* a `&'static str`, not deserialize into one.
//
// We solve this with a mirror enum (`FieldTypeDe`) that uses `String` for
// the `Custom` payload and `Box::leak` to promote the string to `'static`.
// The leak is negligible in practice: custom type names are a small,
// bounded set (e.g. "CITEXT", "MONEY") typically loaded once at startup.

#[cfg(feature = "serde")]
mod serde_support {
    use super::FieldType;

    /// Mirror enum used only for deserialization.
    #[derive(serde::Deserialize)]
    enum FieldTypeDe {
        Text,
        Char(u32),
        Varchar(Option<u32>),
        Int,
        SmallInt,
        BigInt,
        Float,
        Double,
        Decimal,
        Bool,
        Uuid,
        Bytes,
        Timestamp,
        Date,
        Time,
        Duration,
        Json,
        Inet,
        Object,
        TextArray,
        Blob,
        Path,
        Url,
        ResourceId,
        Version,
        Etag,
        Mime,
        Serial,
        BigSerial,
        Custom(String),
    }

    impl From<FieldTypeDe> for FieldType {
        fn from(de: FieldTypeDe) -> Self {
            match de {
                FieldTypeDe::Text => Self::Text,
                FieldTypeDe::Char(n) => Self::Char(n),
                FieldTypeDe::Varchar(n) => Self::Varchar(n),
                FieldTypeDe::Int => Self::Int,
                FieldTypeDe::SmallInt => Self::SmallInt,
                FieldTypeDe::BigInt => Self::BigInt,
                FieldTypeDe::Float => Self::Float,
                FieldTypeDe::Double => Self::Double,
                FieldTypeDe::Decimal => Self::Decimal,
                FieldTypeDe::Bool => Self::Bool,
                FieldTypeDe::Uuid => Self::Uuid,
                FieldTypeDe::Bytes => Self::Bytes,
                FieldTypeDe::Timestamp => Self::Timestamp,
                FieldTypeDe::Date => Self::Date,
                FieldTypeDe::Time => Self::Time,
                FieldTypeDe::Duration => Self::Duration,
                FieldTypeDe::Json => Self::Json,
                FieldTypeDe::Inet => Self::Inet,
                FieldTypeDe::Object => Self::Object,
                FieldTypeDe::TextArray => Self::TextArray,
                FieldTypeDe::Blob => Self::Blob,
                FieldTypeDe::Path => Self::Path,
                FieldTypeDe::Url => Self::Url,
                FieldTypeDe::ResourceId => Self::ResourceId,
                FieldTypeDe::Version => Self::Version,
                FieldTypeDe::Etag => Self::Etag,
                FieldTypeDe::Mime => Self::Mime,
                FieldTypeDe::Serial => Self::Serial,
                FieldTypeDe::BigSerial => Self::BigSerial,
                FieldTypeDe::Custom(s) => {
                    // Promote the owned string to &'static str.
                    // This leaks the allocation, which is acceptable because custom
                    // type names are a small, bounded set loaded at startup.
                    Self::Custom(Box::leak(s.into_boxed_str()))
                }
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for FieldType {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            FieldTypeDe::deserialize(deserializer).map(Into::into)
        }
    }
}

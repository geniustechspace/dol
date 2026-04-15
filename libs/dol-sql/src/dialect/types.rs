//! Logical-to-physical type mapping for SQL dialects.

use std::collections::HashMap;

use dol_core::model::FieldType;

/// A dialect-neutral logical type that mirrors [`FieldType`] but is hashable and serializable.
///
/// `FieldType::Custom` has no mapping — it passes through as-is.
/// Capability types (`Url`, `ResourceId`, `Version`, `Etag`, `Mime`) and composite
/// aliases (`Object`, `Blob`, `Path`) are mapped to their underlying logical types.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LogicalType {
    Uuid,
    Text,
    SmallInt,
    Integer,
    BigInt,
    Boolean,
    Timestamptz,
    Jsonb,
    TextArray,
    Real,
    DoublePrecision,
    Numeric,
    Bytea,
    Date,
    Time,
    Interval,
    Serial,
    BigSerial,
    Inet,
}

impl LogicalType {
    /// Convert a `FieldType` reference to a `LogicalType`.
    /// Returns `None` for `FieldType::Custom` (which passes through as-is).
    ///
    /// Capability types and composite aliases are mapped to their underlying
    /// logical types (e.g. `FieldType::Url` → `LogicalType::Text`).
    pub fn from_field_type(field_type: &FieldType) -> Option<Self> {
        match field_type {
            FieldType::Uuid => Some(Self::Uuid),
            FieldType::Text => Some(Self::Text),
            FieldType::SmallInt => Some(Self::SmallInt),
            FieldType::Int => Some(Self::Integer),
            FieldType::BigInt => Some(Self::BigInt),
            FieldType::Bool => Some(Self::Boolean),
            FieldType::Timestamp => Some(Self::Timestamptz),
            FieldType::Json => Some(Self::Jsonb),
            FieldType::TextArray => Some(Self::TextArray),
            FieldType::Float => Some(Self::Real),
            FieldType::Double => Some(Self::DoublePrecision),
            FieldType::Decimal => Some(Self::Numeric),
            FieldType::Bytes => Some(Self::Bytea),
            FieldType::Date => Some(Self::Date),
            FieldType::Time => Some(Self::Time),
            FieldType::Duration => Some(Self::Interval),
            FieldType::Serial => Some(Self::Serial),
            FieldType::BigSerial => Some(Self::BigSerial),
            FieldType::Inet => Some(Self::Inet),
            // Composite aliases → underlying logical type
            FieldType::Object => Some(Self::Jsonb),
            FieldType::Blob => Some(Self::Bytea),
            FieldType::Path => Some(Self::Text),
            // Capability types → underlying logical type
            FieldType::Url => Some(Self::Text),
            FieldType::ResourceId => Some(Self::Text),
            FieldType::Version => Some(Self::Integer),
            FieldType::Etag => Some(Self::Text),
            FieldType::Mime => Some(Self::Text),
            // Parameterized types — handled directly in TypeMap::resolve()
            FieldType::Char(_) | FieldType::Varchar(_) => None,
            // Custom passes through — no logical mapping
            FieldType::Custom(_) => None,
        }
    }
}

/// Maps logical types to physical SQL type strings for a specific dialect.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeMap {
    mappings: HashMap<LogicalType, String>,
}

impl TypeMap {
    pub fn new(mappings: HashMap<LogicalType, String>) -> Self {
        Self { mappings }
    }

    /// Resolve a `FieldType` to its physical SQL string for this dialect.
    pub fn resolve(&self, field_type: &FieldType) -> String {
        match field_type {
            // Parameterized types — rendered directly, no LogicalType lookup
            FieldType::Custom(name) => name.to_string(),
            FieldType::Char(n) => format!("CHAR({})", n),
            FieldType::Varchar(Some(n)) => format!("VARCHAR({})", n),
            FieldType::Varchar(None) => "VARCHAR".to_string(),
            // All other types go through LogicalType → HashMap lookup
            other => {
                let logical = LogicalType::from_field_type(other)
                    .expect("all non-Custom/non-parameterized FieldType variants have a LogicalType mapping");
                self.mappings
                    .get(&logical)
                    .cloned()
                    .unwrap_or_else(|| panic!("no type mapping for {:?} in dialect", logical))
            }
        }
    }

    /// Override a single mapping.
    pub fn set(&mut self, logical: LogicalType, physical: impl Into<String>) {
        self.mappings.insert(logical, physical.into());
    }

    /// Get the mapping for a logical type (if any).
    pub fn get(&self, logical: &LogicalType) -> Option<&str> {
        self.mappings.get(logical).map(|s| s.as_str())
    }

    // -- Preset type maps --

    pub fn postgres() -> Self {
        Self::new(HashMap::from([
            (LogicalType::Uuid, "UUID".into()),
            (LogicalType::Text, "TEXT".into()),
            (LogicalType::SmallInt, "SMALLINT".into()),
            (LogicalType::Integer, "INTEGER".into()),
            (LogicalType::BigInt, "BIGINT".into()),
            (LogicalType::Boolean, "BOOLEAN".into()),
            (LogicalType::Timestamptz, "TIMESTAMPTZ".into()),
            (LogicalType::Jsonb, "JSONB".into()),
            (LogicalType::TextArray, "TEXT[]".into()),
            (LogicalType::Real, "REAL".into()),
            (LogicalType::DoublePrecision, "DOUBLE PRECISION".into()),
            (LogicalType::Numeric, "NUMERIC".into()),
            (LogicalType::Bytea, "BYTEA".into()),
            (LogicalType::Date, "DATE".into()),
            (LogicalType::Time, "TIME".into()),
            (LogicalType::Interval, "INTERVAL".into()),
            (LogicalType::Serial, "SERIAL".into()),
            (LogicalType::BigSerial, "BIGSERIAL".into()),
            (LogicalType::Inet, "INET".into()),
        ]))
    }

    pub fn mysql() -> Self {
        Self::new(HashMap::from([
            (LogicalType::Uuid, "CHAR(36)".into()),
            (LogicalType::Text, "TEXT".into()),
            (LogicalType::SmallInt, "SMALLINT".into()),
            (LogicalType::Integer, "INT".into()),
            (LogicalType::BigInt, "BIGINT".into()),
            (LogicalType::Boolean, "TINYINT(1)".into()),
            (LogicalType::Timestamptz, "DATETIME(6)".into()),
            (LogicalType::Jsonb, "JSON".into()),
            (LogicalType::TextArray, "JSON".into()),
            (LogicalType::Real, "FLOAT".into()),
            (LogicalType::DoublePrecision, "DOUBLE".into()),
            (LogicalType::Numeric, "DECIMAL".into()),
            (LogicalType::Bytea, "LONGBLOB".into()),
            (LogicalType::Date, "DATE".into()),
            (LogicalType::Time, "TIME".into()),
            (LogicalType::Interval, "VARCHAR(64)".into()),
            (LogicalType::Serial, "INT AUTO_INCREMENT".into()),
            (LogicalType::BigSerial, "BIGINT AUTO_INCREMENT".into()),
            (LogicalType::Inet, "VARCHAR(45)".into()),
        ]))
    }

    pub fn sqlite() -> Self {
        Self::new(HashMap::from([
            (LogicalType::Uuid, "TEXT".into()),
            (LogicalType::Text, "TEXT".into()),
            (LogicalType::SmallInt, "INTEGER".into()),
            (LogicalType::Integer, "INTEGER".into()),
            (LogicalType::BigInt, "INTEGER".into()),
            (LogicalType::Boolean, "INTEGER".into()),
            (LogicalType::Timestamptz, "TEXT".into()),
            (LogicalType::Jsonb, "TEXT".into()),
            (LogicalType::TextArray, "TEXT".into()),
            (LogicalType::Real, "REAL".into()),
            (LogicalType::DoublePrecision, "REAL".into()),
            (LogicalType::Numeric, "NUMERIC".into()),
            (LogicalType::Bytea, "BLOB".into()),
            (LogicalType::Date, "TEXT".into()),
            (LogicalType::Time, "TEXT".into()),
            (LogicalType::Interval, "TEXT".into()),
            (LogicalType::Serial, "INTEGER".into()),
            (LogicalType::BigSerial, "INTEGER".into()),
            (LogicalType::Inet, "TEXT".into()),
        ]))
    }

    pub fn mssql() -> Self {
        Self::new(HashMap::from([
            (LogicalType::Uuid, "UNIQUEIDENTIFIER".into()),
            (LogicalType::Text, "NVARCHAR(MAX)".into()),
            (LogicalType::SmallInt, "SMALLINT".into()),
            (LogicalType::Integer, "INT".into()),
            (LogicalType::BigInt, "BIGINT".into()),
            (LogicalType::Boolean, "BIT".into()),
            (LogicalType::Timestamptz, "DATETIMEOFFSET".into()),
            (LogicalType::Jsonb, "NVARCHAR(MAX)".into()),
            (LogicalType::TextArray, "NVARCHAR(MAX)".into()),
            (LogicalType::Real, "REAL".into()),
            (LogicalType::DoublePrecision, "FLOAT".into()),
            (LogicalType::Numeric, "DECIMAL".into()),
            (LogicalType::Bytea, "VARBINARY(MAX)".into()),
            (LogicalType::Date, "DATE".into()),
            (LogicalType::Time, "TIME".into()),
            (LogicalType::Interval, "VARCHAR(64)".into()),
            (LogicalType::Serial, "INT IDENTITY(1,1)".into()),
            (LogicalType::BigSerial, "BIGINT IDENTITY(1,1)".into()),
            (LogicalType::Inet, "VARCHAR(45)".into()),
        ]))
    }

    pub fn oracle() -> Self {
        Self::new(HashMap::from([
            (LogicalType::Uuid, "RAW(16)".into()),
            (LogicalType::Text, "CLOB".into()),
            (LogicalType::SmallInt, "NUMBER(5)".into()),
            (LogicalType::Integer, "NUMBER(10)".into()),
            (LogicalType::BigInt, "NUMBER(19)".into()),
            (LogicalType::Boolean, "NUMBER(1)".into()),
            (LogicalType::Timestamptz, "TIMESTAMP WITH TIME ZONE".into()),
            (LogicalType::Jsonb, "CLOB".into()),
            (LogicalType::TextArray, "CLOB".into()),
            (LogicalType::Real, "BINARY_FLOAT".into()),
            (LogicalType::DoublePrecision, "BINARY_DOUBLE".into()),
            (LogicalType::Numeric, "NUMBER".into()),
            (LogicalType::Bytea, "BLOB".into()),
            (LogicalType::Date, "DATE".into()),
            (LogicalType::Time, "DATE".into()),
            (LogicalType::Interval, "INTERVAL DAY TO SECOND".into()),
            (
                LogicalType::Serial,
                "NUMBER GENERATED ALWAYS AS IDENTITY".into(),
            ),
            (
                LogicalType::BigSerial,
                "NUMBER GENERATED ALWAYS AS IDENTITY".into(),
            ),
            (LogicalType::Inet, "VARCHAR2(45)".into()),
        ]))
    }
}

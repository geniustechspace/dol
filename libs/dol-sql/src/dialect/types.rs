//! Direct DataType-to-SQL rendering for each dialect.
//!
//! Replaces the old LogicalType/TypeMap indirection with a single function
//! that maps `DataType` variants directly to SQL type strings per dialect.

use dol_entity::DataType;

/// The dialect name, used to select the correct type rendering.
///
/// This is a simple enum rather than a trait so that dialects remain
/// serializable, clonable data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TypeDialect {
    Postgres,
    MySQL,
    SQLite,
    MSSQL,
    Oracle,
}

/// Render a `DataType` to the physical SQL type string for the given dialect.
pub fn render_data_type(dt: &DataType, dialect: &TypeDialect) -> String {
    match dialect {
        TypeDialect::Postgres  => render_postgres(dt),
        TypeDialect::MySQL     => render_mysql(dt),
        TypeDialect::SQLite    => render_sqlite(dt),
        TypeDialect::MSSQL     => render_mssql(dt),
        TypeDialect::Oracle    => render_oracle(dt),
    }
}

// ── PostgreSQL ──────────────────────────────────────────────────────────

fn render_postgres(dt: &DataType) -> String {
    match dt {
        // Primitive
        DataType::Null  => "NULL".into(),
        DataType::Bool  => "BOOLEAN".into(),

        // Integer
        DataType::Int8  | DataType::UInt8  => "SMALLINT".into(),
        DataType::Int16 | DataType::UInt16 => "SMALLINT".into(),
        DataType::Int32 | DataType::UInt32 => "INTEGER".into(),
        DataType::Int64 | DataType::UInt64 => "BIGINT".into(),
        DataType::Int128 | DataType::UInt128 => "NUMERIC(38,0)".into(),

        // Float
        DataType::Float32 => "REAL".into(),
        DataType::Float64 => "DOUBLE PRECISION".into(),

        // Decimal
        DataType::Decimal { precision: Some(p), scale: Some(s) }
            => format!("NUMERIC({},{})", p, s),
        DataType::Decimal { precision: Some(p), scale: None }
            => format!("NUMERIC({})", p),
        DataType::Decimal { .. } => "NUMERIC".into(),

        // Text
        DataType::Char(n)           => format!("CHAR({})", n),
        DataType::Varchar(Some(n))  => format!("VARCHAR({})", n),
        DataType::Varchar(None)     => "VARCHAR".into(),
        DataType::Text              => "TEXT".into(),
        DataType::Json              => "JSONB".into(),
        DataType::Xml               => "XML".into(),

        // Binary
        DataType::Binary(_)         => "BYTEA".into(), // PG has no BINARY(n); just BYTEA
        DataType::Varbinary(_)      => "BYTEA".into(),
        DataType::Uuid              => "UUID".into(),

        // Bit string
        DataType::Bit(n)            => format!("BIT({})", n),
        DataType::Varbit(Some(n))   => format!("VARBIT({})", n),
        DataType::Varbit(None)      => "VARBIT".into(),

        // Temporal
        DataType::Date                    => "DATE".into(),
        DataType::Time { precision }      => format!("TIME({})", precision),
        DataType::DateTime { precision }  => format!("TIMESTAMP({})", precision),
        DataType::TimestampTz { precision } => format!("TIMESTAMPTZ({})", precision),
        DataType::Interval                => "INTERVAL".into(),

        // Network
        DataType::Inet         => "INET".into(),
        DataType::Cidr { .. }  => "CIDR".into(),
        DataType::MacAddr      => "MACADDR".into(),
        DataType::MacAddr8     => "MACADDR8".into(),

        // Geometric
        DataType::Point       => "POINT".into(),
        DataType::Line        => "LINE".into(),
        DataType::LineSegment => "LSEG".into(),
        DataType::Rect        => "BOX".into(),
        DataType::Circle      => "CIRCLE".into(),
        DataType::Path        => "PATH".into(),
        DataType::Polygon     => "POLYGON".into(),

        // Composite
        DataType::Array(elem) => format!("{}[]", render_postgres(elem)),
        DataType::Set(elem)   => format!("{}[]", render_postgres(elem)), // PG has no SET; use array
        DataType::Map { value: _ } => "JSONB".into(), // PG has no MAP; use JSONB
        DataType::Range(elem) => {
            let base = match elem.as_ref() {
                DataType::Int32 | DataType::Int64 => "int8range",
                DataType::Date => "daterange",
                DataType::TimestampTz { .. } => "tstzrange",
                _ => "numrange",
            };
            base.to_string()
        }
        DataType::Tuple(_)    => "RECORD".into(),
        DataType::Struct(_)   => "RECORD".into(),
        DataType::Enum(_)     => "TEXT".into(), // Enums are rendered via CREATE TYPE separately

        // Meta
        DataType::Named(name) => name.to_string(),

        // Semantic — stored as text in PG
        DataType::Url | DataType::Mime | DataType::FilePath | DataType::Version
            => "TEXT".into(),
    }
}

// ── MySQL ───────────────────────────────────────────────────────────────

fn render_mysql(dt: &DataType) -> String {
    match dt {
        DataType::Null  => "NULL".into(),
        DataType::Bool  => "TINYINT(1)".into(),
        DataType::Int8 | DataType::UInt8   => "TINYINT".into(),
        DataType::Int16 | DataType::UInt16 => "SMALLINT".into(),
        DataType::Int32 | DataType::UInt32 => "INT".into(),
        DataType::Int64 | DataType::UInt64 => "BIGINT".into(),
        DataType::Int128 | DataType::UInt128 => "DECIMAL(38,0)".into(),
        DataType::Float32 => "FLOAT".into(),
        DataType::Float64 => "DOUBLE".into(),
        DataType::Decimal { precision: Some(p), scale: Some(s) }
            => format!("DECIMAL({},{})", p, s),
        DataType::Decimal { precision: Some(p), scale: None }
            => format!("DECIMAL({})", p),
        DataType::Decimal { .. } => "DECIMAL".into(),
        DataType::Char(n)          => format!("CHAR({})", n),
        DataType::Varchar(Some(n)) => format!("VARCHAR({})", n),
        DataType::Varchar(None)    => "VARCHAR(255)".into(),
        DataType::Text             => "TEXT".into(),
        DataType::Json             => "JSON".into(),
        DataType::Xml              => "TEXT".into(),
        DataType::Binary(n)        => format!("BINARY({})", n),
        DataType::Varbinary(Some(n)) => format!("VARBINARY({})", n),
        DataType::Varbinary(None)  => "LONGBLOB".into(),
        DataType::Uuid             => "CHAR(36)".into(),
        DataType::Bit(n)           => format!("BIT({})", n),
        DataType::Varbit(_)        => "BIT(64)".into(),
        DataType::Date             => "DATE".into(),
        DataType::Time { .. }      => "TIME".into(),
        DataType::DateTime { precision } => format!("DATETIME({})", precision),
        DataType::TimestampTz { precision } => format!("DATETIME({})", precision),
        DataType::Interval         => "VARCHAR(64)".into(),
        DataType::Inet | DataType::Cidr { .. } => "VARCHAR(45)".into(),
        DataType::MacAddr          => "VARCHAR(17)".into(),
        DataType::MacAddr8         => "VARCHAR(23)".into(),
        DataType::Point | DataType::Line | DataType::LineSegment
        | DataType::Rect | DataType::Circle | DataType::Path
        | DataType::Polygon => "GEOMETRY".into(),
        DataType::Array(_) | DataType::Set(_) | DataType::Map { .. }
        | DataType::Tuple(_) | DataType::Struct(_) => "JSON".into(),
        DataType::Range(_) => "JSON".into(),
        DataType::Enum(variants) => {
            let vs: Vec<String> = variants.iter().map(|v| format!("'{}'", v)).collect();
            format!("ENUM({})", vs.join(", "))
        }
        DataType::Named(name) => name.to_string(),
        DataType::Url | DataType::Mime | DataType::FilePath | DataType::Version
            => "TEXT".into(),
    }
}

// ── SQLite ──────────────────────────────────────────────────────────────

fn render_sqlite(dt: &DataType) -> String {
    match dt {
        DataType::Null  => "NULL".into(),
        DataType::Bool  => "INTEGER".into(),
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
        | DataType::Int128 | DataType::UInt8 | DataType::UInt16 | DataType::UInt32
        | DataType::UInt64 | DataType::UInt128 => "INTEGER".into(),
        DataType::Float32 | DataType::Float64 => "REAL".into(),
        DataType::Decimal { .. }  => "NUMERIC".into(),
        DataType::Char(_) | DataType::Varchar(_) | DataType::Text
        | DataType::Json | DataType::Xml => "TEXT".into(),
        DataType::Binary(_) | DataType::Varbinary(_) => "BLOB".into(),
        DataType::Uuid => "TEXT".into(),
        DataType::Bit(_) | DataType::Varbit(_) => "INTEGER".into(),
        DataType::Date | DataType::Time { .. } | DataType::DateTime { .. }
        | DataType::TimestampTz { .. } | DataType::Interval => "TEXT".into(),
        DataType::Inet | DataType::Cidr { .. } | DataType::MacAddr
        | DataType::MacAddr8 => "TEXT".into(),
        DataType::Point | DataType::Line | DataType::LineSegment
        | DataType::Rect | DataType::Circle | DataType::Path
        | DataType::Polygon => "TEXT".into(),
        DataType::Array(_) | DataType::Set(_) | DataType::Map { .. }
        | DataType::Range(_) | DataType::Tuple(_) | DataType::Struct(_) => "TEXT".into(),
        DataType::Enum(_) => "TEXT".into(),
        DataType::Named(name) => name.to_string(),
        DataType::Url | DataType::Mime | DataType::FilePath | DataType::Version
            => "TEXT".into(),
    }
}

// ── MSSQL ───────────────────────────────────────────────────────────────

fn render_mssql(dt: &DataType) -> String {
    match dt {
        DataType::Null  => "NULL".into(),
        DataType::Bool  => "BIT".into(),
        DataType::Int8 | DataType::UInt8 => "TINYINT".into(),
        DataType::Int16 | DataType::UInt16 => "SMALLINT".into(),
        DataType::Int32 | DataType::UInt32 => "INT".into(),
        DataType::Int64 | DataType::UInt64 => "BIGINT".into(),
        DataType::Int128 | DataType::UInt128 => "DECIMAL(38,0)".into(),
        DataType::Float32 => "REAL".into(),
        DataType::Float64 => "FLOAT".into(),
        DataType::Decimal { precision: Some(p), scale: Some(s) }
            => format!("DECIMAL({},{})", p, s),
        DataType::Decimal { precision: Some(p), scale: None }
            => format!("DECIMAL({})", p),
        DataType::Decimal { .. } => "DECIMAL".into(),
        DataType::Char(n)          => format!("CHAR({})", n),
        DataType::Varchar(Some(n)) => format!("VARCHAR({})", n),
        DataType::Varchar(None)    => "NVARCHAR(MAX)".into(),
        DataType::Text             => "NVARCHAR(MAX)".into(),
        DataType::Json             => "NVARCHAR(MAX)".into(),
        DataType::Xml              => "XML".into(),
        DataType::Binary(n)        => format!("BINARY({})", n),
        DataType::Varbinary(Some(n)) => format!("VARBINARY({})", n),
        DataType::Varbinary(None)  => "VARBINARY(MAX)".into(),
        DataType::Uuid             => "UNIQUEIDENTIFIER".into(),
        DataType::Bit(n)           => format!("BINARY({})", n.div_ceil(8)),
        DataType::Varbit(_)        => "VARBINARY(MAX)".into(),
        DataType::Date             => "DATE".into(),
        DataType::Time { .. }      => "TIME".into(),
        DataType::DateTime { .. }  => "DATETIME2".into(),
        DataType::TimestampTz { .. } => "DATETIMEOFFSET".into(),
        DataType::Interval         => "VARCHAR(64)".into(),
        DataType::Inet | DataType::Cidr { .. } => "VARCHAR(45)".into(),
        DataType::MacAddr          => "VARCHAR(17)".into(),
        DataType::MacAddr8         => "VARCHAR(23)".into(),
        DataType::Point | DataType::Line | DataType::LineSegment
        | DataType::Rect | DataType::Circle | DataType::Path
        | DataType::Polygon => "GEOMETRY".into(),
        DataType::Array(_) | DataType::Set(_) | DataType::Map { .. }
        | DataType::Range(_) | DataType::Tuple(_) | DataType::Struct(_) => "NVARCHAR(MAX)".into(),
        DataType::Enum(_)  => "NVARCHAR(100)".into(),
        DataType::Named(name) => name.to_string(),
        DataType::Url | DataType::Mime | DataType::FilePath | DataType::Version
            => "NVARCHAR(MAX)".into(),
    }
}

// ── Oracle ──────────────────────────────────────────────────────────────

fn render_oracle(dt: &DataType) -> String {
    match dt {
        DataType::Null  => "NULL".into(),
        DataType::Bool  => "NUMBER(1)".into(),
        DataType::Int8 | DataType::UInt8   => "NUMBER(3)".into(),
        DataType::Int16 | DataType::UInt16 => "NUMBER(5)".into(),
        DataType::Int32 | DataType::UInt32 => "NUMBER(10)".into(),
        DataType::Int64 | DataType::UInt64 => "NUMBER(19)".into(),
        DataType::Int128 | DataType::UInt128 => "NUMBER(38)".into(),
        DataType::Float32 => "BINARY_FLOAT".into(),
        DataType::Float64 => "BINARY_DOUBLE".into(),
        DataType::Decimal { precision: Some(p), scale: Some(s) }
            => format!("NUMBER({},{})", p, s),
        DataType::Decimal { precision: Some(p), scale: None }
            => format!("NUMBER({})", p),
        DataType::Decimal { .. } => "NUMBER".into(),
        DataType::Char(n)          => format!("CHAR({})", n),
        DataType::Varchar(Some(n)) => format!("VARCHAR2({})", n),
        DataType::Varchar(None)    => "CLOB".into(),
        DataType::Text             => "CLOB".into(),
        DataType::Json             => "CLOB".into(),
        DataType::Xml              => "XMLTYPE".into(),
        DataType::Binary(n)        => format!("RAW({})", n),
        DataType::Varbinary(Some(n)) => format!("RAW({})", n),
        DataType::Varbinary(None)  => "BLOB".into(),
        DataType::Uuid             => "RAW(16)".into(),
        DataType::Bit(_) | DataType::Varbit(_) => "RAW(8)".into(),
        DataType::Date             => "DATE".into(),
        DataType::Time { .. }      => "DATE".into(),
        DataType::DateTime { .. }  => "TIMESTAMP".into(),
        DataType::TimestampTz { .. } => "TIMESTAMP WITH TIME ZONE".into(),
        DataType::Interval         => "INTERVAL DAY TO SECOND".into(),
        DataType::Inet | DataType::Cidr { .. } => "VARCHAR2(45)".into(),
        DataType::MacAddr          => "VARCHAR2(17)".into(),
        DataType::MacAddr8         => "VARCHAR2(23)".into(),
        DataType::Point | DataType::Line | DataType::LineSegment
        | DataType::Rect | DataType::Circle | DataType::Path
        | DataType::Polygon => "SDO_GEOMETRY".into(),
        DataType::Array(_) | DataType::Set(_) | DataType::Map { .. }
        | DataType::Range(_) | DataType::Tuple(_) | DataType::Struct(_) => "CLOB".into(),
        DataType::Enum(_)  => "VARCHAR2(100)".into(),
        DataType::Named(name) => name.to_string(),
        DataType::Url | DataType::Mime | DataType::FilePath | DataType::Version
            => "CLOB".into(),
    }
}

/// Dialect configuration framework for multi-database SQL rendering.
///
/// A `Dialect` captures all differences between database engines as structured data:
/// parameter style, quoting, type mappings, feature flags, pagination, upsert style,
/// returning clauses, locking, DDL capabilities, and more.
///
/// # Usage
///
/// ```rust
/// use dol_sql::dialect::Dialect;
///
/// // Use a preset
/// let pg = Dialect::postgres();
/// let mysql = Dialect::mysql();
///
/// // Load from a config file
/// // let custom = Dialect::from_file("dialects/my_db.toml").unwrap();
///
/// // Set the global default (one-shot, typically at startup)
/// // dol_sql::dialect::set_default_dialect(pg);
/// ```
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub mod concat;
#[cfg(feature = "config")]
pub mod config;
pub mod ddl;
pub mod features;
pub mod locking;
pub mod pagination;
pub mod param;
mod presets;
pub mod quoting;
pub mod returning;
pub mod types;
pub mod upsert;

pub use concat::ConcatStyle;
#[cfg(feature = "config")]
pub use config::DialectConfigError;
pub use ddl::{AutoIncrementStyle, DdlCapabilities, EnumStyle};
pub use features::DialectFeatures;
pub use locking::LockingCapabilities;
pub use pagination::PaginationStyle;
pub use param::{ParamCounter, ParamStyle};
pub use quoting::QuoteStyle;
pub use returning::ReturningStyle;
pub use types::{LogicalType, TypeMap};
pub use upsert::UpsertStyle;

/// How nested field / JSON access is rendered in SQL.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub enum JsonAccessStyle {
    /// `->>` operator (PostgreSQL, CockroachDB).
    ArrowOperator,
    /// `json_extract(col, '$.path')` (MySQL 5.7+, SQLite json1).
    JsonExtractFunction,
    /// `JSON_VALUE(col, '$.path')` (SQL Server 2016+).
    JsonValueFunction,
    /// Not supported by this dialect.
    #[default]
    Unsupported,
}

/// How array literals are rendered in SQL.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub enum ArrayLiteralStyle {
    /// `ARRAY[1, 2, 3]` (PostgreSQL, CockroachDB).
    ArrayKeyword,
    /// `JSON_ARRAY(1, 2, 3)` (MySQL 8+, SQLite json1).
    JsonArrayFunction,
    /// Not natively supported.
    #[default]
    Unsupported,
}

/// Extension point for future non-SQL store kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub enum StoreKind {
    /// Relational SQL database (default).
    #[default]
    Sql,
    /// Document store (e.g. MongoDB, DynamoDB).
    Document,
    /// Column-family store (e.g. Cassandra, HBase).
    ColumnFamily,
    /// Key-value store (e.g. Redis, etcd).
    KeyValue,
    /// Graph database (e.g. Neo4j, Neptune).
    Graph,
}

/// Complete dialect configuration capturing all database-specific behaviors.
///
/// This is a **data struct**, not a trait — it can be deserialized from config files
/// (JSON, TOML, YAML) or constructed programmatically via preset methods.
#[derive(Debug, Clone, Deserialize)]
pub struct Dialect {
    /// Human-readable dialect name (e.g. "postgresql", "mysql").
    pub name: String,
    /// Bind parameter placeholder style.
    pub param_style: ParamStyle,
    /// Identifier quoting style.
    pub quote_style: QuoteStyle,
    /// Logical-to-physical type mapping.
    pub type_map: TypeMap,
    /// Pagination clause style.
    pub pagination: PaginationStyle,
    /// Upsert (INSERT or UPDATE) style.
    pub upsert_style: UpsertStyle,
    /// RETURNING clause style.
    pub returning_style: ReturningStyle,
    /// Row-level locking capabilities.
    pub locking: LockingCapabilities,
    /// DDL capabilities and styles.
    pub ddl: DdlCapabilities,
    /// Feature flags for optional SQL constructs.
    pub features: DialectFeatures,
    /// SQL literal for boolean TRUE.
    #[serde(default = "default_true_str")]
    pub bool_true: String,
    /// SQL literal for boolean FALSE.
    #[serde(default = "default_false_str")]
    pub bool_false: String,
    /// String concatenation style.
    pub concat_style: ConcatStyle,
    /// Store kind — extension point for non-SQL databases.
    #[serde(default)]
    pub store_kind: StoreKind,
    /// JSON/nested field access style.
    #[serde(default)]
    pub json_access: JsonAccessStyle,
    /// Array literal rendering style.
    #[serde(default)]
    pub array_literal_style: ArrayLiteralStyle,
}

fn default_true_str() -> String {
    "TRUE".into()
}

fn default_false_str() -> String {
    "FALSE".into()
}

impl Dialect {
    /// Create a new `ParamCounter` for this dialect.
    pub fn param_counter(&self) -> ParamCounter {
        ParamCounter::new(&self.param_style)
    }

    /// Quote an identifier using this dialect's quoting style.
    pub fn quote_ident(&self, name: &str) -> String {
        self.quote_style.quote(name)
    }

    /// Resolve a `FieldType` to the physical SQL string for this dialect.
    pub fn resolve_type(&self, field_type: &dol_core::model::FieldType) -> String {
        self.type_map.resolve(field_type)
    }
}

// -- Global default dialect --

static DEFAULT_DIALECT: OnceLock<Dialect> = OnceLock::new();

/// Returns the global default dialect.
///
/// Defaults to SQLite if `set_default_dialect` was never called.
pub fn default_dialect() -> &'static Dialect {
    DEFAULT_DIALECT.get_or_init(Dialect::sqlite)
}

/// Set the global default dialect. Can only be called once (typically at startup).
///
/// Returns `Err(dialect)` (boxed) if the default has already been set.
pub fn set_default_dialect(dialect: Dialect) -> Result<(), Box<Dialect>> {
    DEFAULT_DIALECT.set(dialect).map_err(Box::new)
}

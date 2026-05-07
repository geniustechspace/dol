//! Type-body classification for named types in schema catalogs.

/// Type-body classification for named types.
///
/// Used by `SchemaCatalog` type entries and schema-level DDL operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TypeBody {
    /// Enumerated type (e.g. PostgreSQL `CREATE TYPE ... AS ENUM`).
    Enum,
    /// Composite / record type (e.g. PostgreSQL `CREATE TYPE ... AS (...)`.
    Composite,
    /// Distinct (domain) type with constraints.
    Distinct,
    /// Backend-specific type kind.
    Other,
}

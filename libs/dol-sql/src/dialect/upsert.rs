//! Upsert styles for different SQL dialects.

/// How a dialect implements INSERT-or-UPDATE (upsert) semantics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UpsertStyle {
    /// `INSERT ... ON CONFLICT (cols) DO UPDATE SET ...` (PostgreSQL, SQLite, CockroachDB).
    #[default]
    OnConflict,
    /// `INSERT ... ON DUPLICATE KEY UPDATE ...` (MySQL, MariaDB).
    OnDuplicateKey,
    /// `MERGE INTO target USING source ON ... WHEN MATCHED THEN ... WHEN NOT MATCHED THEN ...`
    /// (SQL Server, Oracle).
    Merge,
    /// Dialect does not support upsert natively.
    Unsupported,
}

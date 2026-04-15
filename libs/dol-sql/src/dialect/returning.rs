//! RETURNING clause styles for different SQL dialects.

/// How a dialect returns rows affected by INSERT/UPDATE/DELETE.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ReturningStyle {
    /// `RETURNING col1, col2` (PostgreSQL, SQLite 3.35+, CockroachDB).
    #[default]
    Returning,
    /// `OUTPUT INSERTED.col1, INSERTED.col2` (SQL Server).
    OutputInserted,
    /// `RETURNING col1, col2 INTO var1, var2` (Oracle — PL/SQL only).
    ReturningInto,
    /// Dialect does not support returning clauses.
    Unsupported,
}

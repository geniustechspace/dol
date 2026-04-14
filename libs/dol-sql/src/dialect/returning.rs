/// RETURNING clause styles for different SQL dialects.
use serde::{Deserialize, Serialize};

/// How a dialect returns rows affected by INSERT/UPDATE/DELETE.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// String concatenation styles for different SQL dialects.
use serde::{Deserialize, Serialize};

/// How a dialect concatenates strings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub enum ConcatStyle {
    /// `a || b` (PostgreSQL, SQLite, Oracle, CockroachDB).
    #[default]
    PipeOperator,
    /// `CONCAT(a, b)` function (MySQL, MariaDB).
    ConcatFunction,
    /// `a + b` (SQL Server).
    PlusOperator,
}

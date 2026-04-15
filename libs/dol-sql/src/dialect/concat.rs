//! String concatenation styles for different SQL dialects.

/// How a dialect concatenates strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConcatStyle {
    /// `a || b` (PostgreSQL, SQLite, Oracle, CockroachDB).
    #[default]
    PipeOperator,
    /// `CONCAT(a, b)` function (MySQL, MariaDB).
    ConcatFunction,
    /// `a + b` (SQL Server).
    PlusOperator,
}

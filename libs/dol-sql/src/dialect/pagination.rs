//! Pagination styles for different SQL dialects.

/// How a dialect implements result-set pagination.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaginationStyle {
    /// `LIMIT n OFFSET m` (PostgreSQL, MySQL, SQLite, MariaDB, CockroachDB).
    #[default]
    LimitOffset,
    /// `OFFSET m ROWS FETCH NEXT n ROWS ONLY` (SQL Server 2012+, Oracle 12c+).
    OffsetFetch,
    /// Legacy Oracle: `WHERE ROWNUM <= n` (pre-12c).
    Rownum,
}

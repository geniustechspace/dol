/// Pagination styles for different SQL dialects.
use serde::{Deserialize, Serialize};

/// How a dialect implements result-set pagination.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaginationStyle {
    /// `LIMIT n OFFSET m` (PostgreSQL, MySQL, SQLite, MariaDB, CockroachDB).
    #[default]
    LimitOffset,
    /// `OFFSET m ROWS FETCH NEXT n ROWS ONLY` (SQL Server 2012+, Oracle 12c+).
    OffsetFetch,
    /// Legacy Oracle: `WHERE ROWNUM <= n` (pre-12c).
    Rownum,
}

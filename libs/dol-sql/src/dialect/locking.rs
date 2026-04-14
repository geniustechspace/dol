/// Row-level locking capabilities per SQL dialect.
use serde::{Deserialize, Serialize};

/// Which row-level locking clauses the dialect supports.
#[derive(Debug, Clone, Deserialize)]
pub struct LockingCapabilities {
    /// Supports `FOR UPDATE`.
    pub for_update: bool,
    /// Supports `FOR SHARE` / `FOR KEY SHARE`.
    pub for_share: bool,
    /// Supports `SKIP LOCKED`.
    pub skip_locked: bool,
    /// Supports `NOWAIT`.
    pub nowait: bool,
    /// Uses table hints instead of `FOR UPDATE` (SQL Server: `WITH (UPDLOCK, ROWLOCK)`).
    pub use_table_hint: bool,
}

impl Default for LockingCapabilities {
    fn default() -> Self {
        Self {
            for_update: true,
            for_share: true,
            skip_locked: false,
            nowait: false,
            use_table_hint: false,
        }
    }
}

impl LockingCapabilities {
    pub fn postgres() -> Self {
        Self {
            for_update: true,
            for_share: true,
            skip_locked: true,
            nowait: true,
            use_table_hint: false,
        }
    }

    pub fn mysql() -> Self {
        Self {
            for_update: true,
            for_share: true,
            skip_locked: true,
            nowait: true,
            use_table_hint: false,
        }
    }

    pub fn sqlite() -> Self {
        // SQLite uses database-level locking, not row-level.
        Self {
            for_update: false,
            for_share: false,
            skip_locked: false,
            nowait: false,
            use_table_hint: false,
        }
    }

    pub fn mssql() -> Self {
        Self {
            for_update: false,
            for_share: false,
            skip_locked: false,
            nowait: true,
            use_table_hint: true,
        }
    }

    pub fn oracle() -> Self {
        Self {
            for_update: true,
            for_share: false,
            skip_locked: true,
            nowait: true,
            use_table_hint: false,
        }
    }
}

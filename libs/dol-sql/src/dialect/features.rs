/// Dialect feature flags for SQL constructs that vary by database.
use serde::{Deserialize, Serialize};

/// Feature flags indicating which SQL constructs a dialect supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectFeatures {
    /// Supports `SELECT DISTINCT ON (cols)` (PostgreSQL-specific).
    pub distinct_on: bool,
    /// Supports `ILIKE` operator (case-insensitive LIKE).
    pub ilike: bool,
    /// Supports `= ANY($1)` for array bind parameters.
    pub array_any: bool,
    /// Supports `NULLS FIRST` / `NULLS LAST` in ORDER BY.
    pub nulls_ordering: bool,
    /// Supports `DO $$ ... END $$` anonymous blocks.
    pub anonymous_blocks: bool,
    /// Supports schema-qualified names (`schema.table`).
    pub schemas: bool,
    /// Supports Common Table Expressions (`WITH ... AS`).
    pub cte: bool,
    /// Supports window functions (`OVER (PARTITION BY ...)`).
    pub window_functions: bool,
    /// Supports lateral joins (`LATERAL`).
    pub lateral_join: bool,
    /// Supports `ON CONFLICT` clause in INSERT for upserts.
    pub on_conflict: bool,
}

impl Default for DialectFeatures {
    fn default() -> Self {
        Self {
            distinct_on: false,
            ilike: false,
            array_any: false,
            nulls_ordering: true,
            anonymous_blocks: false,
            schemas: true,
            cte: true,
            window_functions: true,
            lateral_join: false,
            on_conflict: false,
        }
    }
}

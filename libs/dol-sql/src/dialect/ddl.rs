//! DDL capabilities and styles for different SQL dialects.

/// How a dialect handles enum types.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EnumStyle {
    /// `CREATE TYPE name AS ENUM ('a', 'b')` (PostgreSQL, CockroachDB).
    #[default]
    CreateType,
    /// `ENUM('a', 'b')` inline in column definition (MySQL, MariaDB).
    InlineEnum,
    /// `CHECK (col IN ('a', 'b'))` constraint (SQLite, SQL Server, Oracle).
    CheckConstraint,
}

/// How a dialect handles auto-incrementing primary keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AutoIncrementStyle {
    /// `SERIAL` / `BIGSERIAL` pseudo-types (PostgreSQL).
    #[default]
    SerialType,
    /// `AUTO_INCREMENT` keyword after column type (MySQL, MariaDB).
    AutoIncrement,
    /// `IDENTITY(1,1)` (SQL Server).
    Identity,
    /// `GENERATED ALWAYS AS IDENTITY` (Oracle, PostgreSQL 10+).
    GeneratedIdentity,
    /// `AUTOINCREMENT` keyword (SQLite — only on INTEGER PRIMARY KEY).
    Autoincrement,
}

/// DDL capabilities that differ across dialects.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DdlCapabilities {
    /// Supports `CREATE TABLE IF NOT EXISTS`.
    pub create_if_not_exists: bool,
    /// Supports `DROP TABLE IF EXISTS`.
    pub drop_if_exists: bool,
    /// Supports `CREATE INDEX CONCURRENTLY` (PostgreSQL).
    pub index_concurrently: bool,
    /// How enums are defined.
    pub enum_style: EnumStyle,
    /// How auto-incrementing columns are defined.
    pub auto_increment_style: AutoIncrementStyle,
    /// Supports `ALTER TABLE ... ADD COLUMN`.
    pub alter_add_column: bool,
    /// Supports `ALTER TABLE ... DROP COLUMN`.
    pub alter_drop_column: bool,
    /// Supports `ALTER TABLE ... RENAME COLUMN`.
    pub alter_rename_column: bool,
    /// Supports `ALTER TABLE ... ALTER COLUMN` / `MODIFY COLUMN`.
    pub alter_modify_column: bool,
    /// Supports `ALTER TABLE ... RENAME TO`.
    pub alter_rename_table: bool,
    /// Supports transactional DDL (DDL inside a transaction block).
    pub transactional_ddl: bool,
    /// Supports `CASCADE` on DROP TABLE / DROP TYPE.
    pub drop_cascade: bool,
}

impl Default for DdlCapabilities {
    fn default() -> Self {
        Self {
            create_if_not_exists: true,
            drop_if_exists: true,
            index_concurrently: false,
            enum_style: EnumStyle::default(),
            auto_increment_style: AutoIncrementStyle::default(),
            alter_add_column: true,
            alter_drop_column: true,
            alter_rename_column: true,
            alter_modify_column: true,
            alter_rename_table: true,
            transactional_ddl: true,
            drop_cascade: true,
        }
    }
}

impl DdlCapabilities {
    pub fn postgres() -> Self {
        Self {
            create_if_not_exists: true,
            drop_if_exists: true,
            index_concurrently: true,
            enum_style: EnumStyle::CreateType,
            auto_increment_style: AutoIncrementStyle::SerialType,
            alter_add_column: true,
            alter_drop_column: true,
            alter_rename_column: true,
            alter_modify_column: true,
            alter_rename_table: true,
            transactional_ddl: true,
            drop_cascade: true,
        }
    }

    pub fn mysql() -> Self {
        Self {
            create_if_not_exists: true,
            drop_if_exists: true,
            index_concurrently: false,
            enum_style: EnumStyle::InlineEnum,
            auto_increment_style: AutoIncrementStyle::AutoIncrement,
            alter_add_column: true,
            alter_drop_column: true,
            alter_rename_column: true,
            alter_modify_column: true,
            alter_rename_table: true,
            transactional_ddl: false,
            drop_cascade: false,
        }
    }

    pub fn sqlite() -> Self {
        Self {
            create_if_not_exists: true,
            drop_if_exists: true,
            index_concurrently: false,
            enum_style: EnumStyle::CheckConstraint,
            auto_increment_style: AutoIncrementStyle::Autoincrement,
            alter_add_column: true,
            alter_drop_column: true,
            alter_rename_column: true,
            alter_modify_column: false,
            alter_rename_table: true,
            transactional_ddl: true,
            drop_cascade: false,
        }
    }

    pub fn mssql() -> Self {
        Self {
            create_if_not_exists: false,
            drop_if_exists: true,
            index_concurrently: false,
            enum_style: EnumStyle::CheckConstraint,
            auto_increment_style: AutoIncrementStyle::Identity,
            alter_add_column: true,
            alter_drop_column: true,
            alter_rename_column: false, // Uses sp_rename
            alter_modify_column: true,
            alter_rename_table: false, // Uses sp_rename
            transactional_ddl: true,
            drop_cascade: false,
        }
    }

    pub fn oracle() -> Self {
        Self {
            create_if_not_exists: false,
            drop_if_exists: false,
            index_concurrently: false,
            enum_style: EnumStyle::CheckConstraint,
            auto_increment_style: AutoIncrementStyle::GeneratedIdentity,
            alter_add_column: true,
            alter_drop_column: true,
            alter_rename_column: true,
            alter_modify_column: true,
            alter_rename_table: true,
            transactional_ddl: false,
            drop_cascade: true,
        }
    }
}

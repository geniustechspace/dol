//! Definition operations — canonical representation of schema operations.

use super::EntityRef;
use crate::constraint::{EntityConstraint, FkAction, GeneratedKind};
use crate::types::DataType;

/// An owned model-level constraint for use in operations and builders (not `'static`).
///
/// This mirrors [`EntityConstraint`] but uses owned `String`/`Vec<String>`
/// instead of `&'static str`/`&'static [&'static str]`, enabling serde
/// round-tripping and runtime construction.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OwnedEntityConstraint {
    /// `UNIQUE (field1, field2, ...)`
    Unique(Vec<String>),
    /// `FOREIGN KEY (fields) REFERENCES ref_model (ref_fields) ON DELETE action`
    ForeignKey {
        columns: Vec<String>,
        ref_table: String,
        ref_columns: Vec<String>,
        on_delete: FkAction,
    },
    /// `CHECK (expression)`
    Check(String),
    /// `PRIMARY KEY (field1, field2, ...)` — composite primary key.
    PrimaryKey(Vec<String>),
}

impl From<&EntityConstraint> for OwnedEntityConstraint {
    fn from(c: &EntityConstraint) -> Self {
        match c {
            EntityConstraint::Unique(cols) => {
                Self::Unique(cols.iter().map(|s| (*s).to_string()).collect())
            }
            EntityConstraint::ForeignKey {
                columns,
                ref_table,
                ref_columns,
                on_delete,
            } => Self::ForeignKey {
                columns: columns.iter().map(|s| (*s).to_string()).collect(),
                ref_table: (*ref_table).to_string(),
                ref_columns: ref_columns.iter().map(|s| (*s).to_string()).collect(),
                on_delete: *on_delete,
            },
            EntityConstraint::Check(expr) => Self::Check((*expr).to_string()),
            EntityConstraint::PrimaryKey(cols) => {
                Self::PrimaryKey(cols.iter().map(|s| (*s).to_string()).collect())
            }
        }
    }
}

impl From<EntityConstraint> for OwnedEntityConstraint {
    fn from(c: EntityConstraint) -> Self {
        Self::from(&c)
    }
}

/// Preferred alias for [`OwnedEntityConstraint`].
pub type Constraint = OwnedEntityConstraint;

/// Preferred alias for [`OwnedForeignKeyRef`].
pub type ForeignKeyDef = OwnedForeignKeyRef;

/// Define (create) a new model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineEntity {
    pub name: String,
    pub namespace: Option<String>,
    pub fields: Vec<FieldDef>,
    pub constraints: Vec<OwnedEntityConstraint>,
    pub if_not_exists: bool,
}

/// An owned field definition for use in operations and builders (not `'static`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldDef {
    pub name: String,
    pub data_type: DataType,
    pub primary_key: bool,
    pub nullable: bool,
    pub default_expr: Option<String>,
    pub unique: bool,
    pub references: Option<OwnedForeignKeyRef>,
    pub check: Option<String>,
    pub comment: Option<String>,
    pub collation: Option<String>,
    pub generated: Option<(GeneratedKind, String)>,
    /// Hint that this field should be indexed.
    pub indexed: bool,
    /// Auto-incrementing field (replaces the old Serial/BigSerial types).
    pub auto_increment: bool,
}

impl FieldDef {
    pub fn new(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            primary_key: false,
            nullable: false,
            default_expr: None,
            unique: false,
            references: None,
            check: None,
            comment: None,
            collation: None,
            generated: None,
            indexed: false,
            auto_increment: false,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn optional(self) -> Self {
        self.nullable()
    }

    pub fn required(self) -> Self {
        self
    }

    pub fn default(mut self, expr: &str) -> Self {
        self.default_expr = Some(expr.to_string());
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn references(mut self, fk: OwnedForeignKeyRef) -> Self {
        self.references = Some(fk);
        self
    }

    pub fn check(mut self, expr: &str) -> Self {
        self.check = Some(expr.to_string());
        self
    }

    pub fn comment(mut self, text: &str) -> Self {
        self.comment = Some(text.to_string());
        self
    }

    pub fn collation(mut self, collation: &str) -> Self {
        self.collation = Some(collation.to_string());
        self
    }

    /// Hint that this field should be indexed.
    pub fn index(mut self) -> Self {
        self.indexed = true;
        self
    }

    pub fn generated_stored(mut self, expr: &str) -> Self {
        self.generated = Some((GeneratedKind::Stored, expr.to_string()));
        self
    }

    pub fn generated_virtual(mut self, expr: &str) -> Self {
        self.generated = Some((GeneratedKind::Virtual, expr.to_string()));
        self
    }

    /// Mark as auto-incrementing.
    pub fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }
}

/// An owned foreign key reference (not `'static`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OwnedForeignKeyRef {
    pub table: String,
    pub column: String,
    pub on_delete: FkAction,
    pub on_update: FkAction,
}

impl OwnedForeignKeyRef {
    pub fn new(table: &str, column: &str) -> Self {
        Self {
            table: table.to_string(),
            column: column.to_string(),
            on_delete: FkAction::NoAction,
            on_update: FkAction::NoAction,
        }
    }

    pub fn on_delete(mut self, action: FkAction) -> Self {
        self.on_delete = action;
        self
    }

    pub fn on_update(mut self, action: FkAction) -> Self {
        self.on_update = action;
        self
    }
}

/// Alter an existing model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AlterEntity {
    pub target: EntityRef,
    pub actions: Vec<AlterAction>,
}

/// A single alter action within an ALTER MODEL statement.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AlterAction {
    AddField(FieldDef),
    DropField(String),
    RenameField { from: String, to: String },
    AlterFieldType { name: String, new_type: DataType },
    SetFieldDefault { name: String, expr: String },
    DropFieldDefault(String),
    SetFieldNotNull(String),
    DropFieldNotNull(String),
    AddConstraint(OwnedEntityConstraint),
    DropConstraint(String),
    RenameEntity(String),
}

/// Drop (remove) a model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropEntity {
    pub target: EntityRef,
    pub if_exists: bool,
    pub cascade: bool,
}

/// Define (create) an index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineIndex {
    pub name: String,
    pub target: EntityRef,
    pub columns: Vec<String>,
    pub unique: bool,
    pub if_not_exists: bool,
    pub concurrently: bool,
    pub method: Option<IndexMethod>,
    pub where_clause: Option<String>,
}

/// Index access method — backend-agnostic.
///
/// Each variant expresses a *semantic* index capability. Backends map these
/// to their native access-method names (e.g. SQL/Postgres maps `FullText` →
/// `GIN`, `Spatial` → `GiST`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexMethod {
    /// Ordered B-tree index — universal (range queries, sorting).
    BTree,
    /// Hash index — equality lookups.
    Hash,
    /// Full-text search index (Postgres: GIN).
    FullText,
    /// Geospatial / range-type index (Postgres: GiST).
    Spatial,
    /// Backend-specific method not captured by the above variants.
    Custom(String),
}

/// Drop an index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropIndex {
    pub name: String,
    pub if_exists: bool,
    pub concurrently: bool,
    pub cascade: bool,
}

/// Define a custom type (e.g., enum).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineType {
    pub name: String,
    pub namespace: Option<String>,
    pub variants: Vec<String>,
}

/// Drop a custom type.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropType {
    pub name: String,
    pub if_exists: bool,
}

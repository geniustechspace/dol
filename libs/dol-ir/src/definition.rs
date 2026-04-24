//! Definition operations — DDL for schema creation and modification.
//!
//! These owned, runtime-friendly DDL types embed the schema-layer constraint
//! types (`dol_schema::EntityConstraint`, `dol_schema::ForeignKeyRef`)
//! directly — there is no longer a borrowed/owned mirror split.

use dol_schema::{EntityConstraint, ForeignKeyRef, GeneratedKind};
use dol_types::DataType;

use crate::entity_ref::EntityRef;

/// Define (create) a new entity / table.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineEntity {
    pub name: String,
    pub namespace: Option<String>,
    pub fields: Vec<FieldDef>,
    pub constraints: Vec<EntityConstraint>,
    pub if_not_exists: bool,
}

/// An owned field definition for use in DDL operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldDef {
    pub name: String,
    pub data_type: DataType,
    pub primary_key: bool,
    pub nullable: bool,
    pub default_expr: Option<String>,
    pub unique: bool,
    pub references: Option<ForeignKeyRef>,
    pub check: Option<String>,
    pub comment: Option<String>,
    pub collation: Option<String>,
    pub generated: Option<(GeneratedKind, String)>,
    pub indexed: bool,
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
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    pub fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }
    pub fn index(mut self) -> Self {
        self.indexed = true;
        self
    }
    pub fn default(mut self, expr: &str) -> Self {
        self.default_expr = Some(expr.to_string());
        self
    }
    pub fn comment(mut self, text: &str) -> Self {
        self.comment = Some(text.to_string());
        self
    }
    pub fn collation(mut self, c: &str) -> Self {
        self.collation = Some(c.to_string());
        self
    }
    pub fn check(mut self, expr: &str) -> Self {
        self.check = Some(expr.to_string());
        self
    }
    pub fn references(mut self, fk: ForeignKeyRef) -> Self {
        self.references = Some(fk);
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
}

/// Alter an existing entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AlterEntity {
    pub target: EntityRef,
    pub actions: Vec<AlterAction>,
}

/// A single action within an ALTER ENTITY statement.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    AddConstraint(EntityConstraint),
    DropConstraint(String),
    RenameEntity(String),
}

/// Drop (remove) an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropEntity {
    pub target: EntityRef,
    pub if_exists: bool,
    pub cascade: bool,
}

/// Define (create) an index.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexMethod {
    BTree,
    Hash,
    FullText,
    Spatial,
    Custom(String),
}

/// Drop an index.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropIndex {
    pub name: String,
    pub if_exists: bool,
    pub concurrently: bool,
    pub cascade: bool,
}

/// Define a custom type (e.g., an enum).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineType {
    pub name: String,
    pub namespace: Option<String>,
    pub variants: Vec<String>,
}

/// Drop a custom type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropType {
    pub name: String,
    pub if_exists: bool,
}

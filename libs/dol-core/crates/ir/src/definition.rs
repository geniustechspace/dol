//! Definition IR — canonical representation of schema operations.

use super::EntityRef;
use dol_entity::FieldType;
use dol_entity::constraint::{EntityConstraint, FkAction, GeneratedKind};

/// Define (create) a new model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DefineEntityIR {
    pub name: String,
    pub namespace: Option<String>,
    pub fields: Vec<FieldDef>,
    pub constraints: Vec<EntityConstraint>,
    pub if_not_exists: bool,
}

/// An owned field definition for use in IR and builders (not `'static`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
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
}

impl FieldDef {
    pub fn new(name: &str, field_type: FieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AlterEntityIR {
    pub target: EntityRef,
    pub actions: Vec<AlterAction>,
}

/// A single alter action within an ALTER MODEL statement.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AlterAction {
    AddField(FieldDef),
    DropField(String),
    RenameField { from: String, to: String },
    AlterFieldType { name: String, new_type: FieldType },
    SetFieldDefault { name: String, expr: String },
    DropFieldDefault(String),
    SetFieldNotNull(String),
    DropFieldNotNull(String),
    AddConstraint(EntityConstraint),
    DropConstraint(String),
    RenameEntity(String),
}

/// Drop (remove) a model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropEntityIR {
    pub target: EntityRef,
    pub if_exists: bool,
    pub cascade: bool,
}

/// Define (create) an index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineIndexIR {
    pub name: String,
    pub target: EntityRef,
    pub columns: Vec<String>,
    pub unique: bool,
    pub if_not_exists: bool,
    pub concurrently: bool,
    pub method: Option<IndexMethod>,
    pub where_clause: Option<String>,
}

/// Index method (B-tree, Hash, GIN, GiST, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexMethod {
    BTree,
    Hash,
    Gin,
    Gist,
    SpGist,
    Brin,
}

/// Drop an index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropIndexIR {
    pub name: String,
    pub if_exists: bool,
    pub concurrently: bool,
    pub cascade: bool,
}

/// Define a custom type (e.g., enum).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefineTypeIR {
    pub name: String,
    pub namespace: Option<String>,
    pub variants: Vec<String>,
}

/// Drop a custom type.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DropTypeIR {
    pub name: String,
    pub if_exists: bool,
}

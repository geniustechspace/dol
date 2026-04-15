//! Definition builders — CREATE, ALTER, DROP for models, indexes, and types.
//!
//! - [`CreateFromMeta`]: builds a `DefineEntityIR` from static [`Model`] metadata.
//! - [`DefineEntityBuilder`]: builds a CREATE TABLE from owned [`FieldDef`]s (runtime-defined).
//! - [`AlterEntityBuilder`]: builds ALTER TABLE statements from a Model reference.
//! - [`DropEntityBuilder`]: builds DROP TABLE from a Model reference.
//! - [`DefineIndexBuilder`]: builds CREATE INDEX.
//! - [`DropIndexBuilder`]: builds DROP INDEX.
//! - [`DefineTypeBuilder`]: builds CREATE TYPE (enum types).
//! - [`DropTypeBuilder`]: builds DROP TYPE.
//!
//! For SQL rendering, import the extension traits from `dol-sql`.

use dol_entity::constraint::EntityConstraint;
use dol_entity::{Entity, Field, FieldType};
use dol_ir::EntityRef;
use dol_ir::definition::{
    AlterAction, AlterEntityIR, DefineEntityIR, DefineIndexIR, DefineTypeIR, DropEntityIR,
    DropIndexIR, DropTypeIR, FieldDef, IndexMethod, OwnedForeignKeyRef,
};

// ---------------------------------------------------------------------------
// Field -> FieldDef conversion helper
// ---------------------------------------------------------------------------

/// Converts a static [`Field`] into an owned [`FieldDef`].
fn field_to_field_def(f: &Field) -> FieldDef {
    FieldDef {
        name: f.name.to_string(),
        field_type: f.field_type,
        primary_key: f.primary_key,
        nullable: f.nullable,
        default_expr: f.default_expr.map(|s| s.to_string()),
        unique: f.unique,
        references: f.references.as_ref().map(|fk| OwnedForeignKeyRef {
            table: fk.table.to_string(),
            column: fk.column.to_string(),
            on_delete: fk.on_delete,
            on_update: fk.on_update,
        }),
        check: f.check.map(|s| s.to_string()),
        comment: f.comment.map(|s| s.to_string()),
        collation: f.collation.map(|s| s.to_string()),
        generated: f.generated.map(|(k, e)| (k, e.to_string())),
        indexed: f.indexed,
    }
}

// ===========================================================================
// CreateFromMeta — renders CREATE TABLE from static Model metadata
// ===========================================================================

/// Builds a `DefineEntityIR` from a static [`Model`]'s metadata.
///
/// Converts the model's static field definitions to owned [`FieldDef`]s and
/// includes model-level constraints.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct CreateFromMeta<'a> {
    model: &'a Entity,
    if_not_exists: bool,
}

impl<'a> CreateFromMeta<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            if_not_exists: false,
        }
    }

    pub fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }

    /// Access the underlying model.
    pub fn get_entity(&self) -> &'a Entity {
        self.model
    }

    /// Whether IF NOT EXISTS was requested.
    pub fn has_if_not_exists(&self) -> bool {
        self.if_not_exists
    }

    /// Build the canonical [`DefineEntityIR`].
    ///
    /// Converts static `Field`s to owned `FieldDef`s and copies model
    /// constraints. Note: primary-key constraints derived from fields
    /// are included in the model's `constraints` array when present;
    /// otherwise renderers should extract PKs from `FieldDef::primary_key`.
    pub fn build(&self) -> DefineEntityIR {
        let fields = self.model.fields.iter().map(field_to_field_def).collect();
        DefineEntityIR {
            name: self.model.name.to_string(),
            namespace: self.model.namespace.map(|s| s.to_string()),
            fields,
            constraints: self.model.constraints.to_vec(),
            if_not_exists: self.if_not_exists,
        }
    }
}

// ===========================================================================
// DefineEntityBuilder — CREATE TABLE from owned FieldDefs
// ===========================================================================

/// Builds a `CREATE TABLE` from owned [`FieldDef`]s.
///
/// Use [`Entity::define`] as the entry point for runtime-defined tables,
/// or construct directly with [`DefineEntityBuilder::new`].
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DefineEntityBuilder {
    name: String,
    namespace: Option<String>,
    fields: Vec<FieldDef>,
    constraints: Vec<EntityConstraint>,
    if_not_exists: bool,
}

impl DefineEntityBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            namespace: None,
            fields: Vec::new(),
            constraints: Vec::new(),
            if_not_exists: false,
        }
    }

    pub fn namespace(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.to_string());
        self
    }

    pub fn field(mut self, fd: FieldDef) -> Self {
        self.fields.push(fd);
        self
    }

    pub fn fields(mut self, fds: Vec<FieldDef>) -> Self {
        self.fields.extend(fds);
        self
    }

    pub fn constraint(mut self, c: EntityConstraint) -> Self {
        self.constraints.push(c);
        self
    }

    pub fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }

    /// Build the canonical [`DefineEntityIR`].
    pub fn build(&self) -> DefineEntityIR {
        DefineEntityIR {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            fields: self.fields.clone(),
            constraints: self.constraints.clone(),
            if_not_exists: self.if_not_exists,
        }
    }
}

// ===========================================================================
// AlterEntityBuilder
// ===========================================================================

/// Builds `ALTER TABLE` statements from a [`Model`] reference.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct AlterEntityBuilder<'a> {
    model: &'a Entity,
    actions: Vec<AlterAction>,
}

impl<'a> AlterEntityBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            actions: Vec::new(),
        }
    }

    // -- Field operations --

    /// Add a column from an owned [`FieldDef`].
    pub fn add_field(mut self, fd: FieldDef) -> Self {
        self.actions.push(AlterAction::AddField(fd));
        self
    }

    /// Drop a column by name.
    pub fn drop_field(mut self, name: &str) -> Self {
        self.actions.push(AlterAction::DropField(name.to_string()));
        self
    }

    /// Rename a column.
    pub fn rename_field(mut self, from: &str, to: &str) -> Self {
        self.actions.push(AlterAction::RenameField {
            from: from.to_string(),
            to: to.to_string(),
        });
        self
    }

    /// Change a column's type to the given [`FieldType`].
    pub fn alter_column_type(mut self, name: &str, new_type: FieldType) -> Self {
        self.actions.push(AlterAction::AlterFieldType {
            name: name.to_string(),
            new_type,
        });
        self
    }

    // -- Default operations --

    /// Set a column's DEFAULT expression.
    pub fn set_default(mut self, name: &str, expr: &str) -> Self {
        self.actions.push(AlterAction::SetFieldDefault {
            name: name.to_string(),
            expr: expr.to_string(),
        });
        self
    }

    /// Drop a column's DEFAULT.
    pub fn drop_default(mut self, name: &str) -> Self {
        self.actions
            .push(AlterAction::DropFieldDefault(name.to_string()));
        self
    }

    // -- Nullability operations --

    /// Set a column to NOT NULL.
    pub fn set_not_null(mut self, name: &str) -> Self {
        self.actions
            .push(AlterAction::SetFieldNotNull(name.to_string()));
        self
    }

    /// Drop the NOT NULL constraint (allow NULL).
    pub fn drop_not_null(mut self, name: &str) -> Self {
        self.actions
            .push(AlterAction::DropFieldNotNull(name.to_string()));
        self
    }

    // -- Constraint operations --

    /// Add a model-level constraint.
    pub fn add_constraint(mut self, c: EntityConstraint) -> Self {
        self.actions.push(AlterAction::AddConstraint(c));
        self
    }

    /// Drop a named constraint.
    pub fn drop_constraint(mut self, name: &str) -> Self {
        self.actions
            .push(AlterAction::DropConstraint(name.to_string()));
        self
    }

    // -- Rename --

    /// Rename the model (table).
    pub fn rename_model(mut self, new_name: &str) -> Self {
        self.actions
            .push(AlterAction::RenameEntity(new_name.to_string()));
        self
    }

    // -- Build / render --

    /// Build the canonical [`AlterEntityIR`].
    pub fn build(&self) -> AlterEntityIR {
        AlterEntityIR {
            target: EntityRef {
                name: self.model.name.to_string(),
                namespace: self.model.namespace.map(|s| s.to_string()),
                alias: None,
            },
            actions: self.actions.clone(),
        }
    }
}

// ===========================================================================
// DropEntityBuilder
// ===========================================================================

/// Builds a `DROP TABLE` from a [`Model`] reference.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DropEntityBuilder<'a> {
    model: &'a Entity,
    if_exists: bool,
    cascade: bool,
}

impl<'a> DropEntityBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            if_exists: false,
            cascade: false,
        }
    }

    pub fn if_exists(mut self) -> Self {
        self.if_exists = true;
        self
    }

    pub fn cascade(mut self) -> Self {
        self.cascade = true;
        self
    }

    /// Build the canonical [`DropEntityIR`].
    pub fn build(&self) -> DropEntityIR {
        DropEntityIR {
            target: EntityRef {
                name: self.model.name.to_string(),
                namespace: self.model.namespace.map(|s| s.to_string()),
                alias: None,
            },
            if_exists: self.if_exists,
            cascade: self.cascade,
        }
    }
}

// ===========================================================================
// DefineIndexBuilder
// ===========================================================================

/// Builds a `CREATE INDEX` statement.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DefineIndexBuilder {
    name: String,
    target_name: String,
    target_namespace: Option<String>,
    columns: Vec<String>,
    unique: bool,
    if_not_exists: bool,
    concurrently: bool,
    method: Option<IndexMethod>,
    where_clause: Option<String>,
}

impl DefineIndexBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            target_name: String::new(),
            target_namespace: None,
            columns: Vec::new(),
            unique: false,
            if_not_exists: false,
            concurrently: false,
            method: None,
            where_clause: None,
        }
    }

    /// Set the target model (table) name.
    pub fn on(mut self, model_name: &str) -> Self {
        self.target_name = model_name.to_string();
        self
    }

    /// Set the target model's namespace (schema).
    pub fn namespace(mut self, ns: &str) -> Self {
        self.target_namespace = Some(ns.to_string());
        self
    }

    /// Add multiple columns to the index.
    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.columns.extend(cols.iter().map(|c| c.to_string()));
        self
    }

    /// Add a single column to the index.
    pub fn column(mut self, col: &str) -> Self {
        self.columns.push(col.to_string());
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }

    pub fn concurrently(mut self) -> Self {
        self.concurrently = true;
        self
    }

    pub fn method(mut self, method: IndexMethod) -> Self {
        self.method = Some(method);
        self
    }

    /// Add a partial-index WHERE clause.
    pub fn where_clause(mut self, expr: &str) -> Self {
        self.where_clause = Some(expr.to_string());
        self
    }

    /// Build the canonical [`DefineIndexIR`].
    pub fn build(&self) -> DefineIndexIR {
        DefineIndexIR {
            name: self.name.clone(),
            target: EntityRef {
                name: self.target_name.clone(),
                namespace: self.target_namespace.clone(),
                alias: None,
            },
            columns: self.columns.clone(),
            unique: self.unique,
            if_not_exists: self.if_not_exists,
            concurrently: self.concurrently,
            method: self.method,
            where_clause: self.where_clause.clone(),
        }
    }
}

// ===========================================================================
// DropIndexBuilder
// ===========================================================================

/// Builds a `DROP INDEX` statement.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DropIndexBuilder {
    name: String,
    if_exists: bool,
    concurrently: bool,
    cascade: bool,
}

impl DropIndexBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            if_exists: false,
            concurrently: false,
            cascade: false,
        }
    }

    pub fn if_exists(mut self) -> Self {
        self.if_exists = true;
        self
    }

    pub fn concurrently(mut self) -> Self {
        self.concurrently = true;
        self
    }

    pub fn cascade(mut self) -> Self {
        self.cascade = true;
        self
    }

    /// Build the canonical [`DropIndexIR`].
    pub fn build(&self) -> DropIndexIR {
        DropIndexIR {
            name: self.name.clone(),
            if_exists: self.if_exists,
            concurrently: self.concurrently,
            cascade: self.cascade,
        }
    }
}

// ===========================================================================
// Entity::define — static entry point for runtime-defined models
// ===========================================================================

/// Extension trait providing the `define()` associated function on [`Model`].
pub trait EntityDefineExt {
    /// Start building a `CREATE TABLE` from scratch with owned field definitions.
    ///
    /// Unlike [`Entity::create`](super::EntityBuilderExt::create) which builds from static metadata,
    /// `define` produces a [`DefineEntityBuilder`] for runtime-constructed schemas.
    fn define(name: &str) -> DefineEntityBuilder;
}

impl EntityDefineExt for Entity {
    fn define(name: &str) -> DefineEntityBuilder {
        DefineEntityBuilder::new(name)
    }
}

// ===========================================================================
// DefineTypeBuilder — CREATE TYPE (enum)
// ===========================================================================

/// Builds a `CREATE TYPE` statement for custom enum types.
///
/// ```rust
/// use dol_builder::definition::DefineTypeBuilder;
///
/// let ir = DefineTypeBuilder::new("order_status")
///     .variant("pending")
///     .variant("shipped")
///     .variant("delivered")
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DefineTypeBuilder {
    name: String,
    namespace: Option<String>,
    variants: Vec<String>,
}

impl DefineTypeBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            namespace: None,
            variants: Vec::new(),
        }
    }

    /// Set the namespace (schema) for this type.
    pub fn namespace(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.to_string());
        self
    }

    /// Add a single enum variant.
    pub fn variant(mut self, name: &str) -> Self {
        self.variants.push(name.to_string());
        self
    }

    /// Add multiple enum variants at once.
    pub fn variants(mut self, names: &[&str]) -> Self {
        self.variants.extend(names.iter().map(|s| s.to_string()));
        self
    }

    /// Build the canonical [`DefineTypeIR`].
    pub fn build(&self) -> DefineTypeIR {
        DefineTypeIR {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            variants: self.variants.clone(),
        }
    }
}

// ===========================================================================
// DropTypeBuilder — DROP TYPE
// ===========================================================================

/// Builds a `DROP TYPE` statement.
///
/// ```rust
/// use dol_builder::definition::DropTypeBuilder;
///
/// let ir = DropTypeBuilder::new("order_status")
///     .if_exists()
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DropTypeBuilder {
    name: String,
    if_exists: bool,
}

impl DropTypeBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            if_exists: false,
        }
    }

    pub fn if_exists(mut self) -> Self {
        self.if_exists = true;
        self
    }

    /// Build the canonical [`DropTypeIR`].
    pub fn build(&self) -> DropTypeIR {
        DropTypeIR {
            name: self.name.clone(),
            if_exists: self.if_exists,
        }
    }
}

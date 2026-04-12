//! Definition builders — CREATE, ALTER, DROP for models and indexes.
//!
//! - [`CreateFromMeta`]: builds a `DefineModelIR` from static [`Model`] metadata.
//! - [`DefineModelBuilder`]: builds a CREATE TABLE from owned [`FieldDef`]s (runtime-defined).
//! - [`AlterModelBuilder`]: builds ALTER TABLE statements from a Model reference.
//! - [`DropModelBuilder`]: builds DROP TABLE from a Model reference.
//! - [`DefineIndexBuilder`]: builds CREATE INDEX.
//! - [`DropIndexBuilder`]: builds DROP INDEX.
//!
//! For SQL rendering, import the extension traits from `dol-sql`.

use dol_ir::definition::{
    AlterAction, AlterModelIR, DefineIndexIR, DefineModelIR, DropIndexIR, DropModelIR, FieldDef,
    IndexMethod, OwnedForeignKeyRef,
};
use dol_ir::ModelRef;
use dol_model::constraint::ModelConstraint;
use dol_model::{Field, FieldType, Model};

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

/// Builds a `DefineModelIR` from a static [`Model`]'s metadata.
///
/// Converts the model's static field definitions to owned [`FieldDef`]s and
/// includes model-level constraints.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct CreateFromMeta<'a> {
    model: &'a Model,
    if_not_exists: bool,
}

impl<'a> CreateFromMeta<'a> {
    pub fn new(model: &'a Model) -> Self {
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
    pub fn get_model(&self) -> &'a Model {
        self.model
    }

    /// Whether IF NOT EXISTS was requested.
    pub fn has_if_not_exists(&self) -> bool {
        self.if_not_exists
    }

    /// Build the canonical [`DefineModelIR`].
    ///
    /// Converts static `Field`s to owned `FieldDef`s and copies model
    /// constraints. Note: primary-key constraints derived from fields
    /// are included in the model's `constraints` array when present;
    /// otherwise renderers should extract PKs from `FieldDef::primary_key`.
    pub fn build(&self) -> DefineModelIR {
        let fields = self.model.fields.iter().map(field_to_field_def).collect();
        DefineModelIR {
            name: self.model.name.to_string(),
            namespace: self.model.namespace.map(|s| s.to_string()),
            fields,
            constraints: self.model.constraints.to_vec(),
            if_not_exists: self.if_not_exists,
        }
    }
}

// ===========================================================================
// DefineModelBuilder — CREATE TABLE from owned FieldDefs
// ===========================================================================

/// Builds a `CREATE TABLE` from owned [`FieldDef`]s.
///
/// Use [`Model::define`] as the entry point for runtime-defined tables,
/// or construct directly with [`DefineModelBuilder::new`].
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DefineModelBuilder {
    name: String,
    namespace: Option<String>,
    fields: Vec<FieldDef>,
    constraints: Vec<ModelConstraint>,
    if_not_exists: bool,
}

impl DefineModelBuilder {
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

    /// Alias for `namespace` — backward compat with SQL schema terminology.
    pub fn schema(self, schema: &str) -> Self {
        self.namespace(schema)
    }

    pub fn field(mut self, fd: FieldDef) -> Self {
        self.fields.push(fd);
        self
    }

    pub fn fields(mut self, fds: Vec<FieldDef>) -> Self {
        self.fields.extend(fds);
        self
    }

    pub fn constraint(mut self, c: ModelConstraint) -> Self {
        self.constraints.push(c);
        self
    }

    pub fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }

    /// Build the canonical [`DefineModelIR`].
    pub fn build(&self) -> DefineModelIR {
        DefineModelIR {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            fields: self.fields.clone(),
            constraints: self.constraints.clone(),
            if_not_exists: self.if_not_exists,
        }
    }
}

// ===========================================================================
// AlterModelBuilder
// ===========================================================================

/// Builds `ALTER TABLE` statements from a [`Model`] reference.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct AlterModelBuilder<'a> {
    model: &'a Model,
    actions: Vec<AlterAction>,
}

impl<'a> AlterModelBuilder<'a> {
    pub fn new(model: &'a Model) -> Self {
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

    /// Backward-compat: add a column from a static [`Field`], converting to
    /// an owned [`FieldDef`] internally.
    pub fn add_column(self, col: Field) -> Self {
        self.add_field(field_to_field_def(&col))
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
    pub fn add_constraint(mut self, c: ModelConstraint) -> Self {
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
            .push(AlterAction::RenameModel(new_name.to_string()));
        self
    }

    // -- Build / render --

    /// Build the canonical [`AlterModelIR`].
    pub fn build(&self) -> AlterModelIR {
        AlterModelIR {
            target: ModelRef {
                name: self.model.name.to_string(),
                namespace: self.model.namespace.map(|s| s.to_string()),
                alias: None,
            },
            actions: self.actions.clone(),
        }
    }
}

// ===========================================================================
// DropModelBuilder
// ===========================================================================

/// Builds a `DROP TABLE` from a [`Model`] reference.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DropModelBuilder<'a> {
    model: &'a Model,
    if_exists: bool,
    cascade: bool,
}

impl<'a> DropModelBuilder<'a> {
    pub fn new(model: &'a Model) -> Self {
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

    /// Build the canonical [`DropModelIR`].
    pub fn build(&self) -> DropModelIR {
        DropModelIR {
            target: ModelRef {
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
            target: ModelRef {
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
// Model::define — static entry point for runtime-defined models
// ===========================================================================

/// Extension trait providing the `define()` associated function on [`Model`].
pub trait ModelDefineExt {
    /// Start building a `CREATE TABLE` from scratch with owned field definitions.
    ///
    /// Unlike [`Model::create`](super::ModelBuilderExt::create) which builds from static metadata,
    /// `define` produces a [`DefineModelBuilder`] for runtime-constructed schemas.
    fn define(name: &str) -> DefineModelBuilder;
}

impl ModelDefineExt for Model {
    fn define(name: &str) -> DefineModelBuilder {
        DefineModelBuilder::new(name)
    }
}

//! # dol-query — Backend-Neutral Query Entry Point
//!
//! A standalone, publishable crate that provides a universal query entry point
//! for DOL. Unlike the low-level `dol-builder` crate (which requires a static
//! `&Entity` reference), `dol-query` accepts **both** `Entity` references and
//! plain entity-name strings.
//!
//! This makes it suitable for dynamic/runtime scenarios (e.g. REST APIs,
//! configuration-driven pipelines) where the entity name is only known at
//! runtime and no static schema definition exists.
//!
//! # Quick Start
//!
//! ```rust
//! use dol_query::Query;
//! use dol_schema::{Entity, Field, DataType};
//! use dol_expr::tree::{field, param};
//!
//! // From an Entity — full field-aware API
//! let users = Entity::new("users", vec![
//!     Field::new("id", DataType::Uuid).identity(),
//!     Field::new("email", DataType::unbounded_string()),
//! ]);
//!
//! let (stmt, _arena, interner) = Query::from(&users)
//!     .get()
//!     .filter(field("id").eq(param()))
//!     .build();
//! match stmt {
//!     dol_ir::Statement::Query(q) => {
//!         assert_eq!(interner.get(q.from), "users");
//!         assert_eq!(q.columns.len(), 2);
//!     }
//!     _ => panic!("expected Query"),
//! }
//!
//! // From a plain string — no field metadata needed
//! let (stmt, _arena, interner) = Query::from("users")
//!     .get()
//!     .fields(&["id", "email"])
//!     .filter(field("id").eq(param()))
//!     .build();
//! match stmt {
//!     dol_ir::Statement::Query(q) => {
//!         assert_eq!(interner.get(q.from), "users");
//!     }
//!     _ => panic!("expected Query"),
//! }
//!
//! // From a namespaced string
//! let (stmt, _arena, interner) = Query::from("identity.users")
//!     .get()
//!     .fields(&["id"])
//!     .build();
//! match stmt {
//!     dol_ir::Statement::Query(q) => {
//!         assert_eq!(interner.get(q.from), "identity.users");
//!     }
//!     _ => panic!("expected Query"),
//! }
//!
//! // Namespace chaining — builds hierarchical paths
//! let (stmt, _arena, interner) = Query::from("api")
//!     .namespace("v1")
//!     .namespace("users")
//!     .get()
//!     .fields(&["id"])
//!     .build();
//! match stmt {
//!     dol_ir::Statement::Query(q) => {
//!         assert_eq!(interner.get(q.from), "api.v1.users");
//!     }
//!     _ => panic!("expected Query"),
//! }
//! ```
//!
//! # Backend Neutrality
//!
//! `dol-query` produces backend-agnostic IR types (`Statement`, `Query`, etc.)
//! from `dol-ir`. These can be rendered by **any** backend — SQL, key-value,
//! file system, API, or custom engines.

#![deny(unsafe_code)]

mod delete;
mod get;
mod insert;
mod update;
mod upsert;

pub mod builder;
pub mod ddl;
pub mod prelude;

pub use ddl::{
    AlterEntityBuilder, CreateFromMeta, DefineEntityBuilder, DefineLookupBuilder,
    DefineTypeBuilder, DropEntityBuilder, DropLookupBuilder, DropTypeBuilder, EntityDefineExt,
};

pub use delete::DeleteQuery;
#[allow(deprecated)]
pub use delete::RemoveQuery;
pub use get::GetQuery;
pub use insert::InsertQuery;
pub use update::UpdateQuery;
pub use upsert::UpsertQuery;

use dol_schema::Entity;

// ---------------------------------------------------------------------------
// Query — the universal entry point
// ---------------------------------------------------------------------------

/// Backend-neutral query entry point.
///
/// Construct via `Query::from(&entity)` or `Query::from("entity_name")`.
/// Then call `.get()`, `.insert()`, `.update()`, `.delete()`, or `.upsert()`
/// to begin building a specific operation.
///
/// # Namespace chaining
///
/// Use `.namespace()` to append hierarchical segments. Each call pushes
/// the current name into the namespace prefix and replaces the name with
/// the new segment:
///
/// ```rust
/// use dol_query::Query;
///
/// // Single namespace
/// let (stmt, _arena, interner) = Query::from("api")
///     .namespace("users")
///     .get()
///     .fields(&["id"])
///     .build();
/// match stmt {
///     dol_ir::Statement::Query(q) => {
///         assert_eq!(interner.get(q.from), "api.users");
///     }
///     _ => panic!("expected Query"),
/// }
///
/// // Chained namespaces — builds "api.v1.users"
/// let (stmt, _arena, interner) = Query::from("api")
///     .namespace("v1")
///     .namespace("users")
///     .get()
///     .fields(&["id"])
///     .build();
/// match stmt {
///     dol_ir::Statement::Query(q) => {
///         assert_eq!(interner.get(q.from), "api.v1.users");
///     }
///     _ => panic!("expected Query"),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Query {
    name: String,
    namespace: Option<String>,
    field_names: Option<Vec<String>>,
}

impl Query {
    /// Append a namespace segment.
    ///
    /// Pushes the current `name` into the namespace prefix and sets `name`
    /// to the new segment. Chaining multiple calls builds a hierarchical
    /// path — useful for API endpoints and multi-level schemas:
    ///
    /// ```text
    /// Query::from("api").namespace("v1").namespace("users")
    ///   → namespace = "api.v1", name = "users"
    ///   → SQL: api.v1.users   API: /api/v1/users
    /// ```
    pub fn namespace(mut self, segment: &str) -> Self {
        // Push current name into the namespace prefix.
        self.namespace = Some(match self.namespace.take() {
            Some(ns) => format!("{}.{}", ns, self.name),
            None => self.name.clone(),
        });
        self.name = segment.to_string();
        // Field metadata is no longer valid after changing the target entity.
        self.field_names = None;
        self
    }

    /// Start building a GET (SELECT) query.
    pub fn get(self) -> GetQuery {
        GetQuery::new(self.name, self.namespace, self.field_names)
    }

    /// Start building an INSERT statement.
    pub fn insert(self) -> InsertQuery {
        InsertQuery::new(self.name, self.namespace, self.field_names)
    }

    /// Start building an UPDATE statement.
    pub fn update(self) -> UpdateQuery {
        UpdateQuery::new(self.name, self.namespace)
    }

    /// Start building a DELETE statement.
    pub fn delete(self) -> DeleteQuery {
        DeleteQuery::new(self.name, self.namespace)
    }

    /// Deprecated: use [`delete()`](Self::delete) instead.
    #[deprecated(note = "use `delete()`")]
    pub fn remove(self) -> DeleteQuery {
        DeleteQuery::new(self.name, self.namespace)
    }

    /// Start building an upsert statement.
    pub fn upsert(self) -> UpsertQuery {
        UpsertQuery::new(self.name, self.namespace, self.field_names)
    }
}

// ── From<&Entity> ──────────────────────────────────────────────────────

impl From<&Entity> for Query {
    fn from(entity: &Entity) -> Self {
        Self {
            name: entity.name.to_string(),
            namespace: entity.namespace.as_ref().map(|s| s.to_string()),
            field_names: Some(entity.field_names().map(|s| s.to_string()).collect()),
        }
    }
}

// ── From<&str> ─────────────────────────────────────────────────────────

impl From<&str> for Query {
    /// Parse an entity name string.
    ///
    /// Supports plain names (`"users"`) and dot-separated namespaced names
    /// (`"identity.users"` → namespace `"identity"`, name `"users"`).
    fn from(s: &str) -> Self {
        let (namespace, name) = match s.rsplit_once('.') {
            Some((ns, n)) => (Some(ns.to_string()), n.to_string()),
            None => (None, s.to_string()),
        };
        Self {
            name,
            namespace,
            field_names: None,
        }
    }
}

// ── From<String> ───────────────────────────────────────────────────────

impl From<String> for Query {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ---------------------------------------------------------------------------
// JoinKind — locally defined
// ---------------------------------------------------------------------------

/// The kind of JOIN to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests;

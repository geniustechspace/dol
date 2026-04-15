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
//! use dol_entity::{Entity, Field, FieldType};
//!
//! // From a static Entity — full field-aware API
//! static USERS: Entity = Entity::new("users", &[
//!     Field::new("id", FieldType::Uuid).primary_key(),
//!     Field::new("email", FieldType::Text),
//! ]);
//!
//! let ir = Query::from(&USERS)
//!     .get()
//!     .where_eq("id")
//!     .build();
//! assert_eq!(ir.source.name, "users");
//! assert_eq!(ir.projections.len(), 2);
//!
//! // From a plain string — no field metadata needed
//! let ir = Query::from("users")
//!     .get()
//!     .columns(&["id", "email"])
//!     .where_eq("id")
//!     .build();
//! assert_eq!(ir.source.name, "users");
//!
//! // From a namespaced string
//! let ir = Query::from("identity.users")
//!     .get()
//!     .columns(&["id"])
//!     .build();
//! assert_eq!(ir.source.name, "users");
//! assert_eq!(ir.source.namespace.as_deref(), Some("identity"));
//!
//! // Namespace chaining — builds hierarchical paths
//! let ir = Query::from("api")
//!     .namespace("v1")
//!     .namespace("users")
//!     .get()
//!     .columns(&["id"])
//!     .build();
//! assert_eq!(ir.source.namespace.as_deref(), Some("api.v1"));
//! assert_eq!(ir.source.name, "users");
//! ```
//!
//! # Backend Neutrality
//!
//! `dol-query` produces backend-agnostic IR types (`QueryIR`, `InsertIR`, etc.)
//! from `dol-ir`. These can be rendered by **any** backend — SQL, key-value,
//! file system, API, or custom engines.

#![deny(unsafe_code)]

mod get;
mod insert;
mod remove;
mod update;
mod upsert;

pub use get::GetQuery;
pub use insert::InsertQuery;
pub use remove::RemoveQuery;
pub use update::UpdateQuery;
pub use upsert::UpsertQuery;

use dol_entity::Entity;

// ---------------------------------------------------------------------------
// Query — the universal entry point
// ---------------------------------------------------------------------------

/// Backend-neutral query entry point.
///
/// Construct via `Query::from(&entity)` or `Query::from("entity_name")`.
/// Then call `.get()`, `.insert()`, `.update()`, `.remove()`, or `.upsert()`
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
/// let ir = Query::from("api")
///     .namespace("users")
///     .get()
///     .columns(&["id"])
///     .build();
/// assert_eq!(ir.source.namespace.as_deref(), Some("api"));
/// assert_eq!(ir.source.name, "users");
///
/// // Chained namespaces — builds "api.v1.users"
/// let ir = Query::from("api")
///     .namespace("v1")
///     .namespace("users")
///     .get()
///     .columns(&["id"])
///     .build();
/// assert_eq!(ir.source.namespace.as_deref(), Some("api.v1"));
/// assert_eq!(ir.source.name, "users");
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

    /// Start building a REMOVE (DELETE) statement.
    pub fn remove(self) -> RemoveQuery {
        RemoveQuery::new(self.name, self.namespace)
    }

    /// Start building an UPSERT (INSERT ... ON CONFLICT) statement.
    pub fn upsert(self) -> UpsertQuery {
        UpsertQuery::new(self.name, self.namespace, self.field_names)
    }
}

// ── From<&Entity> ──────────────────────────────────────────────────────

impl From<&Entity> for Query {
    fn from(entity: &Entity) -> Self {
        Self {
            name: entity.name.to_string(),
            namespace: entity.namespace.map(|s| s.to_string()),
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dol_entity::{Field, FieldType};
    use dol_expr::Expr;

    static USERS: Entity = Entity::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("email", FieldType::Text).unique(),
            Field::new("name", FieldType::Text),
        ],
    );

    // ── Query construction ──────────────────────────────────────────

    #[test]
    fn from_entity() {
        let q = Query::from(&USERS);
        assert_eq!(q.name, "users");
        assert!(q.namespace.is_none());
        assert_eq!(
            q.field_names.as_deref(),
            Some(["id", "email", "name"].map(String::from).as_slice())
        );
    }

    #[test]
    fn from_plain_string() {
        let q = Query::from("users");
        assert_eq!(q.name, "users");
        assert!(q.namespace.is_none());
        assert!(q.field_names.is_none());
    }

    #[test]
    fn from_namespaced_string() {
        let q = Query::from("identity.users");
        assert_eq!(q.name, "users");
        assert_eq!(q.namespace.as_deref(), Some("identity"));
        assert!(q.field_names.is_none());
    }

    #[test]
    fn from_owned_string() {
        let q = Query::from(String::from("identity.users"));
        assert_eq!(q.name, "users");
        assert_eq!(q.namespace.as_deref(), Some("identity"));
    }

    // ── Namespace chaining ──────────────────────────────────────────

    #[test]
    fn namespace_single_segment() {
        let q = Query::from("api").namespace("users");
        assert_eq!(q.name, "users");
        assert_eq!(q.namespace.as_deref(), Some("api"));
    }

    #[test]
    fn namespace_chained_segments() {
        let q = Query::from("api").namespace("v1").namespace("users");
        assert_eq!(q.name, "users");
        assert_eq!(q.namespace.as_deref(), Some("api.v1"));
    }

    #[test]
    fn namespace_three_levels() {
        let q = Query::from("api")
            .namespace("v1")
            .namespace("admin")
            .namespace("users");
        assert_eq!(q.name, "users");
        assert_eq!(q.namespace.as_deref(), Some("api.v1.admin"));
    }

    #[test]
    fn namespace_on_already_namespaced_string() {
        // "identity.users" → namespace="identity", name="users"
        // .namespace("profiles") → namespace="identity.users", name="profiles"
        let q = Query::from("identity.users").namespace("profiles");
        assert_eq!(q.name, "profiles");
        assert_eq!(q.namespace.as_deref(), Some("identity.users"));
    }

    #[test]
    fn namespace_propagates_to_get_ir() {
        let ir = Query::from("api")
            .namespace("v1")
            .namespace("users")
            .get()
            .columns(&["id"])
            .build();
        assert_eq!(ir.source.name, "users");
        assert_eq!(ir.source.namespace.as_deref(), Some("api.v1"));
    }

    #[test]
    fn namespace_propagates_to_insert_ir() {
        let ir = Query::from("api")
            .namespace("users")
            .insert()
            .columns(&["id", "email"])
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.target.namespace.as_deref(), Some("api"));
    }

    #[test]
    fn namespace_propagates_to_update_ir() {
        let ir = Query::from("api")
            .namespace("users")
            .update()
            .set("email")
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.target.namespace.as_deref(), Some("api"));
    }

    #[test]
    fn namespace_propagates_to_remove_ir() {
        let ir = Query::from("api")
            .namespace("users")
            .remove()
            .where_eq("id")
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.target.namespace.as_deref(), Some("api"));
    }

    #[test]
    fn namespace_propagates_to_upsert_ir() {
        let ir = Query::from("api")
            .namespace("users")
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_nothing()
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.target.namespace.as_deref(), Some("api"));
    }

    // ── GetQuery from Entity ────────────────────────────────────────

    #[test]
    fn get_from_entity_defaults() {
        let ir = Query::from(&USERS).get().build();
        assert_eq!(ir.source.name, "users");
        assert_eq!(ir.projections.len(), 3);
        assert!(matches!(&ir.projections[0], Expr::Identifier(n) if n == "id"));
        assert!(matches!(&ir.projections[2], Expr::Identifier(n) if n == "name"));
    }

    #[test]
    fn get_from_entity_where_eq() {
        let ir = Query::from(&USERS).get().where_eq("id").build();
        assert_eq!(ir.filters.len(), 1);
    }

    // ── GetQuery from string ────────────────────────────────────────

    #[test]
    fn get_from_string_columns() {
        let ir = Query::from("users")
            .get()
            .columns(&["id", "email"])
            .where_eq("id")
            .build();
        assert_eq!(ir.source.name, "users");
        assert!(ir.source.namespace.is_none());
        assert_eq!(ir.projections.len(), 2);
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn get_from_namespaced_string() {
        let ir = Query::from("identity.users").get().columns(&["id"]).build();
        assert_eq!(ir.source.name, "users");
        assert_eq!(ir.source.namespace.as_deref(), Some("identity"));
    }

    // ── InsertQuery ─────────────────────────────────────────────────

    #[test]
    fn insert_from_entity_defaults() {
        let ir = Query::from(&USERS).insert().build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.fields, ["id", "email", "name"]);
    }

    #[test]
    fn insert_from_string_columns() {
        let ir = Query::from("users")
            .insert()
            .columns(&["id", "email"])
            .rows(2)
            .returning_all()
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.fields, ["id", "email"]);
        assert_eq!(ir.row_count, 2);
        assert_eq!(ir.returning, ["*"]);
    }

    // ── UpdateQuery ─────────────────────────────────────────────────

    #[test]
    fn update_from_string() {
        let ir = Query::from("users")
            .update()
            .set("email")
            .where_eq("id")
            .returning_all()
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.assignments.len(), 1);
        assert_eq!(ir.filters.len(), 1);
        assert_eq!(ir.returning, ["*"]);
    }

    // ── RemoveQuery ─────────────────────────────────────────────────

    #[test]
    fn remove_from_string() {
        let ir = Query::from("users")
            .remove()
            .where_eq("id")
            .returning_all()
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.filters.len(), 1);
        assert_eq!(ir.returning, ["*"]);
    }

    // ── UpsertQuery ─────────────────────────────────────────────────

    #[test]
    fn upsert_from_string() {
        let ir = Query::from("users")
            .upsert()
            .columns(&["id", "email", "name"])
            .on_conflict(&["id"])
            .do_update(&["email", "name"])
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.fields, ["id", "email", "name"]);
        assert_eq!(ir.conflict_fields, ["id"]);
        assert_eq!(ir.update_fields, ["email", "name"]);
    }

    #[test]
    fn upsert_do_nothing() {
        let ir = Query::from("users")
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_nothing()
            .build();
        assert!(ir.do_nothing);
        assert!(ir.update_fields.is_empty());
    }
}

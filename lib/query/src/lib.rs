//! # dol-query — Backend-Neutral Query Entry Point
//!
//! A standalone, publishable crate that provides a universal query entry point
//! for DOL. Builders accept an [`Entity`] reference *or*
//! a plain entity-name string; runtime-known names are first-class.
//!
//! Every `.try_build()` returns a [`dol_command::program::Program`] containing one or more
//! [`dol_command::operation::Operation`]s.
//!
//! # Quick Start
//!
//! ```
//! use dol_query::Query;
//! use dol_expr::tree::{field, param};
//!
//! # #[cfg(feature = "schema")]
//! # {
//! use dol_schema::{DataType, Entity, Field};
//!
//! let users = Entity::new("users", vec![
//!     Field::new("id", DataType::Uuid).identity(),
//!     Field::new("email", DataType::unbounded_string()),
//! ]);
//!
//! // From an Entity — full field-aware API (requires the `schema` feature).
//! let program = Query::from(&users)
//!     .get()
//!     .filter(field("id").eq(param()))
//!     .try_build()
//!     .expect("doc example: trivial filter must lower");
//! assert_eq!(program.operations[0].kind(), dol_command::operation::OpKind::Query);
//! # }
//!
//! // From a plain string — no field metadata or `schema` feature needed.
//! let program = Query::from("users")
//!     .get()
//!     .fields(&["id", "email"])
//!     .try_build()
//!     .expect("doc example: trivial projection must lower");
//! assert_eq!(program.operations[0].kind(), dol_command::operation::OpKind::Query);
//! ```

#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![warn(missing_docs)]

extern crate alloc;

mod delete;
mod error;
mod get;
mod insert;
pub mod pipeline;
pub mod prelude;
pub mod stream;
mod update;
mod upsert;

pub use delete::DeleteQuery;
pub use error::BuildError;
pub use get::GetQuery;
pub use insert::InsertQuery;
pub use update::UpdateQuery;
pub use upsert::UpsertQuery;

#[cfg(feature = "schema")]
use dol_schema::Entity;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Query — the universal entry point
// ---------------------------------------------------------------------------

/// Backend-neutral query entry point.
///
/// Construct via `Query::from(&entity)` or `Query::from("entity_name")`.
/// Then call `.get()`, `.insert()`, `.update()`, `.delete()`, or `.upsert()`
/// to begin building a specific operation.
#[derive(Debug, Clone)]
pub struct Query {
    pub(crate) name: String,
    pub(crate) namespace: Option<String>,
    pub(crate) field_names: Option<Vec<String>>,
}

impl Query {
    /// Append a namespace segment.
    ///
    /// Pushes the current `name` into the namespace prefix and sets `name`
    /// to the new segment.
    pub fn namespace(mut self, segment: &str) -> Self {
        self.namespace = Some(match self.namespace.take() {
            Some(ns) => format!("{}.{}", ns, self.name),
            None => self.name.clone(),
        });
        self.name = segment.to_string();
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

    /// Start building an upsert statement.
    pub fn upsert(self) -> UpsertQuery {
        UpsertQuery::new(self.name, self.namespace, self.field_names)
    }
}

#[cfg(feature = "schema")]
impl From<&Entity> for Query {
    fn from(entity: &Entity) -> Self {
        Self {
            name: entity.name.to_string(),
            namespace: entity.namespace.as_ref().map(|s| s.to_string()),
            field_names: Some(entity.field_names().map(|s| s.to_string()).collect()),
        }
    }
}

impl From<&str> for Query {
    /// Parse an entity name string. Supports plain names (`"users"`) and
    /// dot-separated namespaced names (`"identity.users"` → namespace
    /// `"identity"`, name `"users"`).
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

impl From<String> for Query {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// The kind of JOIN to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    /// `INNER JOIN` — keep rows where both sides match.
    Inner,
    /// `LEFT [OUTER] JOIN` — keep all left rows; right side is `NULL` when unmatched.
    Left,
    /// `RIGHT [OUTER] JOIN` — keep all right rows; left side is `NULL` when unmatched.
    Right,
    /// `FULL [OUTER] JOIN` — keep rows from both sides; unmatched columns are `NULL`.
    Full,
    /// `CROSS JOIN` — Cartesian product; no `ON` clause.
    Cross,
}

#[cfg(all(test, feature = "schema"))]
mod tests;

//! Owned, serde-friendly schema constraint types.
//!
//! These types are the canonical home for `Entity` / `Field` constraints in
//! the data-model layer. They use owned strings (`Arc<str>`) so they can
//! implement `Deserialize` and survive round-trips through serde.
//!
//! `dol-schema` is the single owner of constraint types; `dol-ir` re-exports
//! the names it needs to embed in DDL `Statement` variants.

use std::sync::Arc;

/// Action to take when a referenced record is deleted or updated.
///
/// Each variant has a backend-by-backend mapping:
///
/// | Variant       | SQL                  | Document store          | Graph             |
/// |---------------|----------------------|-------------------------|-------------------|
/// | `Forbid`      | `NO ACTION`          | reject mutation         | reject deletion   |
/// | `Cascade`     | `CASCADE`            | cascade write/delete    | cascade traversal |
/// | `Detach`      | `SET NULL`           | clear referencing field | drop edge         |
/// | `Reject`      | `RESTRICT`           | refuse mutation         | refuse deletion   |
/// | `UseDefault`  | `SET DEFAULT`        | reset to schema default | reset to default  |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RefAction {
    /// Reject the mutation outright; the reference is treated as a hard
    /// invariant. Equivalent to `NO ACTION` in SQL.
    Forbid,
    /// Propagate the mutation to all referencing records.
    Cascade,
    /// Clear the referencing field on the dependent record. Equivalent to
    /// `SET NULL` in SQL.
    Detach,
    /// Refuse the mutation if any referencing record exists. Equivalent to
    /// `RESTRICT` in SQL — semantically narrower than [`Forbid`](Self::Forbid)
    /// in that it forbids the mutation immediately rather than at commit time.
    Reject,
    /// Reset the referencing field to its declared default. Equivalent to
    /// `SET DEFAULT` in SQL.
    UseDefault,
}

/// How a computed field is materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComputedKind {
    /// Materialized on write — the value is stored alongside the record and
    /// recomputed only when its inputs change.
    Materialized,
    /// Computed on demand — the value is recomputed every read; the field
    /// occupies no storage of its own.
    OnDemand,
}

/// An inline relation reference on a single field — the dependent side of a
/// directed link to another entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelationRef {
    /// Target entity name.
    pub entity: Arc<str>,
    /// Target field within `entity`.
    pub field: Arc<str>,
    pub on_delete: RefAction,
    pub on_update: RefAction,
}

impl RelationRef {
    pub fn new(entity: impl Into<Arc<str>>, field: impl Into<Arc<str>>) -> Self {
        Self {
            entity: entity.into(),
            field: field.into(),
            on_delete: RefAction::Forbid,
            on_update: RefAction::Forbid,
        }
    }

    pub fn on_delete(mut self, action: RefAction) -> Self {
        self.on_delete = action;
        self
    }

    pub fn on_update(mut self, action: RefAction) -> Self {
        self.on_update = action;
        self
    }
}

/// An entity-level constraint (composite uniqueness, multi-field relation,
/// invariant expression, composite identity).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityConstraint {
    /// Uniqueness constraint over one or more fields.
    Unique(Vec<Arc<str>>),
    /// A multi-field relation to another entity.
    Relation {
        fields: Vec<Arc<str>>,
        ref_entity: Arc<str>,
        ref_fields: Vec<Arc<str>>,
        on_delete: RefAction,
    },
    /// A boolean invariant expressed as text — every record must satisfy it.
    Invariant(Arc<str>),
    /// Composite identity constraint — these fields uniquely identify an
    /// entity instance.
    Identity(Vec<Arc<str>>),
}

impl EntityConstraint {
    /// Build a `Unique` constraint from any iterable of string-likes.
    pub fn unique<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::Unique(fields.into_iter().map(Into::into).collect())
    }

    /// Build an `Identity` constraint — the fields that uniquely identify an
    /// entity instance.
    pub fn identity<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::Identity(fields.into_iter().map(Into::into).collect())
    }

    /// Build an `Invariant` constraint — a boolean expression every record
    /// must satisfy.
    pub fn invariant(expr: impl Into<Arc<str>>) -> Self {
        Self::Invariant(expr.into())
    }

    /// Build a multi-field `Relation` constraint.
    pub fn relation<I, J, S, T>(
        fields: I,
        ref_entity: impl Into<Arc<str>>,
        ref_fields: J,
        on_delete: RefAction,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        J: IntoIterator<Item = T>,
        S: Into<Arc<str>>,
        T: Into<Arc<str>>,
    {
        Self::Relation {
            fields: fields.into_iter().map(Into::into).collect(),
            ref_entity: ref_entity.into(),
            ref_fields: ref_fields.into_iter().map(Into::into).collect(),
            on_delete,
        }
    }
}

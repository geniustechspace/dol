//! Schema-level field and entity constraint primitives.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

/// Action to take when a referenced record is deleted or updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum RefAction {
    /// Reject the mutation outright; equivalent to `NO ACTION` in SQL.
    Forbid,
    /// Propagate the mutation to all referencing records.
    Cascade,
    /// Clear the referencing field; equivalent to `SET NULL` in SQL.
    Detach,
    /// Refuse the mutation if any referencing record exists.
    Reject,
    /// Reset the referencing field to its declared default.
    UseDefault,
}

/// How a computed field is materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ComputedKind {
    /// Materialized on write.
    Materialized,
    /// Computed on demand.
    OnDemand,
}

/// An inline relation reference on a single field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RelationRef {
    /// Target entity name.
    pub entity: Arc<str>,
    /// Target field within `entity`.
    pub field: Arc<str>,
    /// Action to take when the referenced record is deleted.
    pub on_delete: RefAction,
    /// Action to take when the referenced record is updated.
    pub on_update: RefAction,
}

impl RelationRef {
    /// Build a `RelationRef` defaulted to `RefAction::Forbid`.
    pub fn new(entity: impl Into<Arc<str>>, field: impl Into<Arc<str>>) -> Self {
        Self {
            entity: entity.into(),
            field: field.into(),
            on_delete: RefAction::Forbid,
            on_update: RefAction::Forbid,
        }
    }

    /// Set the `on_delete` action.
    pub fn on_delete(mut self, action: RefAction) -> Self {
        self.on_delete = action;
        self
    }

    /// Set the `on_update` action.
    pub fn on_update(mut self, action: RefAction) -> Self {
        self.on_update = action;
        self
    }
}

/// An entity-level constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EntityConstraint {
    /// Uniqueness constraint over one or more fields.
    Unique(Vec<Arc<str>>),
    /// A multi-field relation to another entity.
    Relation {
        /// Local fields participating in the relation.
        fields: Vec<Arc<str>>,
        /// Target entity name.
        ref_entity: Arc<str>,
        /// Target fields within `ref_entity`.
        ref_fields: Vec<Arc<str>>,
        /// Action to take when the referenced record is deleted.
        on_delete: RefAction,
    },
    /// A boolean invariant expressed as text.
    Invariant(Arc<str>),
    /// Composite identity constraint.
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

    /// Build an `Identity` constraint.
    pub fn identity<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::Identity(fields.into_iter().map(Into::into).collect())
    }

    /// Build an `Invariant` constraint.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_ref_builders() {
        let r = RelationRef::new("users", "id")
            .on_delete(RefAction::Cascade)
            .on_update(RefAction::Detach);
        assert_eq!(&*r.entity, "users");
        assert_eq!(&*r.field, "id");
        assert_eq!(r.on_delete, RefAction::Cascade);
        assert_eq!(r.on_update, RefAction::Detach);
    }
}

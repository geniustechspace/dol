pub use dol_core::strings::{Interner, StrId};

pub struct Reference {
    /// Target entity name.
    pub entity: StrId,
    /// Target field within `entity`.
    pub field: StrId,
    /// Action to take when the referenced record is deleted.
    pub on_delete: RefAction,
    /// Action to take when the referenced record is updated.
    pub on_update: RefAction,
}

impl Reference {
    /// Build a `Reference`.
    pub fn new(entity: StrId, field: StrId) -> Self {
        Self {
            entity,
            field,
            on_delete: RefAction::Forbid,
            on_update: RefAction::Cascade,
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

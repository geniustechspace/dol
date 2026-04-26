//! `Upsert` — insert-or-update with conflict resolution.

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::operation::insert::InsertSource;
use crate::operation::update::UpdateAssignment;
use crate::target::{Symbol, Target};

/// Conflict-resolution clause for an upsert.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OnConflict {
    /// Do nothing on conflict.
    DoNothing,
    /// Update the named fields with the given assignments.
    DoUpdate {
        sets: SmallVec<[UpdateAssignment; 4]>,
        filter: Option<NodeId>,
    },
}

/// `Upsert` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Upsert {
    pub target: Target,
    pub source: InsertSource,
    /// Conflict-detection columns / fields.
    pub conflict_keys: SmallVec<[Symbol; 2]>,
    pub on_conflict: OnConflict,
    pub returning: Option<NodeId>,
}

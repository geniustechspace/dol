//! `Mask` — column / field masking policy.

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::operation::schema::StructuralVerb;
use crate::target::{Symbol, Target};

/// `Mask` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaskOp {
    pub verb: StructuralVerb,
    pub target: Target,
    pub name: Symbol,
    pub fields: SmallVec<[Symbol; 2]>,
    /// Arena `NodeId` for the masking expression (e.g.
    /// `case when current_role = 'admin' then x else null end`).
    pub mask_expr: Option<NodeId>,
}

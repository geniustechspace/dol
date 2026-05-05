//! `Mask` — column / field masking policy.

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// `Mask` operation.
///
/// Defines a column / field masking policy that transforms values based on
/// the current role or context.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{MaskOp, StructuralVerb};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // Mask the "ssn" column for non-admins
/// let op: Operation = MaskOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     name: Symbol::from_hash(1),
///     fields: smallvec::smallvec![Symbol::from_hash(2)],
///     mask_expr: None, // Would be set to arena NodeId for real mask
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Mask);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaskOp {
    /// `Create` / `Drop` / `Alter`.
    pub verb: StructuralVerb,
    /// Target the mask applies to.
    pub target: Target,
    /// Interned mask rule name.
    pub name: Symbol,
    /// Fields covered by this mask.
    pub fields: SmallVec<[Symbol; 2]>,
    /// Arena `NodeId` for the masking expression (e.g.
    /// `case when current_role = 'admin' then x else null end`).
    pub mask_expr: Option<NodeId>,
}

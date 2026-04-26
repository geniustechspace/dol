//! Field-level structural operations (`Operation::Field`).
//!
//! Used to add / drop / alter / rename a single field on a target. This is
//! the v2 split of the v1 `AlterEntity::AddField`/`DropField`/`RenameField`
//! actions. Capability checks pair `OpKind::Field` with the parent
//! [`TargetKind`](crate::target::TargetKind).

use dol_core::DataType;
use dol_expr::ids::NodeId;
use dol_schema::{ComputedKind, RelationRef};

use crate::operation::schema::StructuralVerb;
use crate::target::{Symbol, Target};

/// Field-level structural operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldOp {
    /// `Create` (add column) / `Drop` / `Alter` / `Rename`.
    pub verb: StructuralVerb,
    /// Target the field lives on.
    pub target: Target,
    /// Symbol of the field. For `Rename`, this is the *current* name.
    pub field: Symbol,
    /// Field body for `Create` / `Alter`. `None` for `Drop` / `Rename`.
    pub def: Option<FieldDefV2>,
    /// New field name when `verb == Rename`.
    pub new_name: Option<Symbol>,
}

/// Field definition for v2 ops.
///
/// Every expression slot is an arena [`NodeId`] — there are no `Option<String>`
/// expression fields. This is enforced by code review (per RFC §3) and by the
/// absence of `String` slots on this struct.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldDefV2 {
    pub name: Symbol,
    pub data_type: DataType,
    pub identity: bool,
    pub nullable: bool,
    pub unique: bool,
    pub references: Option<RelationRef>,
    /// Arena `NodeId` for the column default expression.
    pub default_expr: Option<NodeId>,
    /// Arena `NodeId` for a column-level CHECK constraint.
    pub check_expr: Option<NodeId>,
    /// Arena `NodeId` for a generated / computed column expression, paired
    /// with the [`ComputedKind`].
    pub generated: Option<(ComputedKind, NodeId)>,
    pub lookup: bool,
    pub auto_assign: bool,
    pub collation: Option<Symbol>,
    pub comment: Option<Symbol>,
}

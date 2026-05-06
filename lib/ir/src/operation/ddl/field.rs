//! Field-level structural operations (`Operation::Field`).
//!
//! Used to add / drop / alter / rename a single field on a target. Capability
//! checks pair `OpKind::Field` with the parent
//! [`TargetKind`](crate::target::TargetKind).

use dol_core::DataType;
use dol_expr::ids::NodeId;
use dol_schema::{ComputedKind, RelationRef};

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// Field-level structural operation: add, drop, alter, or rename a single
/// field on a target.
///
/// # What this represents
///
/// Maps to SQL `ALTER TABLE … ADD/DROP/ALTER/RENAME COLUMN`, or to
/// adding / removing a property on a document-store collection schema. The
/// [`Self::target`] picks which TargetKind the field lives on (relation,
/// document collection, …).
///
/// # Examples
///
/// ```
/// use dol_core::DataType;
/// use dol_ir::operation::{FieldDef, FieldOp, StructuralVerb};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // ALTER TABLE users ADD COLUMN email TEXT NOT NULL
/// let op: Operation = FieldOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     field: Symbol::from_hash(1),
///     def: Some(FieldDef {
///         name: Symbol::from_hash(1),
///         data_type: DataType::unbounded_string(),
///         identity: false,
///         nullable: false,
///         unique: false,
///         references: None,
///         default_expr: None,
///         check_expr: None,
///         generated: None,
///         lookup: false,
///         auto_assign: false,
///         collation: None,
///         comment: None,
///     }),
///     new_name: None,
/// }
/// .into();
/// assert_eq!(op.kind(), dol_ir::OpKind::Field);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FieldOp {
    /// `Create` (add column) / `Drop` / `Alter` / `Rename`.
    pub verb: StructuralVerb,
    /// Target the field lives on.
    pub target: Target,
    /// Symbol of the field. For `Rename`, this is the *current* name.
    pub field: Symbol,
    /// Field body for `Create` / `Alter`. `None` for `Drop` / `Rename`.
    pub def: Option<FieldDef>,
    /// New field name when `verb == Rename`.
    pub new_name: Option<Symbol>,
}

/// Definition of a single field in a relation, document collection, or
/// equivalent structured target.
///
/// Every expression slot is an arena [`NodeId`] — there are no `String`
/// expression fields. Default values, `CHECK` constraints, and computed
/// expressions all live in the program's [`dol_expr::ExprArena`] and are
/// referenced by id.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FieldDef {
    /// Field / column name.
    pub name: Symbol,
    /// Static type of the field.
    pub data_type: DataType,
    /// Field is part of an `IDENTITY` / surrogate key (auto-generated).
    pub identity: bool,
    /// Field accepts `NULL`.
    pub nullable: bool,
    /// Field carries a `UNIQUE` constraint.
    pub unique: bool,
    /// Foreign-key target, when the field references another relation.
    pub references: Option<RelationRef>,
    /// Arena `NodeId` for the column default expression.
    pub default_expr: Option<NodeId>,
    /// Arena `NodeId` for a column-level CHECK constraint.
    pub check_expr: Option<NodeId>,
    /// Arena `NodeId` for a generated / computed column expression, paired
    /// with the [`ComputedKind`].
    pub generated: Option<(ComputedKind, NodeId)>,
    /// Field is automatically indexed for fast lookup.
    pub lookup: bool,
    /// Field receives an auto-assigned value at insert time when omitted.
    pub auto_assign: bool,
    /// Optional collation name (text fields only).
    pub collation: Option<Symbol>,
    /// Optional comment / description.
    pub comment: Option<Symbol>,
}

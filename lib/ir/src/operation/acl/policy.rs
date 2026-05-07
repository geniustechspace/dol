//! `Policy` — row-level security / authorisation policy lifecycle.

use dol_expr::ids::NodeId;

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// What DML/DQL kinds the policy applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PolicyScope {
    /// Policy applies to read operations (SELECT, Query).
    Read,
    /// Policy applies to write operations (INSERT, UPDATE, DELETE).
    Write,
    /// Policy applies to all operations.
    All,
}

/// `Policy` operation.
///
/// Defines a row-level security policy that restricts access to rows
/// based on filter and check expressions.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{PolicyOp, PolicyScope, StructuralVerb};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::operation::Operation;
///
/// // CREATE POLICY users_rls ON users FOR ALL USING (tenant_id = current_tenant())
/// let op: Operation = PolicyOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     name: Symbol::from_hash(1),
///     scope: PolicyScope::All,
///     using_expr: None, // Would be set to arena NodeId for real policy
///     check_expr: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::operation::OpKind::Policy);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PolicyOp {
    /// `Create` / `Drop` / `Alter`.
    pub verb: StructuralVerb,
    /// Target the policy applies to.
    pub target: Target,
    /// Interned policy name.
    pub name: Symbol,
    /// Which operations the policy governs.
    pub scope: PolicyScope,
    /// Arena `NodeId` for the `USING` filter expression.
    pub using_expr: Option<NodeId>,
    /// Arena `NodeId` for the `CHECK` expression.
    pub check_expr: Option<NodeId>,
}

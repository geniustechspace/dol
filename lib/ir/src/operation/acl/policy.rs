//! `Policy` — row-level security / authorisation policy lifecycle.

use dol_expr::ids::NodeId;

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// What DML/DQL kinds the policy applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyScope {
    Read,
    Write,
    All,
}

/// `Policy` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolicyOp {
    pub verb: StructuralVerb,
    pub target: Target,
    pub name: Symbol,
    pub scope: PolicyScope,
    /// Arena `NodeId` for the `USING` filter expression.
    pub using_expr: Option<NodeId>,
    /// Arena `NodeId` for the `CHECK` expression.
    pub check_expr: Option<NodeId>,
}

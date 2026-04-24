//! Access-control operations — Grant, Revoke, DefinePolicy.

use dol_expr::ids::NodeId;

/// Grant privileges on a resource to a role.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Grant {
    pub privilege: Privilege,
    pub on_target: String,
    pub to_role: String,
}

/// Revoke privileges on a resource from a role.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Revoke {
    pub privilege: Privilege,
    pub on_target: String,
    pub from_role: String,
}

/// Privilege types that can be granted or revoked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Privilege {
    Select,
    Insert,
    Update,
    Delete,
    All,
    Usage,
    Create,
    Connect,
    Custom(String),
}

/// Define a row-level security policy.
///
/// Expression fields use [`NodeId`] arena references (paired with an
/// [`dol_expr::arena::ExprArena`]) instead of the old `Expr<'a>` tree.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefinePolicy {
    pub name: String,
    pub on_model: String,
    pub action: PolicyAction,
    /// Arena-based USING filter expression, or `None`.
    pub using_expr: Option<NodeId>,
    /// Arena-based CHECK expression, or `None`.
    pub check_expr: Option<NodeId>,
}

/// Scope of a policy: which DML commands it applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyAction {
    Read,
    Write,
    All,
}

//! Control operations — canonical representation of access control operations.

use crate::expr::Expr;

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

/// Define a policy for declarative access control.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefinePolicy<'a> {
    pub name: String,
    pub on_model: String,
    pub action: PolicyAction,
    /// Filter expression for which rows are visible.
    pub using_expr: Option<Expr<'a>>,
    /// Check expression for mutations.
    pub check_expr: Option<Expr<'a>>,
}

/// Policy action scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyAction {
    Read,
    Write,
    All,
}

//! Control IR — canonical representation of access control operations.

use dol_expr::Expr;

/// Grant privileges on a resource to a role.
#[derive(Debug, Clone)]
pub struct GrantIR {
    pub privilege: Privilege,
    pub on_target: String,
    pub to_role: String,
}

/// Revoke privileges on a resource from a role.
#[derive(Debug, Clone)]
pub struct RevokeIR {
    pub privilege: Privilege,
    pub on_target: String,
    pub from_role: String,
}

/// Privilege types that can be granted or revoked.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
pub struct DefinePolicyIR {
    pub name: String,
    pub on_model: String,
    pub action: PolicyAction,
    /// Filter expression for which rows are visible.
    pub using_expr: Option<Expr>,
    /// Check expression for mutations.
    pub check_expr: Option<Expr>,
}

/// Policy action scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    Read,
    Write,
    All,
}

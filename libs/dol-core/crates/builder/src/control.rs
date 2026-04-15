//! Control builders — GRANT, REVOKE, DEFINE POLICY.

use dol_expr::Expr;
use dol_ir::control::{DefinePolicyIR, GrantIR, PolicyAction, RevokeIR};

/// Builder for `GRANT` statements.
#[derive(Debug, Clone)]
pub struct GrantBuilder {
    privilege: Privilege,
    on_target: String,
    to_role: String,
}

impl GrantBuilder {
    pub fn new(privilege: Privilege) -> Self {
        Self {
            privilege,
            on_target: String::new(),
            to_role: String::new(),
        }
    }

    pub fn on(mut self, target: &str) -> Self {
        self.on_target = target.to_string();
        self
    }

    pub fn to(mut self, role: &str) -> Self {
        self.to_role = role.to_string();
        self
    }

    /// Build the canonical GrantIR.
    pub fn build(&self) -> GrantIR {
        GrantIR {
            privilege: self.privilege.clone(),
            on_target: self.on_target.clone(),
            to_role: self.to_role.clone(),
        }
    }
}

// Re-export Privilege for convenience
pub use dol_ir::control::Privilege;

/// Builder for `REVOKE` statements.
#[derive(Debug, Clone)]
pub struct RevokeBuilder {
    privilege: Privilege,
    on_target: String,
    from_role: String,
}

impl RevokeBuilder {
    pub fn new(privilege: Privilege) -> Self {
        Self {
            privilege,
            on_target: String::new(),
            from_role: String::new(),
        }
    }

    pub fn on(mut self, target: &str) -> Self {
        self.on_target = target.to_string();
        self
    }

    pub fn from(mut self, role: &str) -> Self {
        self.from_role = role.to_string();
        self
    }

    /// Build the canonical RevokeIR.
    pub fn build(&self) -> RevokeIR {
        RevokeIR {
            privilege: self.privilege.clone(),
            on_target: self.on_target.clone(),
            from_role: self.from_role.clone(),
        }
    }
}

// ===========================================================================
// DefinePolicyBuilder — CREATE POLICY (row-level security)
// ===========================================================================

/// Builder for `CREATE POLICY` statements (row-level security).
///
/// ```rust
/// use dol_builder::control::DefinePolicyBuilder;
/// use dol_expr::{field, param};
/// use dol_ir::control::PolicyAction;
///
/// let ir = DefinePolicyBuilder::new("tenant_isolation")
///     .on("orders")
///     .for_action(PolicyAction::All)
///     .using(field("tenant_id").eq(param()))
///     .check(field("tenant_id").eq(param()))
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DefinePolicyBuilder {
    name: String,
    on_model: String,
    action: PolicyAction,
    using_expr: Option<Expr>,
    check_expr: Option<Expr>,
}

impl DefinePolicyBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            on_model: String::new(),
            action: PolicyAction::All,
            using_expr: None,
            check_expr: None,
        }
    }

    /// Set the target model (table) for this policy.
    pub fn on(mut self, model: &str) -> Self {
        self.on_model = model.to_string();
        self
    }

    /// Set the policy action scope (Read, Write, or All).
    pub fn for_action(mut self, action: PolicyAction) -> Self {
        self.action = action;
        self
    }

    /// Set the USING expression (filter for which rows are visible).
    pub fn using(mut self, expr: impl Into<Expr>) -> Self {
        self.using_expr = Some(expr.into());
        self
    }

    /// Set the WITH CHECK expression (filter for mutations).
    pub fn check(mut self, expr: impl Into<Expr>) -> Self {
        self.check_expr = Some(expr.into());
        self
    }

    /// Build the canonical [`DefinePolicyIR`].
    pub fn build(&self) -> DefinePolicyIR {
        DefinePolicyIR {
            name: self.name.clone(),
            on_model: self.on_model.clone(),
            action: self.action,
            using_expr: self.using_expr.clone(),
            check_expr: self.check_expr.clone(),
        }
    }
}

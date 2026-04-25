//! Control builders — GRANT, REVOKE, DEFINE POLICY.

use dol_expr::tree::Expr;
use dol_ir::control::{Grant, Revoke};

// Re-export Privilege and PolicyAction from dol-ir
pub use dol_ir::control::PolicyAction;
pub use dol_ir::control::Privilege;

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

    /// Build the canonical Grant.
    pub fn build(&self) -> Grant {
        Grant {
            privilege: self.privilege.clone(),
            on_target: self.on_target.clone(),
            to_role: self.to_role.clone(),
        }
    }
}

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

    /// Build the canonical [`Revoke`] (dol-ir).
    pub fn build(&self) -> Revoke {
        Revoke {
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
/// The builder is parameterized over the lifetime of the tree expressions
/// it stores (`Expr<'a>`), so callers can pass expressions that borrow from
/// request-scoped buffers as well as `'static` literals.
///
/// ```rust
/// use dol_query::builder::control::DefinePolicyBuilder;
/// use dol_expr::tree::{field, param};
/// use dol_query::builder::control::PolicyAction;
///
/// let program = DefinePolicyBuilder::new("tenant_isolation")
///     .on("orders")
///     .for_action(PolicyAction::All)
///     .using(field("tenant_id").eq(param()))
///     .check(field("tenant_id").eq(param()))
///     .build();
/// let _stmt = &program.stmt;
/// ```
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DefinePolicyBuilder<'a> {
    name: String,
    on_model: String,
    action: PolicyAction,
    using_expr: Option<Expr<'a>>,
    check_expr: Option<Expr<'a>>,
}

impl<'a> DefinePolicyBuilder<'a> {
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
    pub fn using(mut self, expr: impl Into<Expr<'a>>) -> Self {
        self.using_expr = Some(expr.into());
        self
    }

    /// Set the WITH CHECK expression (filter for mutations).
    pub fn check(mut self, expr: impl Into<Expr<'a>>) -> Self {
        self.check_expr = Some(expr.into());
        self
    }

    /// Build the arena-based IR as a [`dol_ir::Program`].
    pub fn build(&self) -> dol_ir::Program {
        use dol_expr::lower::lower_expr;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();

        // `PolicyAction` is re-exported from `dol_ir::control`, so the
        // builder field and IR type are the same; just copy it.
        let ir_action = self.action;

        let using_id = self
            .using_expr
            .as_ref()
            .map(|e| lower_expr(e, &mut arena, &mut interner));
        let check_id = self
            .check_expr
            .as_ref()
            .map(|e| lower_expr(e, &mut arena, &mut interner));

        let policy = dol_ir::DefinePolicy {
            name: self.name.clone(),
            on_model: self.on_model.clone(),
            action: ir_action,
            using_expr: using_id,
            check_expr: check_id,
        };

        dol_ir::Program::new(dol_ir::Statement::DefinePolicy(policy), arena, interner)
    }
}

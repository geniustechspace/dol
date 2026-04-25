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
    ///
    /// Consumes the builder so the already-owned `String` / `Privilege`
    /// fields can be moved into the IR without cloning.
    pub fn build(self) -> Grant {
        Grant {
            privilege: self.privilege,
            on_target: self.on_target,
            to_role: self.to_role,
        }
    }

    /// Build a [`dol_ir::Program`] wrapping `Statement::Grant`.
    ///
    /// Convenience helper that mirrors [`DefinePolicyBuilder::build`] so
    /// callers can feed the result directly into a backend without manually
    /// wrapping with [`dol_ir::Program::from_stmt`].
    pub fn build_program(self) -> dol_ir::Program {
        dol_ir::Program::from_stmt(dol_ir::Statement::Grant(self.build()))
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
    ///
    /// Consumes the builder so the already-owned `String` / `Privilege`
    /// fields can be moved into the IR without cloning.
    pub fn build(self) -> Revoke {
        Revoke {
            privilege: self.privilege,
            on_target: self.on_target,
            from_role: self.from_role,
        }
    }

    /// Build a [`dol_ir::Program`] wrapping `Statement::Revoke`.
    ///
    /// Convenience helper that mirrors [`DefinePolicyBuilder::build`] so
    /// callers can feed the result directly into a backend without manually
    /// wrapping with [`dol_ir::Program::from_stmt`].
    pub fn build_program(self) -> dol_ir::Program {
        dol_ir::Program::from_stmt(dol_ir::Statement::Revoke(self.build()))
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
    ///
    /// Consumes the builder so the already-owned `name` / `on_model`
    /// `String`s can be moved into the IR directly, while any stored
    /// `Expr<'a>` operands are lowered by reference without an extra clone.
    pub fn build(self) -> dol_ir::Program {
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
            name: self.name,
            on_model: self.on_model,
            action: ir_action,
            using_expr: using_id,
            check_expr: check_id,
        };

        dol_ir::Program::new(dol_ir::Statement::DefinePolicy(policy), arena, interner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_expr::tree::bool_expr;

    #[test]
    fn define_policy_build_lowers_using_and_check_exprs() {
        let program = DefinePolicyBuilder::new("tenant_isolation")
            .on("accounts")
            .for_action(PolicyAction::Read)
            .using(bool_expr(true))
            .check(bool_expr(false))
            .build();

        let policy = match &program.stmt {
            dol_ir::Statement::DefinePolicy(policy) => policy,
            stmt => panic!("expected Statement::DefinePolicy, got {stmt:?}"),
        };

        assert_eq!(policy.name, "tenant_isolation");
        assert_eq!(policy.on_model, "accounts");
        assert_eq!(policy.action, PolicyAction::Read);
        assert!(
            policy.using_expr.is_some(),
            "expected using_expr to be lowered into a NodeId"
        );
        assert!(
            policy.check_expr.is_some(),
            "expected check_expr to be lowered into a NodeId"
        );

        let rendered_once = format!("{:?}", &program.stmt);
        let rendered_twice = format!("{:?}", &program.stmt);
        assert_eq!(rendered_once, rendered_twice);
        assert!(rendered_once.contains("DefinePolicy"));
        assert!(rendered_once.contains("using_expr: Some("));
        assert!(rendered_once.contains("check_expr: Some("));
    }
}

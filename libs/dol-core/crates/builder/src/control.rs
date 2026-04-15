//! Control builders — GRANT, REVOKE.

use dol_ir::control::{GrantIR, RevokeIR};

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

//! Policy definition.
//!
//! A [`Policy`] attaches access-control rules to an [`Entity`].
//! [`PolicyKind`] categorises the permission level.
//!
//! [`Entity`]: crate::schema::entity::Entity

extern crate alloc;

use dol_cas::handle::{PolicyId, StrId};

/// An access-control policy attached to an entity.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 250.
#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    /// Arena-local identifier for this policy.
    pub id: PolicyId,
    /// Interned name. Resolved via `SchemaCatalog.strings`.
    pub name: StrId,
    /// The permission category.
    pub kind: PolicyKind,
}

impl Policy {
    /// Construct a new policy.
    #[must_use]
    pub fn new(id: PolicyId, name: StrId, kind: PolicyKind) -> Self {
        Self { id, name, kind }
    }
}

/// The permission category of a policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PolicyKind {
    /// Read access (SELECT / GET).
    Read,
    /// Write access (INSERT / UPDATE / DELETE).
    Write,
    /// Administrative access (DDL / GRANT / REVOKE).
    Admin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::ids::Id;

    fn make_policy_id(n: u32) -> PolicyId {
        Id::from_u32(n).unwrap()
    }

    fn make_str_id(n: u32) -> StrId {
        Id::from_u32(n).unwrap()
    }

    #[test]
    fn policy_new_basic() {
        let p = Policy::new(make_policy_id(1), make_str_id(10), PolicyKind::Read);
        assert_eq!(p.id, make_policy_id(1));
        assert_eq!(p.name, make_str_id(10));
        assert_eq!(p.kind, PolicyKind::Read);
    }

    #[test]
    fn policy_debug_round_trip() {
        let p = Policy::new(make_policy_id(2), make_str_id(20), PolicyKind::Write);
        let dbg = format!("{p:?}");
        assert!(dbg.contains("Policy"));
        assert!(dbg.contains("Write"));
    }

    #[test]
    fn policy_kind_hash_eq() {
        let mut set = std::collections::HashSet::new();
        set.insert(PolicyKind::Read);
        set.insert(PolicyKind::Write);
        set.insert(PolicyKind::Admin);
        assert_eq!(set.len(), 3);
    }
}

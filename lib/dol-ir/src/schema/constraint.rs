//! Constraint definition.
//!
//! A [`Constraint`] is an inline integrity rule attached to an
//! [`Entity`]. [`ConstraintKind`] enumerates the supported constraint
//! categories.
//!
//! [`Entity`]: crate::schema::entity::Entity

extern crate alloc;

use dol_cas::handle::{EntityId, NodeId, StrId};

/// An inline integrity constraint on an entity.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 247.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// Interned constraint name. Resolved via `SchemaCatalog.strings`.
    pub name: StrId,
    /// The kind of constraint.
    pub kind: ConstraintKind,
}

impl Constraint {
    /// Construct a new constraint.
    #[must_use]
    pub fn new(name: StrId, kind: ConstraintKind) -> Self {
        Self { name, kind }
    }
}

/// The category of an integrity constraint.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConstraintKind {
    /// The field(s) must not be NULL.
    NotNull,
    /// The field(s) must be unique across all rows.
    Unique,
    /// The field(s) form the primary key.
    PrimaryKey,
    /// The field(s) reference another entity's primary key.
    ForeignKey {
        /// The referenced entity.
        references: EntityId,
    },
    /// An arbitrary boolean expression that must hold.
    Check {
        /// The expression node producing a boolean.
        expr: NodeId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::ids::Id;

    fn make_str_id(n: u32) -> StrId {
        Id::from_u32(n).unwrap()
    }

    fn make_entity_id(n: u32) -> EntityId {
        Id::from_u32(n).unwrap()
    }

    fn make_node_id(n: u32) -> NodeId {
        Id::from_u32(n).unwrap()
    }

    #[test]
    fn constraint_new_basic() {
        let c = Constraint::new(make_str_id(1), ConstraintKind::NotNull);
        assert_eq!(c.name, make_str_id(1));
        assert!(matches!(c.kind, ConstraintKind::NotNull));
    }

    #[test]
    fn constraint_debug_round_trip() {
        let c = Constraint::new(make_str_id(2), ConstraintKind::PrimaryKey);
        let dbg = format!("{c:?}");
        assert!(dbg.contains("Constraint"));
        assert!(dbg.contains("PrimaryKey"));
    }

    #[test]
    fn constraint_kind_foreign_key() {
        let kind = ConstraintKind::ForeignKey {
            references: make_entity_id(99),
        };
        let ConstraintKind::ForeignKey { references } = kind else {
            panic!("expected ForeignKey");
        };
        assert_eq!(references, make_entity_id(99));
    }

    #[test]
    fn constraint_kind_check() {
        let kind = ConstraintKind::Check {
            expr: make_node_id(42),
        };
        let ConstraintKind::Check { expr } = kind else {
            panic!("expected Check");
        };
        assert_eq!(expr, make_node_id(42));
    }
}

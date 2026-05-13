//! Entity definition.
//!
//! An [`Entity`] is the schema-level representation of a table,
//! collection, or document type. It owns a field list, constraints,
//! relations, and attached policies.

extern crate alloc;

use alloc::vec::Vec;

use dol_cas::handle::{EntityId, FieldId, PolicyId, RelationId, StrId};

use crate::schema::constraint::Constraint;

/// A schema entity (table / collection / document type).
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 1428.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Arena-local identifier for this entity.
    pub id: EntityId,
    /// Interned name. Resolved via `SchemaCatalog.strings`.
    pub name: StrId,
    /// Ordered field list.
    pub fields: Vec<FieldId>,
    /// Inline constraints (PK, FK, CHECK, UNIQUE, NOT NULL).
    pub constraints: Vec<Constraint>,
    /// Outbound relations (edges to other entities).
    pub relations: Vec<RelationId>,
    /// Attached access-control policies.
    pub policies: Vec<PolicyId>,
}

impl Entity {
    /// Construct a new entity with empty field/constraint/relation/policy
    /// vectors.
    #[must_use]
    pub fn new(id: EntityId, name: StrId) -> Self {
        Self {
            id,
            name,
            fields: Vec::new(),
            constraints: Vec::new(),
            relations: Vec::new(),
            policies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::ids::Id;

    fn make_entity_id(n: u32) -> EntityId {
        Id::from_u32(n).unwrap()
    }

    fn make_str_id(n: u32) -> StrId {
        Id::from_u32(n).unwrap()
    }

    #[test]
    fn entity_new_creates_empty_vecs() {
        let e = Entity::new(make_entity_id(1), make_str_id(42));
        assert_eq!(e.id, make_entity_id(1));
        assert_eq!(e.name, make_str_id(42));
        assert!(e.fields.is_empty());
        assert!(e.constraints.is_empty());
        assert!(e.relations.is_empty());
        assert!(e.policies.is_empty());
    }

    #[test]
    fn entity_debug_round_trip() {
        let e = Entity::new(make_entity_id(7), make_str_id(99));
        let dbg = format!("{e:?}");
        assert!(dbg.contains("Entity"));
        assert!(dbg.contains("id:"));
    }

    #[test]
    fn entity_clone_eq() {
        let e1 = Entity::new(make_entity_id(3), make_str_id(10));
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }
}

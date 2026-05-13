//! Relation definition.
//!
//! A [`Relation`] models a directed edge between two [`Entity`] endpoints.
//! [`RelationKind`] captures the cardinality of the association.
//!
//! [`Entity`]: crate::schema::entity::Entity

extern crate alloc;

use dol_cas::handle::{EntityId, RelationId, StrId};

/// A relation (association / edge) between two entities.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 248.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    /// Arena-local identifier for this relation.
    pub id: RelationId,
    /// Interned name. Resolved via `SchemaCatalog.strings`.
    pub name: StrId,
    /// Source entity.
    pub from: EntityId,
    /// Target entity.
    pub to: EntityId,
    /// Cardinality of the relation.
    pub kind: RelationKind,
}

impl Relation {
    /// Construct a new relation.
    #[must_use]
    pub fn new(
        id: RelationId,
        name: StrId,
        from: EntityId,
        to: EntityId,
        kind: RelationKind,
    ) -> Self {
        Self { id, name, from, to, kind }
    }
}

/// The cardinality / multiplicity of a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RelationKind {
    /// One source row relates to exactly one target row.
    OneToOne,
    /// One source row relates to many target rows.
    OneToMany,
    /// Many source rows relate to many target rows (join table).
    ManyToMany,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::ids::Id;

    fn make_relation_id(n: u32) -> RelationId {
        Id::from_u32(n).unwrap()
    }

    fn make_entity_id(n: u32) -> EntityId {
        Id::from_u32(n).unwrap()
    }

    fn make_str_id(n: u32) -> StrId {
        Id::from_u32(n).unwrap()
    }

    #[test]
    fn relation_new_basic() {
        let r = Relation::new(
            make_relation_id(1),
            make_str_id(10),
            make_entity_id(2),
            make_entity_id(3),
            RelationKind::OneToMany,
        );
        assert_eq!(r.id, make_relation_id(1));
        assert_eq!(r.from, make_entity_id(2));
        assert_eq!(r.to, make_entity_id(3));
        assert_eq!(r.kind, RelationKind::OneToMany);
    }

    #[test]
    fn relation_debug_round_trip() {
        let r = Relation::new(
            make_relation_id(5),
            make_str_id(50),
            make_entity_id(6),
            make_entity_id(7),
            RelationKind::ManyToMany,
        );
        let dbg = format!("{r:?}");
        assert!(dbg.contains("Relation"));
        assert!(dbg.contains("ManyToMany"));
    }

    #[test]
    fn relation_kind_hash_eq() {
        let mut set = std::collections::HashSet::new();
        set.insert(RelationKind::OneToOne);
        set.insert(RelationKind::OneToMany);
        set.insert(RelationKind::ManyToMany);
        assert_eq!(set.len(), 3);
    }
}

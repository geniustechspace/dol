//! Lookup definition.
//!
//! A [`Lookup`] defines an indexed access path (primary key, secondary
//! index, unique constraint) into an [`Entity`]'s field set.
//!
//! [`Entity`]: crate::schema::entity::Entity

extern crate alloc;

use alloc::vec::Vec;

use dol_cas::handle::{EntityId, FieldId, LookupId, StrId};

/// An indexed access path into an entity.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 249.
#[derive(Debug, Clone, PartialEq)]
pub struct Lookup {
    /// Arena-local identifier for this lookup.
    pub id: LookupId,
    /// Interned name. Resolved via `SchemaCatalog.strings`.
    pub name: StrId,
    /// The entity this lookup indexes.
    pub entity: EntityId,
    /// Ordered list of indexed fields.
    pub fields: Vec<FieldId>,
    /// Whether the index enforces uniqueness.
    pub unique: bool,
}

impl Lookup {
    /// Construct a new lookup.
    #[must_use]
    pub fn new(
        id: LookupId,
        name: StrId,
        entity: EntityId,
        fields: Vec<FieldId>,
        unique: bool,
    ) -> Self {
        Self { id, name, entity, fields, unique }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use dol_core::ids::Id;

    fn make_lookup_id(n: u32) -> LookupId {
        Id::from_u32(n).unwrap()
    }

    fn make_entity_id(n: u32) -> EntityId {
        Id::from_u32(n).unwrap()
    }

    fn make_field_id(n: u32) -> FieldId {
        Id::from_u32(n).unwrap()
    }

    fn make_str_id(n: u32) -> StrId {
        Id::from_u32(n).unwrap()
    }

    #[test]
    fn lookup_new_basic() {
        let lu = Lookup::new(
            make_lookup_id(1),
            make_str_id(10),
            make_entity_id(2),
            vec![make_field_id(3), make_field_id(4)],
            true,
        );
        assert_eq!(lu.id, make_lookup_id(1));
        assert_eq!(lu.entity, make_entity_id(2));
        assert_eq!(lu.fields.len(), 2);
        assert!(lu.unique);
    }

    #[test]
    fn lookup_debug_round_trip() {
        let lu = Lookup::new(
            make_lookup_id(5),
            make_str_id(50),
            make_entity_id(6),
            vec![make_field_id(7)],
            false,
        );
        let dbg = format!("{lu:?}");
        assert!(dbg.contains("Lookup"));
        assert!(dbg.contains("unique: false"));
    }

    #[test]
    fn lookup_clone_eq() {
        let lu1 = Lookup::new(
            make_lookup_id(8),
            make_str_id(80),
            make_entity_id(9),
            vec![],
            true,
        );
        let lu2 = lu1.clone();
        assert_eq!(lu1, lu2);
    }
}

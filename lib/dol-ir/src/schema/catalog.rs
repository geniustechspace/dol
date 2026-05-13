//! Schema catalog — the container that owns every schema record.
//!
//! Per `dol-rewrite-plan-v2.md` §8.6 lines 1452–1460. The catalog is
//! the single point of truth for entity / field / relation / lookup /
//! policy definitions. Every name is interned into the catalog's own
//! [`StringPool`] so that name comparisons are pointer-cheap and the
//! same `StrId` round-trips into any consumer (the wire codec, the
//! query planner, the backend trait).
//!
//! The schema vocabulary is deliberately **backend-neutral**: an
//! [`Entity`] is *not* an SQL table, a [`Field`] is *not* an SQL
//! column, and a [`Lookup`] is *not* an SQL index. The same record
//! shapes describe document stores, graph databases, time-series
//! buckets, and key/value tables.
//!
//! This is the **M3d-β** slice — storage + name lookup only. The
//! constraint / FK / lookup integrity validation passes land in
//! **M3d-γ**.

extern crate alloc;

use core::fmt;

use dol_cas::handle::{EntityId, FieldId, LookupId, PolicyId, RelationId, StrId};
use dol_cas::pool::{ArenaStorage, DynPool};
use dol_cas::string_pool::{InternError, StringPool};
use dol_core::data_type::DataType;
use dol_core::ids::Id;

use crate::schema::entity::Entity;
use crate::schema::field::{Field, FieldType};
use crate::schema::lookup::Lookup;
use crate::schema::policy::{Policy, PolicyKind};
use crate::schema::relation::{Relation, RelationKind};

// ─── Error ──────────────────────────────────────────────────────────────────

/// First-violation enum returned by mutating catalog operations.
///
/// All catalog mutations either succeed and return the freshly minted
/// id, or fail with one of these variants. Variants surface the
/// underlying primitive failure (intern / pool capacity) rather than
/// wrapping it behind a generic "define failed" code so callers can
/// route each class to the right diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogError {
    /// Interning the record's name into the catalog's [`StringPool`]
    /// failed. Carries the underlying [`InternError`] (hash collision
    /// / capacity exceeded / pool poisoned).
    InternFailed(InternError),
    /// One of the schema pools refused to allocate because its slot
    /// table would overflow `u32::MAX`. All pool overflows collapse
    /// into this single variant — the catalog is `u32::MAX`-bounded
    /// across the board.
    PoolOverflow,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternFailed(inner) => write!(f, "schema catalog: {inner}"),
            Self::PoolOverflow => f.write_str("schema catalog: pool overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CatalogError {}

impl From<InternError> for CatalogError {
    #[inline]
    fn from(err: InternError) -> Self {
        Self::InternFailed(err)
    }
}

// ─── Catalog ────────────────────────────────────────────────────────────────

/// Container owning every schema record plus the interner that backs
/// every record name.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 1452. Fields are public to
/// match the plan exactly; the `define_*` / `*_by_name` helpers are
/// the recommended construction path but direct field access remains
/// available for callers that already hold their own ids (e.g. wire
/// decoders rehydrating a stored catalog).
#[derive(Debug)]
pub struct SchemaCatalog {
    /// Entity definitions, indexed by [`EntityId`].
    pub entities: DynPool<Entity>,
    /// Field definitions, indexed by [`FieldId`].
    pub fields: DynPool<Field>,
    /// Relation definitions, indexed by [`RelationId`].
    pub relations: DynPool<Relation>,
    /// Lookup definitions, indexed by [`LookupId`].
    pub lookups: DynPool<Lookup>,
    /// Policy definitions, indexed by [`PolicyId`].
    pub policies: DynPool<Policy>,
    /// String interner shared by every record name and every interned
    /// literal that needs a stable address.
    pub strings: StringPool,
}

impl Default for SchemaCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaCatalog {
    /// Construct an empty catalog backed by [`StringPool::standard`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: DynPool::new(),
            fields: DynPool::new(),
            relations: DynPool::new(),
            lookups: DynPool::new(),
            policies: DynPool::new(),
            strings: StringPool::standard(),
        }
    }

    /// Construct an empty catalog wrapping an explicit [`StringPool`].
    /// Useful when several catalogs share an upstream interner.
    #[must_use]
    pub fn with_strings(strings: StringPool) -> Self {
        Self {
            entities: DynPool::new(),
            fields: DynPool::new(),
            relations: DynPool::new(),
            lookups: DynPool::new(),
            policies: DynPool::new(),
            strings,
        }
    }

    // ─── Entity ────────────────────────────────────────────────────────────

    /// Intern `name`, allocate a fresh [`EntityId`], and push an empty
    /// [`Entity`] record into the catalog. Returns the new id.
    pub fn define_entity(&mut self, name: &str) -> Result<EntityId, CatalogError> {
        let name_id = self.strings.intern(name)?;
        let id = next_id::<_>(self.entities.len()).ok_or(CatalogError::PoolOverflow)?;
        let entity = Entity::new(id, name_id);
        push_or_overflow(&mut self.entities, entity)?;
        Ok(id)
    }

    /// Look up an [`Entity`] by id.
    #[must_use]
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get_by_id(id)
    }

    /// Mutable lookup of an [`Entity`] by id. Used by callers that
    /// need to attach fields / constraints / relations / policies
    /// after the entity has been defined.
    pub fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        get_mut_by_id(&mut self.entities, id)
    }

    /// Resolve an entity by its interned name (case-sensitive). Runs
    /// in `O(n_entities)`; intended for small catalogs and offline
    /// tooling.
    #[must_use]
    pub fn entity_by_name(&self, name: &str) -> Option<&Entity> {
        let name_id = self.strings.get_id(name)?;
        find_by_name(&self.entities, |e| e.name, name_id)
    }

    // ─── Field ─────────────────────────────────────────────────────────────

    /// Intern `name`, allocate a fresh [`FieldId`], and push a scalar
    /// [`Field`] with no default. Returns the new id.
    pub fn define_scalar_field(
        &mut self,
        name: &str,
        data_type: DataType,
        nullable: bool,
    ) -> Result<FieldId, CatalogError> {
        let name_id = self.strings.intern(name)?;
        let id = next_id::<_>(self.fields.len()).ok_or(CatalogError::PoolOverflow)?;
        let field = Field::new_scalar(id, name_id, data_type, nullable);
        push_or_overflow(&mut self.fields, field)?;
        Ok(id)
    }

    /// Intern `name`, allocate a fresh [`FieldId`], and push a field
    /// with an explicit [`FieldType`]. The caller decides whether the
    /// field is scalar / relation / computed and whether it carries a
    /// default literal.
    pub fn define_field(
        &mut self,
        name: &str,
        ty: FieldType,
        nullable: bool,
    ) -> Result<FieldId, CatalogError> {
        let name_id = self.strings.intern(name)?;
        let id = next_id::<_>(self.fields.len()).ok_or(CatalogError::PoolOverflow)?;
        let field = Field {
            id,
            name: name_id,
            ty,
            nullable,
            default: None,
        };
        push_or_overflow(&mut self.fields, field)?;
        Ok(id)
    }

    /// Look up a [`Field`] by id.
    #[must_use]
    pub fn field(&self, id: FieldId) -> Option<&Field> {
        self.fields.get_by_id(id)
    }

    /// Resolve a field by its interned name. Runs in `O(n_fields)`.
    #[must_use]
    pub fn field_by_name(&self, name: &str) -> Option<&Field> {
        let name_id = self.strings.get_id(name)?;
        find_by_name(&self.fields, |f| f.name, name_id)
    }

    // ─── Relation ──────────────────────────────────────────────────────────

    /// Intern `name`, allocate a fresh [`RelationId`], and push a
    /// [`Relation`] between `from` and `to`.
    pub fn define_relation(
        &mut self,
        name: &str,
        from: EntityId,
        to: EntityId,
        kind: RelationKind,
    ) -> Result<RelationId, CatalogError> {
        let name_id = self.strings.intern(name)?;
        let id = next_id::<_>(self.relations.len()).ok_or(CatalogError::PoolOverflow)?;
        let relation = Relation::new(id, name_id, from, to, kind);
        push_or_overflow(&mut self.relations, relation)?;
        Ok(id)
    }

    /// Look up a [`Relation`] by id.
    #[must_use]
    pub fn relation(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get_by_id(id)
    }

    /// Resolve a relation by its interned name. Runs in `O(n_relations)`.
    #[must_use]
    pub fn relation_by_name(&self, name: &str) -> Option<&Relation> {
        let name_id = self.strings.get_id(name)?;
        find_by_name(&self.relations, |r| r.name, name_id)
    }

    // ─── Lookup ────────────────────────────────────────────────────────────

    /// Intern `name`, allocate a fresh [`LookupId`], and push a
    /// [`Lookup`] over `fields` on `entity`.
    pub fn define_lookup(
        &mut self,
        name: &str,
        entity: EntityId,
        fields: alloc::vec::Vec<FieldId>,
        unique: bool,
    ) -> Result<LookupId, CatalogError> {
        let name_id = self.strings.intern(name)?;
        let id = next_id::<_>(self.lookups.len()).ok_or(CatalogError::PoolOverflow)?;
        let lookup = Lookup::new(id, name_id, entity, fields, unique);
        push_or_overflow(&mut self.lookups, lookup)?;
        Ok(id)
    }

    /// Look up a [`Lookup`] by id.
    #[must_use]
    pub fn lookup(&self, id: LookupId) -> Option<&Lookup> {
        self.lookups.get_by_id(id)
    }

    /// Resolve a lookup by its interned name. Runs in `O(n_lookups)`.
    #[must_use]
    pub fn lookup_by_name(&self, name: &str) -> Option<&Lookup> {
        let name_id = self.strings.get_id(name)?;
        find_by_name(&self.lookups, |lu| lu.name, name_id)
    }

    // ─── Policy ────────────────────────────────────────────────────────────

    /// Intern `name`, allocate a fresh [`PolicyId`], and push a
    /// [`Policy`] of the given [`PolicyKind`].
    pub fn define_policy(
        &mut self,
        name: &str,
        kind: PolicyKind,
    ) -> Result<PolicyId, CatalogError> {
        let name_id = self.strings.intern(name)?;
        let id = next_id::<_>(self.policies.len()).ok_or(CatalogError::PoolOverflow)?;
        let policy = Policy::new(id, name_id, kind);
        push_or_overflow(&mut self.policies, policy)?;
        Ok(id)
    }

    /// Look up a [`Policy`] by id.
    #[must_use]
    pub fn policy(&self, id: PolicyId) -> Option<&Policy> {
        self.policies.get_by_id(id)
    }

    /// Resolve a policy by its interned name. Runs in `O(n_policies)`.
    #[must_use]
    pub fn policy_by_name(&self, name: &str) -> Option<&Policy> {
        let name_id = self.strings.get_id(name)?;
        find_by_name(&self.policies, |p| p.name, name_id)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Compute the next typed id corresponding to the slot that
/// [`DynPool::push`] is about to fill. Returns `None` when the pool
/// is full (`slot_idx >= u32::MAX`).
#[inline]
fn next_id<Tag: ?Sized>(slot_idx: usize) -> Option<Id<Tag>> {
    Id::<Tag>::from_index(slot_idx)
}

/// Push `item` into `pool` and translate the `None` return of
/// [`DynPool::push`] (slot overflow) into [`CatalogError::PoolOverflow`].
#[inline]
fn push_or_overflow<T>(pool: &mut DynPool<T>, item: T) -> Result<(), CatalogError> {
    pool.push(item).ok_or(CatalogError::PoolOverflow).map(|_| ())
}

/// Mutable counterpart of [`DynPool::get_by_id`]. Lives here because
/// the pool side only ships an immutable typed lookup.
#[inline]
fn get_mut_by_id<T, Tag: ?Sized>(pool: &mut DynPool<T>, id: Id<Tag>) -> Option<&mut T> {
    pool.get_mut(id.index())
}

/// Linear scan finding the first record whose `extract`-ed name
/// matches `target`. `O(n)` is fine for the catalog size we expect
/// (sub-thousand entities); a name index can land later without
/// touching the public API.
#[inline]
fn find_by_name<T>(pool: &DynPool<T>, extract: impl Fn(&T) -> StrId, target: StrId) -> Option<&T> {
    for i in 0..pool.len() {
        let item = pool.get(i)?;
        if extract(item) == target {
            return Some(item);
        }
    }
    None
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn new_catalog_is_empty() {
        let cat = SchemaCatalog::new();
        assert!(cat.entities.is_empty());
        assert!(cat.fields.is_empty());
        assert!(cat.relations.is_empty());
        assert!(cat.lookups.is_empty());
        assert!(cat.policies.is_empty());
        assert_eq!(cat.strings.len().unwrap(), 0);
    }

    #[test]
    fn default_matches_new() {
        let a = SchemaCatalog::default();
        let b = SchemaCatalog::new();
        assert_eq!(a.entities.len(), b.entities.len());
        assert_eq!(a.fields.len(), b.fields.len());
    }

    #[test]
    fn define_entity_assigns_one_based_ids() {
        let mut cat = SchemaCatalog::new();
        let a = cat.define_entity("orders").unwrap();
        let b = cat.define_entity("customers").unwrap();
        assert_eq!(a.get(), 1);
        assert_eq!(b.get(), 2);
    }

    #[test]
    fn define_entity_round_trips_through_getters() {
        let mut cat = SchemaCatalog::new();
        let id = cat.define_entity("orders").unwrap();
        let by_id = cat.entity(id).unwrap();
        assert_eq!(by_id.id, id);
        let name_id = cat.strings.get_id("orders").unwrap();
        assert_eq!(by_id.name, name_id);
        let by_name = cat.entity_by_name("orders").unwrap();
        assert_eq!(by_name.id, id);
    }

    #[test]
    fn entity_mut_lets_callers_attach_fields() {
        let mut cat = SchemaCatalog::new();
        let entity = cat.define_entity("orders").unwrap();
        let field = cat
            .define_scalar_field("id", DataType::Int64, false)
            .unwrap();
        cat.entity_mut(entity).unwrap().fields.push(field);
        assert_eq!(cat.entity(entity).unwrap().fields, vec![field]);
    }

    #[test]
    fn entity_by_name_returns_none_for_missing_name() {
        let cat = SchemaCatalog::new();
        assert!(cat.entity_by_name("orders").is_none());
    }

    #[test]
    fn define_entity_interns_name_only_once() {
        let mut cat = SchemaCatalog::new();
        let _ = cat.define_entity("orders").unwrap();
        // Re-interning the same name keeps the pool at len 1.
        let _ = cat.strings.intern("orders").unwrap();
        assert_eq!(cat.strings.len().unwrap(), 1);
    }

    #[test]
    fn define_scalar_field_round_trip() {
        let mut cat = SchemaCatalog::new();
        let id = cat
            .define_scalar_field("total", DataType::Float64, true)
            .unwrap();
        let f = cat.field(id).unwrap();
        assert_eq!(f.id, id);
        assert!(f.nullable);
        assert!(matches!(f.ty, FieldType::Scalar(DataType::Float64)));
        let by_name = cat.field_by_name("total").unwrap();
        assert_eq!(by_name.id, id);
    }

    #[test]
    fn define_field_supports_computed_kind() {
        let mut cat = SchemaCatalog::new();
        let node_id = Id::from_u32(7).unwrap();
        let id = cat
            .define_field("derived", FieldType::Computed { expr: node_id }, false)
            .unwrap();
        let f = cat.field(id).unwrap();
        let FieldType::Computed { expr } = f.ty else {
            panic!("expected Computed");
        };
        assert_eq!(expr, node_id);
    }

    #[test]
    fn define_relation_round_trip() {
        let mut cat = SchemaCatalog::new();
        let o = cat.define_entity("orders").unwrap();
        let c = cat.define_entity("customers").unwrap();
        let id = cat
            .define_relation("placed_by", o, c, RelationKind::OneToMany)
            .unwrap();
        let r = cat.relation(id).unwrap();
        assert_eq!(r.from, o);
        assert_eq!(r.to, c);
        assert_eq!(r.kind, RelationKind::OneToMany);
        let by_name = cat.relation_by_name("placed_by").unwrap();
        assert_eq!(by_name.id, id);
    }

    #[test]
    fn define_lookup_round_trip() {
        let mut cat = SchemaCatalog::new();
        let o = cat.define_entity("orders").unwrap();
        let f1 = cat
            .define_scalar_field("id", DataType::Int64, false)
            .unwrap();
        let f2 = cat
            .define_scalar_field("ts", DataType::Int64, false)
            .unwrap();
        let id = cat
            .define_lookup("pk_orders", o, vec![f1, f2], true)
            .unwrap();
        let lu = cat.lookup(id).unwrap();
        assert_eq!(lu.entity, o);
        assert_eq!(lu.fields, vec![f1, f2]);
        assert!(lu.unique);
        let by_name = cat.lookup_by_name("pk_orders").unwrap();
        assert_eq!(by_name.id, id);
    }

    #[test]
    fn define_policy_round_trip() {
        let mut cat = SchemaCatalog::new();
        let id = cat.define_policy("readers", PolicyKind::Read).unwrap();
        let p = cat.policy(id).unwrap();
        assert_eq!(p.id, id);
        assert_eq!(p.kind, PolicyKind::Read);
        let by_name = cat.policy_by_name("readers").unwrap();
        assert_eq!(by_name.id, id);
    }

    #[test]
    fn catalog_error_displays_and_converts_from_intern() {
        let err = CatalogError::from(InternError::CapacityExceeded);
        assert!(matches!(err, CatalogError::InternFailed(_)));
        let msg = alloc::format!("{err}");
        assert!(msg.starts_with("schema catalog:"));

        let overflow = CatalogError::PoolOverflow;
        let msg2 = alloc::format!("{overflow}");
        assert!(msg2.contains("pool overflow"));
    }

    #[test]
    fn names_share_a_single_interner_across_record_kinds() {
        let mut cat = SchemaCatalog::new();
        let e = cat.define_entity("orders").unwrap();
        // Defining a *field* called "orders" reuses the same StrId
        // because both record kinds drain into the shared interner.
        let f = cat
            .define_scalar_field("orders", DataType::Int64, false)
            .unwrap();
        let entity = cat.entity(e).unwrap();
        let field = cat.field(f).unwrap();
        assert_eq!(entity.name, field.name);
        assert_eq!(cat.strings.len().unwrap(), 1);
    }

    #[test]
    fn missing_id_lookups_return_none() {
        let cat = SchemaCatalog::new();
        let missing: EntityId = Id::from_u32(99).unwrap();
        assert!(cat.entity(missing).is_none());
        let missing_field: FieldId = Id::from_u32(99).unwrap();
        assert!(cat.field(missing_field).is_none());
    }
}

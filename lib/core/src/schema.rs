//! Schema and field *handle* types — the small, zero-dep newtypes and
//! enums used as IR addressing primitives.
//!
//! These types are deliberately kept in `dol-core` (rather than
//! `dol-schema`) so that downstream IR crates (`dol-command`,
//! `dol-query`, …) can address into a schema catalog without taking a
//! dependency on the heavier `dol-schema` crate that owns the catalog
//! *storage* (`SchemaCatalog`, `Entity`, `Field`, `DataType`, …).
//!
//! - **Schema handles** ([`SchemaRef`], [`SchemaId`], [`CatalogId`]) —
//!   `(catalog, schema)` pair that resolves to a schema body inside a
//!   `Program::schema_catalog`.
//! - **Type-body classification** ([`TypeBody`]) — enum/composite/distinct
//!   tag attached to named-type entries.
//! - **Field-level reference primitives** ([`ComputedKind`],
//!   [`RefAction`], [`RelationRef`]) — referenced by `dol-command`'s
//!   `FieldDef` (`generated`, `references`).
//!
//! `dol-schema` re-exports the same names so downstream code that wrote
//! `use dol_schema::SchemaRef` keeps working.

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;

// ── Catalog handles ─────────────────────────────────────────────────────────

/// Catalog identifier.
///
/// A program can address multiple catalogs (e.g. a "core" catalog plus
/// per-extension catalogs). The default `0` is the program's own embedded
/// catalog.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CatalogId(
    /// Raw catalog index.
    pub u32,
);

impl CatalogId {
    /// The program's own embedded catalog.
    pub const SELF: CatalogId = CatalogId(0);

    /// Construct a catalog id from a raw index.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Underlying raw catalog index.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for CatalogId {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

/// Schema identifier within a catalog.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SchemaId(
    /// Raw schema index within the catalog.
    pub u32,
);

impl SchemaId {
    /// Construct a schema id from a raw index.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Underlying raw schema index.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for SchemaId {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

/// Reference to a schema in a catalog.
///
/// The pair `(catalog, schema)` resolves to a schema body inside a
/// Program's `schema_catalog`. The schema itself stays in the catalog
/// rather than being inlined into every Operation payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SchemaRef {
    /// Which catalog this schema belongs to.
    pub catalog: CatalogId,
    /// Index within that catalog.
    pub schema: SchemaId,
}

impl SchemaRef {
    /// Construct a schema reference from explicit catalog and schema ids.
    #[inline]
    pub const fn new(catalog: CatalogId, schema: SchemaId) -> Self {
        Self { catalog, schema }
    }

    /// Build a reference into the program's own embedded catalog.
    #[inline]
    pub const fn local(schema: SchemaId) -> Self {
        Self {
            catalog: CatalogId::SELF,
            schema,
        }
    }
}

// ── Type-body classification ────────────────────────────────────────────────

/// Type-body classification for named types.
///
/// Used by `SchemaCatalog` type entries and schema-level DDL operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TypeBody {
    /// Enumerated type (e.g. PostgreSQL `CREATE TYPE ... AS ENUM`).
    Enum,
    /// Composite / record type (e.g. PostgreSQL `CREATE TYPE ... AS (...)`).
    Composite,
    /// Distinct (domain) type with constraints.
    Distinct,
    /// Backend-specific type kind.
    Other,
}

// ── Field-level reference primitives ────────────────────────────────────────

/// Action to take when a referenced record is deleted or updated.
///
/// Each variant has a backend-by-backend mapping:
///
/// | Variant       | SQL                  | Document store          | Graph             |
/// |---------------|----------------------|-------------------------|-------------------|
/// | `Forbid`      | `NO ACTION`          | reject mutation         | reject deletion   |
/// | `Cascade`     | `CASCADE`            | cascade write/delete    | cascade traversal |
/// | `Detach`      | `SET NULL`           | clear referencing field | drop edge         |
/// | `Reject`      | `RESTRICT`           | refuse mutation         | refuse deletion   |
/// | `UseDefault`  | `SET DEFAULT`        | reset to schema default | reset to default  |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum RefAction {
    /// Reject the mutation outright; the reference is treated as a hard
    /// invariant. Equivalent to `NO ACTION` in SQL.
    Forbid,
    /// Propagate the mutation to all referencing records.
    Cascade,
    /// Clear the referencing field on the dependent record. Equivalent to
    /// `SET NULL` in SQL.
    Detach,
    /// Refuse the mutation if any referencing record exists. Equivalent to
    /// `RESTRICT` in SQL — semantically narrower than [`Forbid`](Self::Forbid)
    /// in that it forbids the mutation immediately rather than at commit time.
    Reject,
    /// Reset the referencing field to its declared default. Equivalent to
    /// `SET DEFAULT` in SQL.
    UseDefault,
}

/// How a computed field is materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ComputedKind {
    /// Materialized on write — the value is stored alongside the record and
    /// recomputed only when its inputs change.
    Materialized,
    /// Computed on demand — the value is recomputed every read; the field
    /// occupies no storage of its own.
    OnDemand,
}

/// An inline relation reference on a single field — the dependent side of a
/// directed link to another entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RelationRef {
    /// Target entity name.
    pub entity: Arc<str>,
    /// Target field within `entity`.
    pub field: Arc<str>,
    /// Action to take when the referenced record is deleted.
    pub on_delete: RefAction,
    /// Action to take when the referenced record is updated.
    pub on_update: RefAction,
}

impl RelationRef {
    /// Build a `RelationRef` defaulted to `RefAction::Forbid` on both
    /// `on_delete` and `on_update`.
    pub fn new(entity: impl Into<Arc<str>>, field: impl Into<Arc<str>>) -> Self {
        Self {
            entity: entity.into(),
            field: field.into(),
            on_delete: RefAction::Forbid,
            on_update: RefAction::Forbid,
        }
    }

    /// Set the `on_delete` action.
    pub fn on_delete(mut self, action: RefAction) -> Self {
        self.on_delete = action;
        self
    }

    /// Set the `on_update` action.
    pub fn on_update(mut self, action: RefAction) -> Self {
        self.on_update = action;
        self
    }
}

/// An entity-level constraint (composite uniqueness, multi-field relation,
/// invariant expression, composite identity).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EntityConstraint {
    /// Uniqueness constraint over one or more fields.
    Unique(Vec<Arc<str>>),
    /// A multi-field relation to another entity.
    Relation {
        /// Local fields participating in the relation.
        fields: Vec<Arc<str>>,
        /// Target entity name.
        ref_entity: Arc<str>,
        /// Target fields within `ref_entity`.
        ref_fields: Vec<Arc<str>>,
        /// Action to take when the referenced record is deleted.
        on_delete: RefAction,
    },
    /// A boolean invariant expressed as text — every record must satisfy it.
    Invariant(Arc<str>),
    /// Composite identity constraint — these fields uniquely identify an
    /// entity instance.
    Identity(Vec<Arc<str>>),
}

impl EntityConstraint {
    /// Build a `Unique` constraint from any iterable of string-likes.
    pub fn unique<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::Unique(fields.into_iter().map(Into::into).collect())
    }

    /// Build an `Identity` constraint — the fields that uniquely identify an
    /// entity instance.
    pub fn identity<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::Identity(fields.into_iter().map(Into::into).collect())
    }

    /// Build an `Invariant` constraint — a boolean expression every record
    /// must satisfy.
    pub fn invariant(expr: impl Into<Arc<str>>) -> Self {
        Self::Invariant(expr.into())
    }

    /// Build a multi-field `Relation` constraint.
    pub fn relation<I, J, S, T>(
        fields: I,
        ref_entity: impl Into<Arc<str>>,
        ref_fields: J,
        on_delete: RefAction,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        J: IntoIterator<Item = T>,
        S: Into<Arc<str>>,
        T: Into<Arc<str>>,
    {
        Self::Relation {
            fields: fields.into_iter().map(Into::into).collect(),
            ref_entity: ref_entity.into(),
            ref_fields: ref_fields.into_iter().map(Into::into).collect(),
            on_delete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_uses_catalog_self() {
        let r = SchemaRef::local(SchemaId::new(7));
        assert_eq!(r.catalog, CatalogId::SELF);
        assert_eq!(r.schema.raw(), 7);
    }

    #[test]
    fn raw_round_trips() {
        let c = CatalogId::from(3u32);
        let s = SchemaId::from(9u32);
        assert_eq!(c.raw(), 3);
        assert_eq!(s.raw(), 9);
    }

    #[test]
    fn type_body_variants_compile() {
        let _ = TypeBody::Enum;
        let _ = TypeBody::Composite;
        let _ = TypeBody::Distinct;
        let _ = TypeBody::Other;
    }

    #[test]
    fn relation_ref_builders() {
        let r = RelationRef::new("users", "id")
            .on_delete(RefAction::Cascade)
            .on_update(RefAction::Detach);
        assert_eq!(&*r.entity, "users");
        assert_eq!(&*r.field, "id");
        assert_eq!(r.on_delete, RefAction::Cascade);
        assert_eq!(r.on_update, RefAction::Detach);
    }
}

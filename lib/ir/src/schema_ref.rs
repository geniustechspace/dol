//! Schema catalog handles.
//!
//! Schemas are data, not embedded Rust type walls. A
//! [`Program`](crate::Program) carries an optional `SchemaCatalog`, and
//! operations reference catalog entries by [`SchemaRef`] handle.
//!
//! `CatalogId` and `SchemaId` are both transparent `u32` newtypes so a
//! `SchemaRef` is two integers — cheap to copy, cheap to compare, and
//! trivially serialisable.
//!

/// Catalog identifier.
///
/// A program can address multiple catalogs (e.g. a "core" catalog plus
/// per-extension catalogs). The default `0` is the program's own embedded
/// catalog.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CatalogId(pub u32);

impl CatalogId {
    /// The program's own embedded catalog.
    pub const SELF: CatalogId = CatalogId(0);

    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchemaId(pub u32);

impl SchemaId {
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

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
/// [`Program`](crate::Program)'s `schema_catalog`. The schema itself stays in
/// the catalog rather than being inlined into every
/// [`Operation`](crate::operation::Operation) payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchemaRef {
    pub catalog: CatalogId,
    pub schema: SchemaId,
}

impl SchemaRef {
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
}

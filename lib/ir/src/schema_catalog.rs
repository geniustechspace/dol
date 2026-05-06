//! In-program schema catalog.
//!
//! Schemas live in a catalog rather than being embedded inline in every
//! [`Operation`](crate::operation::Operation). Each catalog entry is keyed by
//! [`SchemaId`] and resolved through a [`SchemaRef`](crate::SchemaRef).
//!
//! The catalog stores [`dol_schema::Entity`] for entity bodies and a small
//! [`TypeEntry`] for named-type bodies; backends can extend the catalog
//! through [`CatalogEntry::Extension`] for backend-specific shapes.

use alloc::vec::Vec;
use smallvec::SmallVec;

use crate::schema_ref::{CatalogId, SchemaId};
use crate::target::Symbol;

extern crate alloc;

/// One entry in a [`SchemaCatalog`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum CatalogEntry {
    /// Entity body (relation / document / KV / blob bucket).
    Entity(dol_schema::Entity),
    /// Named-type body.
    Type(TypeEntry),
    /// Backend-specific body, codec-encoded.
    Extension {
        /// Interned kind tag identifying the extension type. Resolve against
        /// the surrounding program's [`Interner`](dol_expr::Interner).
        kind: Symbol,
        /// Opaque payload bytes.
        payload: Vec<u8>,
    },
}

/// Named-type body stored in the catalog.
///
/// Names and members are interned [`Symbol`]s; resolve against the
/// surrounding program's [`Interner`](dol_expr::Interner). The four-element
/// inline buffer for `members` keeps the common case (small enums) in-line.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypeEntry {
    /// Interned type name.
    pub name: Symbol,
    /// Type classification (enum, composite, distinct).
    pub kind: crate::operation::TypeBody,
    /// Optional list of variants (for enums) or fields (for composites),
    /// each interned. The four-slot inline buffer covers the common case.
    pub members: SmallVec<[Symbol; 4]>,
}

/// Catalog of schemas referenced by a program.
///
/// The catalog is a flat indexed table; lookups are `O(1)` by [`SchemaId`].
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SchemaCatalog {
    /// Identifier of this catalog. Default is [`CatalogId::SELF`].
    pub id: CatalogId,
    entries: Vec<CatalogEntry>,
}

impl SchemaCatalog {
    /// Empty catalog with id [`CatalogId::SELF`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty catalog with a chosen [`CatalogId`].
    pub fn with_id(id: CatalogId) -> Self {
        Self {
            id,
            entries: Vec::new(),
        }
    }

    /// Insert an entry and return its [`SchemaId`].
    pub fn insert(&mut self, entry: CatalogEntry) -> SchemaId {
        let id = SchemaId::new(self.entries.len() as u32);
        self.entries.push(entry);
        id
    }

    /// Resolve a [`SchemaId`].
    pub fn get(&self, id: SchemaId) -> Option<&CatalogEntry> {
        self.entries.get(id.raw() as usize)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if the catalog has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrow the entries as a contiguous slice.
    pub fn entries_slice(&self) -> &[CatalogEntry] {
        &self.entries
    }

    /// Iterate over `(SchemaId, &CatalogEntry)`.
    pub fn iter(&self) -> impl Iterator<Item = (SchemaId, &CatalogEntry)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, e)| (SchemaId::new(i as u32), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut cat = SchemaCatalog::new();
        let id = cat.insert(CatalogEntry::Type(TypeEntry {
            name: Symbol::from_hash(0),
            kind: crate::operation::TypeBody::Enum,
            members: smallvec::smallvec![Symbol::from_hash(1), Symbol::from_hash(2)],
        }));
        assert_eq!(cat.len(), 1);
        assert!(matches!(cat.get(id), Some(CatalogEntry::Type(_))));
    }
}

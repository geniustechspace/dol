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

use crate::schema_ref::{CatalogId, SchemaId};

extern crate alloc;

/// One entry in a [`SchemaCatalog`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CatalogEntry {
    /// Entity body (relation / document / KV / blob bucket).
    Entity(dol_schema::Entity),
    /// Named-type body.
    Type(TypeEntry),
    /// Backend-specific body, codec-encoded.
    Extension {
        /// Kind tag identifying the extension type.
        kind: alloc::string::String,
        /// Opaque payload bytes.
        payload: Vec<u8>,
    },
}

/// Named-type body stored in the catalog.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeEntry {
    /// Type name.
    pub name: alloc::string::String,
    /// Type classification (enum, composite, distinct).
    pub kind: crate::operation::TypeBody,
    /// Optional list of variants (for enums) or fields (for composites).
    pub members: Vec<alloc::string::String>,
}

/// Catalog of schemas referenced by a program.
///
/// The catalog is a flat indexed table; lookups are `O(1)` by [`SchemaId`].
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            name: "status".into(),
            kind: crate::operation::TypeBody::Enum,
            members: vec!["ok".into(), "err".into()],
        }));
        assert_eq!(cat.len(), 1);
        assert!(matches!(cat.get(id), Some(CatalogEntry::Type(_))));
    }
}

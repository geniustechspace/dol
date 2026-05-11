//! `ContentIndex` — lazy node → content-address map.
//!
//! Per `dol-rewrite-plan-v2.md` §7.6. M2 ships the stub; the
//! bottom-up walker that populates it is wired up in M3 when
//! `ExprArena` exists.

#[cfg(feature = "std")]
extern crate alloc;

#[cfg(feature = "std")]
use hashbrown::HashMap;

#[cfg(feature = "std")]
use crate::handle::NodeId;

/// Lazy map from [`NodeId`] to its BLAKE3-128 content address.
///
/// In M3, the `ExprArena` will populate this index bottom-up during
/// node construction or via a deferred walk. Entries can be
/// invalidated when a node is rewritten (e.g. constant folding).
#[derive(Debug, Default, Clone)]
#[cfg(feature = "std")]
pub struct ContentIndex {
    node_hashes: HashMap<NodeId, [u8; 16]>,
}

#[cfg(feature = "std")]
impl ContentIndex {
    /// Construct an empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the content address of `id`, if it has been computed
    /// and not subsequently invalidated.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<[u8; 16]> {
        self.node_hashes.get(&id).copied()
    }

    /// Record `digest` as the content address of `id`.
    pub fn insert(&mut self, id: NodeId, digest: [u8; 16]) {
        self.node_hashes.insert(id, digest);
    }

    /// Drop the cached content address of `id`. Call after a rewrite
    /// or schema-change invalidates the node's structural hash.
    pub fn invalidate(&mut self, id: NodeId) {
        self.node_hashes.remove(&id);
    }

    /// Number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.node_hashes.len()
    }

    /// `true` when no entries are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.node_hashes.is_empty()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn insert_get_round_trip() {
        let mut idx = ContentIndex::new();
        let id = NodeId::from_index(0).unwrap();
        assert_eq!(idx.get(id), None);
        idx.insert(id, [7; 16]);
        assert_eq!(idx.get(id), Some([7; 16]));
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn invalidate_drops_entry() {
        let mut idx = ContentIndex::new();
        let id = NodeId::from_index(3).unwrap();
        idx.insert(id, [1; 16]);
        idx.invalidate(id);
        assert!(idx.get(id).is_none());
        assert!(idx.is_empty());
    }
}

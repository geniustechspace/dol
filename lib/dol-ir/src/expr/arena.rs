//! Raw [`ExprArena`] — a `DynPool<ExprNode>`-backed flat node store
//! with an optional structural-dedup index.
//!
//! Per `dol-rewrite-plan-v2.md` §8.1 + §8.4 (dedup half).
//!
//! ## Two construction modes
//!
//! - [`ExprArena::push`] — raw, append-only. Always allocates a fresh
//!   slot, returns a fresh [`NodeId`]. Use when callers control
//!   uniqueness themselves (e.g. when each node already references
//!   distinct child ids).
//! - [`ExprArena::intern_node`] — checks the dedup index first.
//!   Returns an existing [`NodeId`] when the byte-identical node is
//!   already present, otherwise pushes and records the new id. The
//!   index is keyed by [`dol_core::hash::fast64`] over the 16 raw
//!   bytes of [`ExprNode`]; collisions are handled by comparing the
//!   actual node bytes after a hash hit.
//!
//! The two modes interoperate: nodes inserted via `push` are never
//! recorded in the dedup index (so a later `intern_node` of the same
//! value will allocate a new slot), but they remain reachable by id
//! and may be referenced by interned children. Mixing modes is
//! intentionally unsurprising — the dedup index is a pure cache that
//! never causes existing ids to disappear.

#[cfg(feature = "std")]
use dol_core::hash::fast64;

#[cfg(feature = "std")]
use dol_cas::handle::NodeId;
#[cfg(feature = "std")]
use dol_cas::pool::{ArenaStorage, DynPool};

#[cfg(feature = "std")]
use hashbrown::HashMap;

#[cfg(feature = "std")]
use super::node::ExprNode;

/// Flat arena of [`ExprNode`]s addressed by [`NodeId`].
///
/// Backed by [`DynPool<ExprNode>`](dol_cas::pool::DynPool); `Send +
/// Sync` since `ExprNode` is `Pod`. The arena is **monotonically
/// growing** — once a node is pushed its [`NodeId`] is stable for
/// the lifetime of the arena.
#[cfg(feature = "std")]
#[derive(Debug, Default, Clone)]
pub struct ExprArena {
    pool: DynPool<ExprNode>,
    /// Lazy structural-dedup index: `fast64(bytes(node)) -> NodeId`.
    ///
    /// Built on first `intern_node` call; never populated by `push`.
    /// `fast64` collisions are resolved by comparing the raw 16-byte
    /// `ExprNode` slabs of the candidate and the existing entry.
    /// On a true collision the candidate wins a fresh slot — the
    /// index simply forgets the older entry. (Both ids remain valid;
    /// future `intern_node` of the older value will then re-allocate.
    /// `fast64` collisions on identical 16-byte inputs are vanishingly
    /// rare; this strategy is the simple, panic-free choice.)
    dedup: HashMap<u64, NodeId>,
}

#[cfg(feature = "std")]
impl ExprArena {
    /// Construct an empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pool: DynPool::new(),
            dedup: HashMap::new(),
        }
    }

    /// Pre-allocate capacity for `cap` nodes (and the same capacity
    /// in the dedup index — the upper bound on distinct entries).
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pool: DynPool::with_capacity(cap),
            dedup: HashMap::with_capacity(cap),
        }
    }

    /// Push a node and return its [`NodeId`].
    ///
    /// Returns `None` only when the arena would exceed
    /// [`u32::MAX`] nodes (the [`NodeId`] niche limit).
    ///
    /// The dedup index is **not** consulted or updated. Use
    /// [`Self::intern_node`] for structural sharing.
    pub fn push(&mut self, node: ExprNode) -> Option<NodeId> {
        self.pool.push_id(node)
    }

    /// Intern a node — return its existing id if a byte-identical
    /// node is already in the arena, otherwise push and record it.
    ///
    /// Returns `None` only when the arena would exceed [`u32::MAX`]
    /// nodes (the [`NodeId`] niche limit).
    pub fn intern_node(&mut self, node: ExprNode) -> Option<NodeId> {
        let bytes: [u8; 16] = bytemuck::cast(node);
        let key = fast64(&bytes);
        if let Some(&existing) = self.dedup.get(&key) {
            // Confirm byte equality — `fast64` is not collision-free.
            if let Some(prev) = self.pool.get_by_id(existing) {
                if prev == &node {
                    return Some(existing);
                }
            }
            // Collision (or stale entry): fall through and allocate
            // a fresh slot, then overwrite the index entry below.
        }
        let id = self.pool.push_id(node)?;
        self.dedup.insert(key, id);
        Some(id)
    }

    /// Borrow a node by id.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&ExprNode> {
        self.pool.get_by_id(id)
    }

    /// Number of nodes in the arena.
    ///
    /// This is the total number of slots — interned and `push`ed
    /// alike — not the number of distinct dedup entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// `true` when the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }

    /// Number of entries currently tracked in the dedup index.
    /// Always `<= len()`. Test/diagnostic helper.
    #[must_use]
    pub fn dedup_len(&self) -> usize {
        self.dedup.len()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::expr::node::ExprNode;
    use crate::expr::ops::{BinOp, UnaryOp};
    use dol_cas::handle::Lid;

    fn nid(i: u32) -> NodeId {
        Lid::from_u32(i).unwrap()
    }

    #[test]
    fn empty_arena_round_trip() {
        let a = ExprArena::new();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
        assert_eq!(a.dedup_len(), 0);
    }

    #[test]
    fn push_and_get() {
        let mut a = ExprArena::new();
        let n0 = ExprNode::param(0);
        let n1 = ExprNode::param(1);
        let id0 = a.push(n0).unwrap();
        let id1 = a.push(n1).unwrap();
        assert_ne!(id0, id1);
        assert_eq!(a.len(), 2);
        assert_eq!(a.get(id0), Some(&n0));
        assert_eq!(a.get(id1), Some(&n1));
    }

    #[test]
    fn ids_are_dense_and_one_based() {
        let mut a = ExprArena::new();
        let id0 = a.push(ExprNode::wildcard()).unwrap();
        let id1 = a.push(ExprNode::count_all()).unwrap();
        assert_eq!(id0.index(), 0);
        assert_eq!(id1.index(), 1);
        assert_eq!(id0.get(), 1);
        assert_eq!(id1.get(), 2);
    }

    #[test]
    fn nested_node_references() {
        // Build (a + b) where a, b are wildcard sentinels — the arena
        // doesn't enforce semantic correctness, only structural.
        let mut arena = ExprArena::new();
        let left = arena.push(ExprNode::wildcard()).unwrap();
        let right = arena.push(ExprNode::wildcard()).unwrap();
        let sum = arena.push(ExprNode::bin(BinOp::Add, left, right)).unwrap();
        let (op, l, r) = arena.get(sum).unwrap().as_bin().unwrap();
        assert_eq!(op, BinOp::Add);
        assert_eq!(l, left);
        assert_eq!(r, right);
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let a = ExprArena::new();
        assert!(a.get(nid(1)).is_none());
    }

    // ─── intern_node / dedup ─────────────────────────────────────────

    #[test]
    fn intern_returns_same_id_for_byte_identical_node() {
        let mut a = ExprArena::new();
        let n = ExprNode::param(7);
        let id1 = a.intern_node(n).unwrap();
        let id2 = a.intern_node(n).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(a.len(), 1);
        assert_eq!(a.dedup_len(), 1);
    }

    #[test]
    fn intern_distinct_nodes_get_distinct_ids() {
        let mut a = ExprArena::new();
        let id1 = a.intern_node(ExprNode::param(1)).unwrap();
        let id2 = a.intern_node(ExprNode::param(2)).unwrap();
        let id3 = a.intern_node(ExprNode::wildcard()).unwrap();
        let id4 = a.intern_node(ExprNode::count_all()).unwrap();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id3, id4);
        assert_eq!(a.len(), 4);
        assert_eq!(a.dedup_len(), 4);
    }

    #[test]
    fn intern_dedups_structural_subtrees() {
        // (x + y) where both children are interned wildcards — the
        // shared `wildcard()` constant collapses to a single id.
        let mut a = ExprArena::new();
        let l1 = a.intern_node(ExprNode::wildcard()).unwrap();
        let l2 = a.intern_node(ExprNode::wildcard()).unwrap();
        assert_eq!(l1, l2);
        let sum = a.intern_node(ExprNode::bin(BinOp::Add, l1, l2)).unwrap();
        // Re-intern the same binary should find the same id.
        let sum2 = a.intern_node(ExprNode::bin(BinOp::Add, l1, l2)).unwrap();
        assert_eq!(sum, sum2);
        // Different operator → different id, even with same operands.
        let prod = a.intern_node(ExprNode::bin(BinOp::Mul, l1, l2)).unwrap();
        assert_ne!(sum, prod);
        // Total: 1 wildcard + 1 add + 1 mul.
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn flag_difference_makes_distinct_dedup_keys() {
        use crate::expr::flags::NodeFlags;
        let mut a = ExprArena::new();
        let mut n0 = ExprNode::unary(UnaryOp::Not, nid(1));
        let mut n1 = ExprNode::unary(UnaryOp::Not, nid(1));
        n1.set_flags(NodeFlags::new().with_negated(true));
        let id0 = a.intern_node(n0).unwrap();
        let id1 = a.intern_node(n1).unwrap();
        assert_ne!(id0, id1);
        // And re-interning n0 still finds the original.
        let id0_again = a.intern_node(n0).unwrap();
        assert_eq!(id0, id0_again);
        // Touch the binding to keep clippy happy on no-op mutation.
        n0.set_flags(n0.flags());
    }

    #[test]
    fn push_does_not_populate_dedup_index() {
        let mut a = ExprArena::new();
        let n = ExprNode::param(5);
        let pushed = a.push(n).unwrap();
        // Now intern the same value: dedup is cold so we get a *new*
        // slot. This is the documented mode-mixing behaviour.
        let interned = a.intern_node(n).unwrap();
        assert_ne!(pushed, interned);
        assert_eq!(a.len(), 2);
        assert_eq!(a.dedup_len(), 1);
        // But re-interning hits the cache.
        let again = a.intern_node(n).unwrap();
        assert_eq!(again, interned);
    }
}

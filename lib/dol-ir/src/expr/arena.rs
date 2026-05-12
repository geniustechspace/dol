//! Raw [`ExprArena`] — a `DynPool<ExprNode>`-backed flat node store.
//!
//! Per `dol-rewrite-plan-v2.md` §8.1.
//!
//! M3a ships only the raw push/get surface. The dedup index, deep-walk
//! helpers, and bottom-up content-address walker land in M3b together
//! with `Expr<'a>` lowering — until then there is no caller that can
//! produce duplicate nodes worth deduplicating.

#[cfg(feature = "std")]
use dol_cas::handle::NodeId;
#[cfg(feature = "std")]
use dol_cas::pool::{ArenaStorage, DynPool};

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
}

#[cfg(feature = "std")]
impl ExprArena {
    /// Construct an empty arena.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pool: DynPool::new(),
        }
    }

    /// Pre-allocate capacity for `cap` nodes.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pool: DynPool::with_capacity(cap),
        }
    }

    /// Push a node and return its [`NodeId`].
    ///
    /// Returns `None` only when the arena would exceed
    /// [`u32::MAX`] nodes (the [`NodeId`] niche limit).
    pub fn push(&mut self, node: ExprNode) -> Option<NodeId> {
        self.pool.push_id(node)
    }

    /// Borrow a node by id.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&ExprNode> {
        self.pool.get_by_id(id)
    }

    /// Number of nodes in the arena.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// `true` when the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::expr::node::ExprNode;
    use crate::expr::ops::BinOp;
    use dol_cas::handle::Lid;

    fn nid(i: u32) -> NodeId {
        Lid::from_u32(i).unwrap()
    }

    #[test]
    fn empty_arena_round_trip() {
        let a = ExprArena::new();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
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
}

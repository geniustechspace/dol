//! `ContextPool` — typed-arena wrapper that maps [`ContextId`] to
//! the lowered scope-context.
//!
//! Per `dol-rewrite-plan-v2.md` §3.3 ("side pools") and §8.4 (lowering
//! of `Expr::Scoped`).
//!
//! ## Role in the lowering pipeline
//!
//! `Expr::Scoped { expr, context: Context<'a> }` (the tree-DSL form)
//! lowers to `ExprNode::scoped(NodeId, ContextId)` (the arena form).
//! [`Context<'a>`](crate::expr::context::Context) carries `Vec<Expr>`
//! lists for partition-by and order-by plus an optional [`Frame`], so
//! it cannot be inlined into the 16-byte
//! [`ExprNode`](crate::expr::node::ExprNode); the lowering pipeline
//! interns the lowered representation here and stores a
//! [`ContextId`] in the node.
//!
//! ## What this slice ships
//!
//! - [`LoweredOrderByExpr`] — the arena form of
//!   [`OrderByExpr`](crate::expr::order::OrderByExpr) with the
//!   `expr` field replaced by an interned [`NodeId`].
//! - [`LoweredContext`] — the arena form of
//!   [`Context`](crate::expr::context::Context).
//! - Push-only intern API; structural dedup is deferred (the lowered
//!   shape contains `Vec`s that need a deliberate canonical hash).

extern crate alloc;

use alloc::vec::Vec;

use dol_cas::handle::{ContextId, NodeId};
use dol_cas::pool::{ArenaStorage, DynPool};

use crate::expr::frame::Frame;
use crate::expr::order::{NullsOrder, SortDirection};

/// Arena form of [`OrderByExpr`](crate::expr::order::OrderByExpr) —
/// the `expr` payload has been lowered to a [`NodeId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoweredOrderByExpr {
    /// Lowered ordering expression.
    pub expr: NodeId,
    /// Sort direction.
    pub dir: SortDirection,
    /// Null-ordering rule.
    pub nulls: NullsOrder,
}

impl LoweredOrderByExpr {
    /// Convenience constructor with backend-default ordering.
    #[inline]
    #[must_use]
    pub const fn new(expr: NodeId) -> Self {
        Self {
            expr,
            dir: SortDirection::Asc,
            nulls: NullsOrder::Default,
        }
    }
}

/// Arena form of [`Context<'a>`](crate::expr::context::Context). All
/// `Expr<'a>` payloads have been lowered to arena
/// [`NodeId`](dol_cas::handle::NodeId)s.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredContext {
    /// Partitioning keys (lowered). Empty means "single partition".
    pub partition_by: Vec<NodeId>,
    /// Ordering applied within each partition.
    pub order_by: Vec<LoweredOrderByExpr>,
    /// Optional bounded frame within the ordered partition.
    pub frame: Option<Frame>,
}

impl LoweredContext {
    /// An empty lowered context: no partitioning, no ordering, no
    /// frame. Backends typically elide this during emission.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: None,
        }
    }

    /// Returns `true` iff the context carries no partitioning, no
    /// ordering, and no frame.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.partition_by.is_empty() && self.order_by.is_empty() && self.frame.is_none()
    }
}

impl Default for LoweredContext {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

/// Errors returned by [`ContextPool::intern`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextPoolError {
    /// The pool's slot table would overflow [`u32::MAX`] — the
    /// [`ContextId`] niche limit. Mirrors the other side-pool error
    /// shapes so the recursive lowerer composes them uniformly.
    CapacityExceeded,
}

impl core::fmt::Display for ContextPoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("context pool: capacity exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ContextPoolError {}

/// Typed arena of [`LoweredContext`] addressed by [`ContextId`].
/// Append-only; once an id is handed out it is stable for the
/// lifetime of the pool.
#[derive(Debug, Clone)]
pub struct ContextPool {
    items: DynPool<LoweredContext>,
}

impl Default for ContextPool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ContextPool {
    /// Construct an empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: DynPool::new(),
        }
    }

    /// Pre-allocate capacity for `cap` lowered contexts.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: DynPool::with_capacity(cap),
        }
    }

    /// Intern a [`LoweredContext`] — clones into a fresh slot.
    ///
    /// # Errors
    ///
    /// Returns [`ContextPoolError::CapacityExceeded`] when the pool
    /// would exceed [`u32::MAX`] entries.
    pub fn intern(&mut self, ctx: &LoweredContext) -> Result<ContextId, ContextPoolError> {
        self.items
            .push_id::<dol_cas::handle::tags::ContextTag>(ctx.clone())
            .ok_or(ContextPoolError::CapacityExceeded)
    }

    /// Borrow a lowered context by id.
    #[must_use]
    pub fn get(&self, id: ContextId) -> Option<&LoweredContext> {
        self.items.get_by_id(id)
    }

    /// Number of lowered contexts currently in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` when the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_cas::handle::Lid;

    fn nid(i: u32) -> NodeId {
        Lid::from_u32(i).unwrap()
    }

    #[test]
    fn empty_pool_round_trip() {
        let pool = ContextPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn lowered_context_default_matches_empty() {
        assert_eq!(LoweredContext::default(), LoweredContext::empty());
    }

    #[test]
    fn intern_and_get_round_trip() {
        let mut pool = ContextPool::new();
        let ctx = LoweredContext {
            partition_by: alloc::vec![nid(1)],
            order_by: alloc::vec![LoweredOrderByExpr::new(nid(2))],
            frame: None,
        };
        let id = pool.intern(&ctx).unwrap();
        assert_eq!(pool.get(id), Some(&ctx));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn lowered_order_by_new_uses_defaults() {
        let o = LoweredOrderByExpr::new(nid(7));
        assert_eq!(o.dir, SortDirection::Asc);
        assert_eq!(o.nulls, NullsOrder::Default);
    }

    #[test]
    fn empty_context_is_empty_predicate() {
        assert!(LoweredContext::empty().is_empty());
        let ctx = LoweredContext {
            partition_by: alloc::vec![nid(1)],
            order_by: alloc::vec![],
            frame: None,
        };
        assert!(!ctx.is_empty());
    }

    #[test]
    fn pool_error_display_is_human_readable() {
        let s = alloc::format!("{}", ContextPoolError::CapacityExceeded);
        assert!(s.contains("capacity"));
    }
}

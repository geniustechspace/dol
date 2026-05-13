//! `PathPool` — typed-arena wrapper that maps [`PathId`] to
//! [`Path<StrId>`], with structural dedup.
//!
//! Per `dol-rewrite-plan-v2.md` §3.3 ("side pools") and §8.4 (lowering
//! of `Expr::Ref`).
//!
//! ## Role in the lowering pipeline
//!
//! `Expr::Ref(Path<Name>)` (the tree-DSL form) lowers in two steps:
//!
//! 1. [`crate::expr::lower::lower_path`] interns each segment into the
//!    `StringPool` producing a [`Path<StrId>`] —  the single,
//!    authorised conversion site (plan §8.4 line 1389).
//! 2. The recursive lowerer interns the resulting [`Path<StrId>`] into
//!    [`PathPool`] receiving a typed [`PathId`], then writes
//!    `ExprNode::field_ref(...)` (today via [`crate::expr::node::ExprNode::field_ref`])
//!    using the path id in the node's `a` slot.
//!
//! `Path<StrId>` is variable-length (target + optional namespace + N
//! field segments) so it cannot be inlined into the 16-byte
//! [`ExprNode`](crate::expr::node::ExprNode); the side pool is its
//! required carrier.
//!
//! ## What this slice ships (M3c-δ₂b prereq #4)
//!
//! - **Structural dedup**. Two interns of the same [`Path<StrId>`]
//!   return the **same** [`PathId`]. Unlike
//!   [`LiteralPool`](crate::expr::literals::LiteralPool) (which is
//!   push-only because [`Literal`](dol_core::literal::Literal) lacks
//!   `Hash` due to `f64`), [`Path<StrId>`] is `Hash + Eq` via the
//!   blanket [`PathSegment`](dol_core::path::PathSegment) constraints
//!   plus the `NonZeroU32` niche of [`StrId`]. The dedup keeps the
//!   surrounding [`ExprArena`](crate::expr::arena::ExprArena)
//!   structural-dedup property sound: two `Expr::Ref` calls into the
//!   same path produce byte-identical `ExprNode::field_ref` records.
//! - Typed [`PathId`] handles via [`DynPool::push_id`].
//! - [`PathPoolError::CapacityExceeded`] mirrors
//!   [`LiteralPoolError::CapacityExceeded`](crate::expr::literals::LiteralPoolError),
//!   [`FuncRegistryError::CapacityExceeded`](crate::expr::funcs::FuncRegistryError),
//!   [`OperandSlabError::CapacityExceeded`](crate::expr::slab::OperandSlabError),
//!   and [`LowerError::ArenaOverflow`](crate::expr::lower::LowerError)
//!   so the upcoming recursive lowerer composes them with `?`.
//!
//! ## Out of scope (future slices)
//!
//! - Cross-process content addresses (`to_cid`) for paths.
//! - Garbage collection — paths never disappear once interned.

extern crate alloc;

use hashbrown::HashMap;

use dol_cas::handle::{PathId, StrId};
use dol_cas::pool::{ArenaStorage, DynPool};
use dol_core::path::Path;

/// Errors returned by [`PathPool::intern`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathPoolError {
    /// The pool's slot table would overflow [`u32::MAX`] — the
    /// [`PathId`] niche limit. Mirrors the
    /// [`LiteralPoolError::CapacityExceeded`](crate::expr::literals::LiteralPoolError),
    /// [`FuncRegistryError::CapacityExceeded`](crate::expr::funcs::FuncRegistryError),
    /// [`OperandSlabError::CapacityExceeded`](crate::expr::slab::OperandSlabError),
    /// and [`LowerError::ArenaOverflow`](crate::expr::lower::LowerError)
    /// shapes so the recursive lowerer composes them uniformly.
    CapacityExceeded,
}

impl core::fmt::Display for PathPoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("path pool: capacity exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PathPoolError {}

/// Typed arena of [`Path<StrId>`] addressed by [`PathId`], with
/// structural dedup.
///
/// Backed by [`DynPool<Path<StrId>>`]. `Send + Sync` since
/// [`Path<StrId>`] is `Send + Sync` (it owns a `Box<[StrId]>` and a
/// `Box<StrId>`, both `Send + Sync`).
///
/// The pool is **append-only**: once a [`PathId`] is handed out it is
/// stable for the pool's lifetime, and existing entries are never
/// mutated or removed.
#[derive(Debug, Clone)]
pub struct PathPool {
    items: DynPool<Path<StrId>>,
    /// `Path<StrId> -> PathId` dedup index. The key is cloned in
    /// rather than referenced because `DynPool` may relocate slots on
    /// growth; the [`HashMap`] owns its keys for stability.
    index: HashMap<Path<StrId>, PathId>,
}

impl Default for PathPool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl PathPool {
    /// Construct an empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: DynPool::new(),
            index: HashMap::new(),
        }
    }

    /// Pre-allocate capacity for `cap` distinct paths.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: DynPool::with_capacity(cap),
            index: HashMap::with_capacity(cap),
        }
    }

    /// Intern a path — returns the existing [`PathId`] when a
    /// structurally-equal path is already in the pool, otherwise
    /// pushes a fresh slot and records it in the dedup index.
    ///
    /// Path equality is exact: target, namespace presence/value, and
    /// the full ordered field chain must all match. Two paths that
    /// differ only in field order are **distinct** entries.
    ///
    /// # Errors
    ///
    /// Returns [`PathPoolError::CapacityExceeded`] when the pool
    /// would exceed [`u32::MAX`] distinct entries.
    pub fn intern(&mut self, path: &Path<StrId>) -> Result<PathId, PathPoolError> {
        if let Some(&existing) = self.index.get(path) {
            return Ok(existing);
        }
        let id = self
            .items
            .push_id::<dol_cas::handle::tags::PathTag>(path.clone())
            .ok_or(PathPoolError::CapacityExceeded)?;
        self.index.insert(path.clone(), id);
        Ok(id)
    }

    /// Borrow a path by id.
    #[must_use]
    pub fn get(&self, id: PathId) -> Option<&Path<StrId>> {
        self.items.get_by_id(id)
    }

    /// Look up a path without inserting.
    #[must_use]
    pub fn get_id(&self, path: &Path<StrId>) -> Option<PathId> {
        self.index.get(path).copied()
    }

    /// Number of distinct paths in the pool.
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

    fn sid(n: u32) -> StrId {
        Lid::from_u32(n).unwrap()
    }

    #[test]
    fn empty_pool_round_trip() {
        let pool = PathPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
        let p = Path::from_segment(sid(1));
        assert!(pool.get_id(&p).is_none());
    }

    #[test]
    fn intern_dedups_structurally_equal_paths() {
        let mut pool = PathPool::new();
        let p1 = Path::from_segment(sid(1)).get(sid(2)).get(sid(3));
        let p2 = Path::from_segment(sid(1)).get(sid(2)).get(sid(3));
        let a = pool.intern(&p1).unwrap();
        let b = pool.intern(&p2).unwrap();
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn distinct_paths_get_distinct_ids() {
        let mut pool = PathPool::new();
        let p1 = Path::from_segment(sid(1));
        let p2 = Path::from_segment(sid(2));
        let p3 = Path::from_segment(sid(1)).namespace(sid(9));
        let p4 = Path::from_segment(sid(1)).get(sid(2));
        let a = pool.intern(&p1).unwrap();
        let b = pool.intern(&p2).unwrap();
        let c = pool.intern(&p3).unwrap();
        let d = pool.intern(&p4).unwrap();
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(c, d);
        assert_eq!(pool.len(), 4);
    }

    #[test]
    fn field_order_matters() {
        let mut pool = PathPool::new();
        let ab = Path::from_segment(sid(1)).get(sid(2)).get(sid(3));
        let ba = Path::from_segment(sid(1)).get(sid(3)).get(sid(2));
        let a = pool.intern(&ab).unwrap();
        let b = pool.intern(&ba).unwrap();
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn get_round_trips_an_interned_path() {
        let mut pool = PathPool::new();
        let p = Path::from_segment(sid(7))
            .namespace(sid(8))
            .get(sid(9))
            .get(sid(10));
        let id = pool.intern(&p).unwrap();
        let got = pool.get(id).unwrap();
        assert_eq!(got, &p);
    }

    #[test]
    fn get_id_returns_existing_id() {
        let mut pool = PathPool::new();
        let p = Path::from_segment(sid(1)).get(sid(2));
        let id = pool.intern(&p).unwrap();
        assert_eq!(pool.get_id(&p), Some(id));
        let missing = Path::from_segment(sid(99));
        assert!(pool.get_id(&missing).is_none());
    }

    #[test]
    fn ids_are_dense_and_one_based_via_lid() {
        let mut pool = PathPool::new();
        let id0 = pool.intern(&Path::from_segment(sid(1))).unwrap();
        let id1 = pool.intern(&Path::from_segment(sid(2))).unwrap();
        assert_eq!(id0.index(), 0);
        assert_eq!(id1.index(), 1);
    }

    #[test]
    fn unknown_id_returns_none() {
        let pool = PathPool::new();
        let bogus: PathId = Lid::from_u32(99).unwrap();
        assert!(pool.get(bogus).is_none());
    }

    #[test]
    fn error_display_is_human_readable() {
        let s = alloc::format!("{}", PathPoolError::CapacityExceeded);
        assert!(s.contains("capacity"));
    }
}

//! Generic arena storage — [`ArenaStorage`] trait, [`DynPool`] (alloc),
//! [`StaticPool`] (no-alloc).
//!
//! Per `dol-rewrite-plan-v2.md` §7.3. The `dol-ir` crate (M3) will use
//! a `Pool<ExprNode>` to back `ExprArena`; this module isolates the
//! storage trait so the same node arena can be backed by either
//! `Vec`-grown storage on host or a fixed-size `[T; CAP]` on bare metal.

use crate::handle::Lid;

#[cfg(feature = "std")]
extern crate alloc;

/// Storage backend trait for [`crate::handle::Lid`]-keyed arenas.
///
/// Implementations must guarantee that:
///
/// 1. `push(item)` returns `Some(i)` where `i` is the zero-based index
///    of the just-inserted item; or `None` if capacity is exhausted.
/// 2. `get(i)` returns `Some(&item)` for every `i` returned by a prior
///    `push`, and `None` for any other index.
///
/// Together these guarantee the arena's monotonic, dense-index
/// contract — the same contract [`Lid`] relies on.
pub trait ArenaStorage<T> {
    /// Push a new item; returns its zero-based index, or `None` when
    /// capacity is exhausted.
    fn push(&mut self, item: T) -> Option<usize>;

    /// Borrow the item at the given index.
    fn get(&self, index: usize) -> Option<&T>;

    /// Mutably borrow the item at the given index.
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;

    /// Current number of stored items.
    fn len(&self) -> usize;

    /// `true` when no items are stored.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ─── DynPool — Vec-backed (alloc / std) ─────────────────────────────────────

/// `Vec`-backed dynamic arena. Requires `alloc`.
#[cfg(feature = "std")]
#[derive(Debug, Default, Clone)]
pub struct DynPool<T> {
    items: alloc::vec::Vec<T>,
}

#[cfg(feature = "std")]
impl<T> DynPool<T> {
    /// Construct an empty pool.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: alloc::vec::Vec::new(),
        }
    }

    /// Pre-allocate capacity for `cap` items.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: alloc::vec::Vec::with_capacity(cap),
        }
    }

    /// Push `item` and return its typed [`Lid`].
    ///
    /// Returns `None` when the index would overflow [`u32::MAX`].
    pub fn push_id<Tag>(&mut self, item: T) -> Option<Lid<Tag>> {
        let i = self.push(item)?;
        Lid::<Tag>::from_index(i)
    }

    /// Look up by typed [`Lid`].
    #[must_use]
    pub fn get_by_id<Tag>(&self, id: Lid<Tag>) -> Option<&T> {
        self.get(id.index())
    }
}

#[cfg(feature = "std")]
impl<T> ArenaStorage<T> for DynPool<T> {
    #[inline]
    fn push(&mut self, item: T) -> Option<usize> {
        let i = self.items.len();
        self.items.push(item);
        Some(i)
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }
}

// ─── StaticPool — fixed-size [Option<T>; CAP] ────────────────────────────────

/// Fixed-capacity arena backed by `[Option<T>; CAP]`. Requires no
/// allocator. Used by `iot-min` deployments where pool size is known
/// at compile time.
#[derive(Debug)]
pub struct StaticPool<T, const CAP: usize> {
    items: [Option<T>; CAP],
    len: usize,
}

impl<T, const CAP: usize> StaticPool<T, CAP> {
    /// Construct an empty pool.
    ///
    /// Not a `const fn` because zero-initialising `[Option<T>; CAP]`
    /// for an arbitrary `T` requires either `T: Copy` (excludes most
    /// payload types) or `unsafe` (forbidden workspace-wide).
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: core::array::from_fn(|_| None),
            len: 0,
        }
    }

    /// Returns the configured capacity (`CAP`).
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Push `item` and return its typed [`Lid`], or `None` if full.
    pub fn push_id<Tag>(&mut self, item: T) -> Option<Lid<Tag>> {
        let i = self.push(item)?;
        Lid::<Tag>::from_index(i)
    }

    /// Look up by typed [`Lid`].
    #[must_use]
    pub fn get_by_id<Tag>(&self, id: Lid<Tag>) -> Option<&T> {
        self.get(id.index())
    }
}

impl<T, const CAP: usize> Default for StaticPool<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAP: usize> ArenaStorage<T> for StaticPool<T, CAP> {
    fn push(&mut self, item: T) -> Option<usize> {
        if self.len >= CAP {
            return None;
        }
        let i = self.len;
        let slot = self.items.get_mut(i)?;
        *slot = Some(item);
        // `self.len < CAP <= usize::MAX` so the increment cannot overflow.
        #[allow(clippy::arithmetic_side_effects)]
        {
            self.len += 1;
        }
        Some(i)
    }

    fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        self.items.get(index).and_then(Option::as_ref)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        self.items.get_mut(index).and_then(Option::as_mut)
    }

    fn len(&self) -> usize {
        self.len
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    enum FooTag {}

    #[test]
    fn dyn_pool_push_and_get_round_trip() {
        let mut p: DynPool<u32> = DynPool::new();
        let a = p.push_id::<FooTag>(10).unwrap();
        let b = p.push_id::<FooTag>(20).unwrap();
        assert_eq!(p.get_by_id(a), Some(&10));
        assert_eq!(p.get_by_id(b), Some(&20));
        assert_eq!(p.len(), 2);
        assert_eq!(a.index(), 0);
        assert_eq!(b.index(), 1);
    }

    #[test]
    fn static_pool_full_returns_none() {
        let mut p: StaticPool<u32, 2> = StaticPool::new();
        assert!(p.push_id::<FooTag>(1).is_some());
        assert!(p.push_id::<FooTag>(2).is_some());
        assert!(p.push_id::<FooTag>(3).is_none());
        assert_eq!(p.len(), 2);
        assert_eq!(p.capacity(), 2);
    }

    #[test]
    fn static_pool_get_oob_returns_none() {
        let mut p: StaticPool<u32, 4> = StaticPool::new();
        p.push(42).unwrap();
        assert_eq!(p.get(0), Some(&42));
        assert_eq!(p.get(1), None);
        assert_eq!(p.get(4), None);
    }
}

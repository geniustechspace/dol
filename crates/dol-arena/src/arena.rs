use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::id::Id;

/// Generic typed arena: a contiguous `Vec<T>` keyed by [`Id<T, Tag>`].
///
/// The `Tag` parameter lets two arenas with the same payload type expose
/// distinct, non-interchangeable id types — useful when you have, e.g.,
/// "input" and "output" pools of the same struct. Defaults to `Tag = T`.
///
/// All operations are O(1). Ids are stable for the lifetime of the arena
/// (there is no removal API; rebuild if you need compaction).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arena<T, Tag: ?Sized = T> {
    items: Vec<T>,
    #[cfg_attr(feature = "serde", serde(skip))]
    _tag: PhantomData<fn() -> *const Tag>,
}

impl<T, Tag: ?Sized> Default for Arena<T, Tag> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Tag: ?Sized> Arena<T, Tag> {
    /// Create an empty arena. Allocates nothing.
    #[inline]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            _tag: PhantomData,
        }
    }

    /// Create an arena pre-allocated for `cap` items. Use this on hot paths
    /// where the final size is known (or a good upper bound is known).
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: Vec::with_capacity(cap),
            _tag: PhantomData,
        }
    }

    /// Allocate a value, returning its [`Id`]. O(1) amortised.
    #[inline]
    pub fn allocate(&mut self, item: T) -> Id<T, Tag> {
        let raw = self.items.len() as u32;
        debug_assert!(raw != crate::id::NULL_ID, "arena overflow");
        self.items.push(item);
        Id::from_raw(raw)
    }

    /// Borrow the value referenced by `id`. Panics if `id` is out of bounds.
    #[inline]
    #[allow(clippy::needless_pass_by_value)]
    pub fn get(&self, id: Id<T, Tag>) -> &T {
        &self.items[id.to_raw() as usize]
    }

    /// Mutably borrow the value referenced by `id`.
    #[inline]
    #[allow(clippy::needless_pass_by_value)]
    pub fn get_mutable(&mut self, id: Id<T, Tag>) -> &mut T {
        &mut self.items[id.to_raw() as usize]
    }

    /// Try to borrow the value referenced by `id`. Returns `None` for null or
    /// out-of-bounds ids.
    #[inline]
    #[allow(clippy::needless_pass_by_value)]
    pub fn try_get(&self, id: Id<T, Tag>) -> Option<&T> {
        if id.is_null() {
            return None;
        }
        self.items.get(id.to_raw() as usize)
    }

    /// Number of items in the arena.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` if no items have been allocated.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Remove every item, retaining capacity.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Iterate `(id, &item)` in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (Id<T, Tag>, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, v)| (Id::from_raw(i as u32), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    struct Tag;

    #[test]
    fn id_is_4_bytes() {
        assert_eq!(size_of::<Id<u8, ()>>(), 4);
        assert_eq!(size_of::<Id<u64, Tag>>(), 4);
    }

    #[test]
    fn allocate_get_roundtrip() {
        let mut a: Arena<u32> = Arena::new();
        let i0 = a.allocate(10);
        let i1 = a.allocate(20);
        assert_eq!(*a.get(i0), 10);
        assert_eq!(*a.get(i1), 20);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn try_get_nullable() {
        let a: Arena<u8> = Arena::new();
        assert!(a.try_get(Id::from_raw(crate::NULL_ID)).is_none());
        assert!(a.try_get(Id::from_raw(0)).is_none());
    }
}

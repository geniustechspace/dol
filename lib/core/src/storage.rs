//! # `storage` — abstract backing store for arenas.
//!
//! Most `dol-*` arenas are built on top of `Vec<T>` today, which works
//! everywhere `alloc` is available but is the wrong primitive on the
//! tightest embedded targets where the workspace also wants to run.
//!
//! The [`Storage`] trait is the seam: an arena that takes `S: Storage<T>`
//! can be backed by `Vec<T>` on hosts and by `heapless::Vec<T, N>` /
//! `arrayvec::ArrayVec<T, N>` on bare-metal targets, without changing
//! the arena's API surface.
//!
//! This module ships the [`Vec<T>`] impl. Bare-metal impls live in the
//! crates that need them, behind their own optional feature flags, so
//! `dol-core` does not pull in `heapless` for hosts that will never use
//! it.
//!
//! ## Surface
//!
//! [`Storage`] is intentionally minimal: push, get, get_mut, len, and an
//! iterator. Anything an arena needs beyond that — slice views,
//! capacity hints, retain — should be added as a separate, narrower
//! trait so that the embedded impls do not have to perform those
//! operations.

use alloc::vec::Vec;

/// Append-only random-access container abstraction.
///
/// Implementors guarantee:
///
/// - `len()` is the number of items previously pushed minus any that
///   were never pushed (no removal API is exposed).
/// - `get(i)` returns `Some(&T)` exactly when `i < len()`.
/// - Indices remain stable for the lifetime of the storage; `push`
///   never invalidates an index returned for an earlier element.
///
/// The trait is deliberately permissive about errors: the only fallible
/// operation is [`Storage::push`], which returns `Result<(), Self::Error>`
/// so bounded-capacity backends can refuse without panicking. Hosts that
/// use `Vec` get an `Error = core::convert::Infallible` impl and can
/// `.unwrap()` cleanly.
pub trait Storage<T> {
    /// Reason a [`Storage::push`] was refused.
    type Error;

    /// Number of items currently stored.
    fn len(&self) -> usize;

    /// Whether the storage is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Append `value`, returning its zero-based index on success.
    ///
    /// Bounded backends return `Err` once full; growable backends always
    /// succeed unless the underlying allocator does.
    fn push(&mut self, value: T) -> Result<usize, Self::Error>;

    /// Borrow the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&T>;

    /// Mutably borrow the element at `index`, or `None` if out of
    /// bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
}

/// Default impl: `Vec` is unbounded on hosts.
impl<T> Storage<T> for Vec<T> {
    type Error = core::convert::Infallible;

    #[inline]
    fn len(&self) -> usize {
        Vec::len(self)
    }

    #[inline]
    fn push(&mut self, value: T) -> Result<usize, Self::Error> {
        let index = self.len();
        Vec::push(self, value);
        Ok(index)
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn vec_push_returns_sequential_indices() {
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(<Vec<u8> as Storage<u8>>::push(&mut v, 10).unwrap(), 0);
        assert_eq!(<Vec<u8> as Storage<u8>>::push(&mut v, 20).unwrap(), 1);
        assert_eq!(<Vec<u8> as Storage<u8>>::push(&mut v, 30).unwrap(), 2);
        assert_eq!(<Vec<u8> as Storage<u8>>::len(&v), 3);
        assert!(!<Vec<u8> as Storage<u8>>::is_empty(&v));
    }

    #[test]
    fn vec_get_and_get_mut_round_trip() {
        let mut v: Vec<u32> = vec![1, 2, 3];
        assert_eq!(<Vec<u32> as Storage<u32>>::get(&v, 0), Some(&1));
        assert_eq!(<Vec<u32> as Storage<u32>>::get(&v, 2), Some(&3));
        assert_eq!(<Vec<u32> as Storage<u32>>::get(&v, 3), None);

        if let Some(slot) = <Vec<u32> as Storage<u32>>::get_mut(&mut v, 1) {
            *slot = 42;
        }
        assert_eq!(<Vec<u32> as Storage<u32>>::get(&v, 1), Some(&42));
    }

    #[test]
    fn empty_vec_reports_empty() {
        let v: Vec<()> = Vec::new();
        assert!(<Vec<()> as Storage<()>>::is_empty(&v));
        assert_eq!(<Vec<()> as Storage<()>>::len(&v), 0);
    }

    /// A toy bounded backend exercises the `Error` associated type and
    /// proves the trait is genuinely usable from outside `Vec`.
    #[derive(Default)]
    struct Bounded4<T> {
        items: [Option<T>; 4],
        len: usize,
    }

    #[derive(Debug, PartialEq)]
    struct Full;

    impl<T> Storage<T> for Bounded4<T> {
        type Error = Full;
        fn len(&self) -> usize {
            self.len
        }
        fn push(&mut self, value: T) -> Result<usize, Self::Error> {
            if self.len >= self.items.len() {
                return Err(Full);
            }
            let idx = self.len;
            self.items[idx] = Some(value);
            self.len += 1;
            Ok(idx)
        }
        fn get(&self, index: usize) -> Option<&T> {
            if index < self.len {
                self.items[index].as_ref()
            } else {
                None
            }
        }
        fn get_mut(&mut self, index: usize) -> Option<&mut T> {
            if index < self.len {
                self.items[index].as_mut()
            } else {
                None
            }
        }
    }

    #[test]
    fn bounded_backend_refuses_when_full() {
        let mut b: Bounded4<u8> = Bounded4::default();
        for i in 0..4 {
            assert_eq!(b.push(i).unwrap(), i as usize);
        }
        assert_eq!(b.push(99), Err(Full));
        assert_eq!(b.len(), 4);
    }
}

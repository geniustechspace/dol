//! `StringPool` — content-addressed string interner.
//!
//! M3 ships a single-locked variant whose byte storage is a vector of
//! per-slot heap allocations (`Vec<Box<[u8]>>`). Compared to M2's
//! shared growable byte buffer, the per-slot boxes have **stable
//! addresses** for the lifetime of the pool — the outer `Vec` may
//! reallocate when growing, but each `Box<[u8]>`'s payload sits at a
//! fixed heap address until the pool is dropped. That stability is
//! what allows [`StringPool::get`] to return `&str` (zero-copy) and is
//! the precondition for [`PathSegment for Lid<StrTag>`](crate::handle::StrId).
//!
//! Sharding remains deferred to a later milestone (see plan §7.4):
//! per-shard slot tables would force shard-index bits into the
//! [`StrId`], breaking the simple `Lid::index() == slot_idx` contract.
//! The user-visible API is identical to the planned sharded variant.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use std::sync::RwLock;

use hashbrown::HashMap;

use dol_core::config::PoolConfig;
use dol_core::hash::{content128, fast64_seeded};

use crate::handle::{Cid, Lid, StrCid, StrId};

/// Reason a string failed to intern.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InternError {
    /// Two distinct strings produced the same xxHash3 fingerprint
    /// **and** the byte-confirm step decided to refuse rather than
    /// rehash. The `existing` field carries the [`StrId`] that already
    /// occupies the slot.
    HashCollision {
        /// Pre-existing handle.
        existing: StrId,
    },
    /// Pool capacity exhausted: slot count would exceed [`u32::MAX`].
    CapacityExceeded,
    /// The pool's `RwLock` was poisoned by a panic in another thread.
    Poisoned,
}

impl fmt::Display for InternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HashCollision { existing } => {
                write!(f, "intern: hash collision with existing {existing}")
            }
            Self::CapacityExceeded => f.write_str("intern: capacity exceeded"),
            Self::Poisoned => f.write_str("intern: pool lock was poisoned"),
        }
    }
}

impl std::error::Error for InternError {}

#[derive(Debug, Default)]
struct Inner {
    /// Each slot owns the bytes for one interned string. `Box<[u8]>`
    /// has a stable heap address for as long as the box is not
    /// dropped; we never drop a box until the whole pool is dropped,
    /// so the underlying bytes outlive any borrow against the pool.
    slots: Vec<Box<[u8]>>,
    /// Lazy BLAKE3-128 content addresses, one per slot. `None` until
    /// the first [`StringPool::to_cid`] call for that slot.
    cids: Vec<Option<[u8; 16]>>,
    /// xxHash3(seed, bytes) → slot index. Hash collisions are
    /// resolved by byte-equality against the existing slot's bytes.
    index: HashMap<u64, u32>,
}

/// Content-addressed string interner.
///
/// Concurrent `intern()` calls for the same string from many threads
/// always return the same [`StrId`]. Lookup is via
/// [`fast64_seeded`](dol_core::hash::fast64_seeded) with byte-confirm;
/// cross-process stable content addresses are computed lazily via
/// [`content128`](dol_core::hash::content128) on the first
/// [`StringPool::to_cid`] call for each slot.
#[derive(Debug)]
pub struct StringPool {
    inner: RwLock<Inner>,
    hash_seed: u64,
}

impl StringPool {
    /// Construct a new pool with the given [`PoolConfig`].
    ///
    /// Only `cfg.hash_seed` and `cfg.initial_bytes` are consumed; the
    /// latter is used as a slot-vector capacity hint (one slot per
    /// expected interned string is the worst case).
    #[must_use]
    pub fn new(cfg: &PoolConfig) -> Self {
        // Each slot holds one `Box<[u8]>`; reserve a slot vector
        // capacity proportional to `initial_bytes` to avoid early
        // reallocations of the slot vector. We use a divisor of 16
        // (a typical short-name length) as a coarse heuristic; this is
        // a pure capacity hint and never observable.
        let slot_hint = (cfg.initial_bytes as usize).max(16) / 16;
        let inner = Inner {
            slots: Vec::with_capacity(slot_hint),
            cids: Vec::with_capacity(slot_hint),
            index: HashMap::new(),
        };
        Self {
            inner: RwLock::new(inner),
            hash_seed: cfg.hash_seed,
        }
    }

    /// Construct a pool with the standard host preset.
    #[must_use]
    pub fn standard() -> Self {
        Self::new(&PoolConfig::standard())
    }

    /// Current number of interned strings.
    ///
    /// # Errors
    ///
    /// Returns [`InternError::Poisoned`] if the underlying lock was
    /// poisoned by a panicking writer.
    pub fn len(&self) -> Result<usize, InternError> {
        let inner = self.inner.read().map_err(|_| InternError::Poisoned)?;
        Ok(inner.slots.len())
    }

    /// `true` when no strings have been interned.
    pub fn is_empty(&self) -> Result<bool, InternError> {
        Ok(self.len()? == 0)
    }

    /// Intern `s`. Returns the existing [`StrId`] if `s` was already
    /// in the pool, or a freshly issued one otherwise.
    ///
    /// # Errors
    ///
    /// - [`InternError::HashCollision`] when a different string already
    ///   maps to the same xxHash3 fingerprint (extraordinarily rare;
    ///   bytes are byte-confirmed against the existing slot first).
    /// - [`InternError::CapacityExceeded`] when the slot table would
    ///   overflow [`u32::MAX`].
    /// - [`InternError::Poisoned`] when the underlying lock was
    ///   poisoned by another thread's panic.
    pub fn intern(&self, s: &str) -> Result<StrId, InternError> {
        let bytes = s.as_bytes();
        let hash = fast64_seeded(bytes, self.hash_seed);

        // Fast path: read-only lookup.
        {
            let inner = self.inner.read().map_err(|_| InternError::Poisoned)?;
            if let Some(id) = lookup(&inner, hash, bytes)? {
                return Ok(id);
            }
        }

        // Slow path: write-lock + insert.
        let mut inner = self.inner.write().map_err(|_| InternError::Poisoned)?;
        // Re-check after acquiring the write lock — another thread may
        // have inserted the same string between read and write
        // critical sections.
        if let Some(id) = lookup(&inner, hash, bytes)? {
            return Ok(id);
        }

        // Insert. One heap allocation per interned string keeps the
        // payload at a stable address.
        let slot_idx_u = inner.slots.len();
        if slot_idx_u >= u32::MAX as usize {
            return Err(InternError::CapacityExceeded);
        }
        let boxed: Box<[u8]> = Box::from(bytes);
        inner.slots.push(boxed);
        inner.cids.push(None);
        #[allow(clippy::cast_possible_truncation)]
        let slot_idx = slot_idx_u as u32;
        inner.index.insert(hash, slot_idx);

        Lid::from_index(slot_idx_u).ok_or(InternError::CapacityExceeded)
    }

    /// Look up an interned string without inserting it.
    ///
    /// Returns `None` if `s` is not present or the lock is poisoned.
    #[must_use]
    pub fn get_id(&self, s: &str) -> Option<StrId> {
        let bytes = s.as_bytes();
        let hash = fast64_seeded(bytes, self.hash_seed);
        let inner = self.inner.read().ok()?;
        lookup(&inner, hash, bytes).ok().flatten()
    }

    /// Resolve `id` to its interned string slice, zero-copy.
    ///
    /// The returned slice borrows from the pool's per-slot heap
    /// allocation and is valid for the lifetime of `&self`.
    ///
    /// Returns `None` if `id` is out of range, the slot's bytes are
    /// not valid UTF-8 (which cannot happen for any handle issued by
    /// [`intern`](Self::intern)), or the lock is poisoned.
    #[must_use]
    pub fn get(&self, id: StrId) -> Option<&str> {
        let inner = self.inner.read().ok()?;
        let slot: &[u8] = inner.slots.get(id.index())?.as_ref();
        let s = core::str::from_utf8(slot).ok()?;

        // SAFETY: The byte slice we are extending was obtained from
        // `Box<[u8]>::as_ref` against a `Box` that lives inside
        // `Inner.slots`. Three invariants make this lifetime extension
        // sound:
        //
        //   1. **Box payload stability.** A `Box<[u8]>`'s heap
        //      allocation has a fixed address from the moment it is
        //      created until the box is dropped. The outer
        //      `Vec<Box<[u8]>>` may reallocate when growing, but
        //      reallocation moves only the *box headers* (data
        //      pointer + length pairs), never the payloads they own.
        //      `&[u8]` is `(data_ptr, len)` — both copied out of the
        //      header — so the slice remains valid even if the vector
        //      reallocates.
        //
        //   2. **No deletion.** Slots are append-only; we never call
        //      `Vec::remove`, `Vec::pop`, `Vec::truncate`, or
        //      reassign an existing slot. Therefore the box backing
        //      this slot is not dropped before the pool is dropped.
        //
        //   3. **Byte immutability.** Once interned, a slot's bytes
        //      are never mutated. No `&mut [u8]` is ever produced
        //      from `inner.slots[i]` after its initial push, so the
        //      shared `&[u8]` does not race with any writer.
        //
        // The pool itself is borrowed for `'_` (the elided lifetime
        // of `&self`), so the returned `&str` cannot outlive the
        // pool. The read guard is dropped at the end of the function;
        // that drop only releases the `RwLock`, not the boxed bytes.
        #[allow(unsafe_code)]
        let s_extended: &str = unsafe { core::mem::transmute::<&str, &str>(s) };
        Some(s_extended)
    }

    /// Resolve `id` to its [`StrCid`] (BLAKE3-128 content address).
    ///
    /// The first call for a given slot computes the hash and caches
    /// it; subsequent calls return the cached value.
    pub fn to_cid(&self, id: StrId) -> Option<StrCid> {
        let idx = id.index();
        // Fast path: read lock + check cache. Hash outside the lock
        // if we miss, to keep the critical section short.
        let bytes_owned;
        {
            let inner = self.inner.read().ok()?;
            let slot: &[u8] = inner.slots.get(idx)?.as_ref();
            if let Some(Some(cached)) = inner.cids.get(idx) {
                return Some(Cid::from_bytes(*cached));
            }
            bytes_owned = slot.to_vec();
        }
        let digest = content128(&bytes_owned);
        // Slow path: write lock to cache. If another thread cached in
        // the interim, its value is identical (BLAKE3 is deterministic).
        let mut inner_w = self.inner.write().ok()?;
        if let Some(slot_cache) = inner_w.cids.get_mut(idx) {
            *slot_cache = Some(digest);
        }
        Some(Cid::from_bytes(digest))
    }

    /// Inspect whether the [`StrCid`] for `id` has been materialised.
    /// Used by tests; not part of the public stability contract.
    #[doc(hidden)]
    #[must_use]
    pub fn is_cid_cached(&self, id: StrId) -> bool {
        if let Ok(inner) = self.inner.read() {
            matches!(inner.cids.get(id.index()), Some(Some(_)))
        } else {
            false
        }
    }
}

/// Lookup helper used by both the read-only fast path and the
/// re-check inside the write critical section.
fn lookup(inner: &Inner, hash: u64, bytes: &[u8]) -> Result<Option<StrId>, InternError> {
    let Some(&slot_idx) = inner.index.get(&hash) else {
        return Ok(None);
    };
    let Some(slot) = inner.slots.get(slot_idx as usize) else {
        return Ok(None);
    };
    let existing: &[u8] = slot.as_ref();
    if existing == bytes {
        let id = Lid::from_index(slot_idx as usize).ok_or(InternError::CapacityExceeded)?;
        Ok(Some(id))
    } else {
        let existing_id =
            Lid::from_index(slot_idx as usize).ok_or(InternError::CapacityExceeded)?;
        Err(InternError::HashCollision {
            existing: existing_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_returns_same_id_for_same_string() {
        let pool = StringPool::standard();
        let a = pool.intern("hello").unwrap();
        let b = pool.intern("hello").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn intern_returns_different_ids_for_different_strings() {
        let pool = StringPool::standard();
        let a = pool.intern("hello").unwrap();
        let b = pool.intern("world").unwrap();
        assert_ne!(a, b);
        assert_eq!(pool.get(a), Some("hello"));
        assert_eq!(pool.get(b), Some("world"));
    }

    #[test]
    fn get_returns_borrowed_str() {
        let pool = StringPool::standard();
        let id = pool.intern("borrowed").unwrap();
        let s: &str = pool.get(id).unwrap();
        assert_eq!(s, "borrowed");
    }

    #[test]
    fn get_id_finds_existing() {
        let pool = StringPool::standard();
        assert!(pool.get_id("unknown").is_none());
        let id = pool.intern("known").unwrap();
        assert_eq!(pool.get_id("known"), Some(id));
    }

    #[test]
    fn to_cid_is_lazy_then_cached() {
        let pool = StringPool::standard();
        let id = pool.intern("payload").unwrap();
        assert!(!pool.is_cid_cached(id));
        let cid = pool.to_cid(id).unwrap();
        assert!(pool.is_cid_cached(id));
        // Stable across calls.
        let cid2 = pool.to_cid(id).unwrap();
        assert_eq!(cid, cid2);
    }

    #[test]
    fn to_cid_matches_content128_directly() {
        let pool = StringPool::standard();
        let id = pool.intern("foobar").unwrap();
        let cid = pool.to_cid(id).unwrap();
        let direct = content128(b"foobar");
        assert_eq!(cid.as_bytes(), &direct);
    }

    #[test]
    fn cross_pool_to_cid_stability() {
        let p1 = StringPool::standard();
        let p2 = StringPool::standard();
        let i1 = p1.intern("addr").unwrap();
        let i2 = p2.intern("addr").unwrap();
        let c1 = p1.to_cid(i1).unwrap();
        let c2 = p2.to_cid(i2).unwrap();
        assert_eq!(c1, c2);
    }

    #[test]
    fn concurrent_intern_same_string_returns_same_id() {
        use std::sync::Arc;
        use std::thread;
        let pool = Arc::new(StringPool::standard());
        let mut handles = Vec::new();
        for _ in 0..16 {
            let p = pool.clone();
            handles.push(thread::spawn(move || p.intern("contended").unwrap()));
        }
        let ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let first = ids[0];
        for id in &ids[1..] {
            assert_eq!(*id, first);
        }
    }

    #[test]
    fn len_grows_monotonically_for_distinct_strings() {
        let pool = StringPool::standard();
        assert_eq!(pool.len().unwrap(), 0);
        pool.intern("a").unwrap();
        pool.intern("b").unwrap();
        pool.intern("a").unwrap(); // dup, no growth
        assert_eq!(pool.len().unwrap(), 2);
    }

    #[test]
    fn slot_addresses_are_stable_across_growth() {
        // Validate the property that closing the path bridge depends
        // on: bytes obtained from `get()` keep their address even
        // after subsequent interns force the slot vector to grow.
        let pool = StringPool::standard();
        let first_id = pool.intern("anchor").unwrap();
        let first_ptr = pool.get(first_id).unwrap().as_ptr();
        // Insert enough distinct strings to force at least one slot
        // vector reallocation past the initial capacity hint.
        for i in 0..256 {
            let s = alloc::format!("filler-{i}");
            pool.intern(&s).unwrap();
        }
        let again_ptr = pool.get(first_id).unwrap().as_ptr();
        assert_eq!(first_ptr, again_ptr);
        assert_eq!(pool.get(first_id), Some("anchor"));
    }
}

//! `StringPool` — content-addressed string interner.
//!
//! M2 ships a single-locked variant: one global `RwLock<Inner>` per
//! pool. The plan describes a sharded scheme, but the per-shard slot
//! tables would force shard-index bits into the [`StrId`] (which
//! breaks the simple `Lid::index() == slot_idx` contract). M3 will
//! revisit sharding either by encoding shard bits in the `Lid` upper
//! word or by sharding only the lookup index while keeping the slot
//! and byte vectors global. The user-visible API is identical.

extern crate alloc;

use alloc::string::String;
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
    /// Pool capacity exhausted: slot count would exceed [`u32::MAX`]
    /// or byte offset would exceed [`u32::MAX`].
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

/// One slot in the slot table: a `(offset, len)` pair into the pool's
/// byte buffer plus a lazy [`StrCid`] cache.
#[derive(Debug, Clone, Copy)]
struct Slot {
    offset: u32,
    len: u32,
}

#[derive(Debug, Default)]
struct Inner {
    bytes: Vec<u8>,
    slots: Vec<Slot>,
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
    /// Only `cfg.hash_seed` and `cfg.initial_bytes` are consumed in
    /// M2; `shard_count` is reserved for the sharded rewrite.
    #[must_use]
    pub fn new(cfg: &PoolConfig) -> Self {
        let inner = Inner {
            bytes: Vec::with_capacity(cfg.initial_bytes as usize),
            slots: Vec::new(),
            cids: Vec::new(),
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
    /// - [`InternError::CapacityExceeded`] when the slot table or byte
    ///   buffer would overflow [`u32::MAX`].
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

        // Insert.
        let offset = inner.bytes.len();
        let len = bytes.len();
        if offset > u32::MAX as usize || len > u32::MAX as usize {
            return Err(InternError::CapacityExceeded);
        }
        inner.bytes.extend_from_slice(bytes);

        let slot_idx_u = inner.slots.len();
        if slot_idx_u >= u32::MAX as usize {
            return Err(InternError::CapacityExceeded);
        }
        #[allow(clippy::cast_possible_truncation)]
        let offset_u32 = offset as u32;
        #[allow(clippy::cast_possible_truncation)]
        let len_u32 = len as u32;
        inner.slots.push(Slot {
            offset: offset_u32,
            len: len_u32,
        });
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

    /// Resolve `id` to its interned string.
    ///
    /// Returns a heap-allocated `String` because the underlying bytes
    /// live behind an `RwLock`; zero-copy resolution will land with
    /// the sharded rewrite when slot tables are visible through a
    /// stable guard.
    #[must_use]
    pub fn get(&self, id: StrId) -> Option<String> {
        let inner = self.inner.read().ok()?;
        let slot = inner.slots.get(id.index())?;
        let bytes = inner.bytes.get(slot.offset as usize..)?;
        let bytes = bytes.get(..slot.len as usize)?;
        let s = core::str::from_utf8(bytes).ok()?;
        Some(String::from(s))
    }

    /// Resolve `id` to its [`StrCid`] (BLAKE3-128 content address).
    ///
    /// The first call for a given slot computes the hash and caches
    /// it; subsequent calls return the cached value.
    pub fn to_cid(&self, id: StrId) -> Option<StrCid> {
        let idx = id.index();
        // Fast path: read lock + check cache.
        let bytes_owned;
        {
            let inner = self.inner.read().ok()?;
            let slot = inner.slots.get(idx)?;
            if let Some(Some(cached)) = inner.cids.get(idx) {
                return Some(Cid::from_bytes(*cached));
            }
            // Materialise the slot's bytes for hashing outside the
            // read lock — `content128` is cheap but we keep the
            // critical section minimal.
            let head = inner.bytes.get(slot.offset as usize..)?;
            let body = head.get(..slot.len as usize)?;
            bytes_owned = body.to_vec();
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
    let head = inner
        .bytes
        .get(slot.offset as usize..)
        .ok_or(InternError::CapacityExceeded)?;
    let existing = head
        .get(..slot.len as usize)
        .ok_or(InternError::CapacityExceeded)?;
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
        assert_eq!(pool.get(a).as_deref(), Some("hello"));
        assert_eq!(pool.get(b).as_deref(), Some("world"));
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
}

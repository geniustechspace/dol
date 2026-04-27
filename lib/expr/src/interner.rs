#[cfg(feature = "serde")]
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::{BuildHasher, Hasher};
use hashbrown::{DefaultHashBuilder, HashTable};

use crate::ids::StrId;

/// String interner backed by a single bump-allocated byte blob.
///
/// Each unique string is appended once to `bytes`; the public `StrId` is an
/// index into `slices`, which records `(offset, len)` pairs so that
/// `get(id)` performs a single bounds-checked byte slice lookup. The lookup
/// table stores only `StrId`s and recovers the string content from the
/// bytes blob on demand, so insertion needs no per-string heap allocation
/// beyond the bytes themselves.
///
/// This replaces an earlier `Vec<Arc<str>> + HashMap<Arc<str>, StrId>`
/// design which paid for an extra `Arc` allocation (and 16 B refcount
/// header) per unique string. For a typical query plan dominated by short
/// identifiers (column / table names) that overhead routinely exceeded the
/// payload itself.
///
/// The on-the-wire serde codec is unchanged: a flat `Vec<String>` whose
/// order matches `StrId` assignment.
#[derive(Debug, Clone, Default)]
pub struct Interner {
    /// Concatenated UTF-8 bytes for every interned string.
    bytes: Vec<u8>,
    /// `(offset, len)` into `bytes` for each `StrId`.
    slices: Vec<(u32, u32)>,
    /// Lookup table from interned string content to its `StrId`.
    ///
    /// Stores only the `StrId`; equality is checked against the bytes blob
    /// on demand so the table itself carries no string data.
    table: HashTable<StrId>,
    /// Owned `BuildHasher` so the *same* hash is produced for the same
    /// bytes throughout this interner's lifetime — `DefaultHashBuilder`
    /// (foldhash) seeds a new random state on every `default()` call, so a
    /// per-call builder would mean `intern("x")` and `try_get("x")` see
    /// different hashes and never collide.
    hasher: DefaultHashBuilder,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern `s`, returning a stable [`StrId`].
    ///
    /// Idempotent: repeated calls with equal `&str`s return the same id
    /// without appending to the bytes blob.
    pub fn intern(&mut self, s: &str) -> StrId {
        let hash = self.hash_bytes(s.as_bytes());

        if let Some(&existing) = self.table.find(hash, |&id| {
            let (off, len) = self.slices[id as usize];
            let stored = &self.bytes[off as usize..off as usize + len as usize];
            stored == s.as_bytes()
        }) {
            return existing;
        }

        let id = self.slices.len() as StrId;
        let off = self.bytes.len() as u32;
        let len = s.len() as u32;
        self.bytes.extend_from_slice(s.as_bytes());
        self.slices.push((off, len));

        let bytes = &self.bytes;
        let slices = &self.slices;
        let hasher = &self.hasher;
        self.table.insert_unique(hash, id, |&inserted| {
            let (off, len) = slices[inserted as usize];
            let stored = &bytes[off as usize..off as usize + len as usize];
            let mut h = hasher.build_hasher();
            h.write(stored);
            h.finish()
        });
        id
    }

    /// Retrieve a previously-interned string by its [`StrId`].
    ///
    /// # Panics
    ///
    /// Panics if `id` was not produced by this interner (out-of-range).
    pub fn get(&self, id: StrId) -> &str {
        let (off, len) = self.slices[id as usize];
        let raw = &self.bytes[off as usize..off as usize + len as usize];
        // Every byte slice in `bytes` was appended from a `&str` in
        // `intern`, which guarantees valid UTF-8 at the slice boundaries.
        // We still go through `from_utf8` so the crate stays
        // `#![deny(unsafe_code)]`-clean; the cost is a single
        // bounds-checked validation against trusted input.
        core::str::from_utf8(raw).expect("interner stores only valid UTF-8")
    }

    /// Return the [`StrId`] for `s` if it has already been interned.
    pub fn try_get(&self, s: &str) -> Option<StrId> {
        let hash = self.hash_bytes(s.as_bytes());
        self.table
            .find(hash, |&id| {
                let (off, len) = self.slices[id as usize];
                let stored = &self.bytes[off as usize..off as usize + len as usize];
                stored == s.as_bytes()
            })
            .copied()
    }

    pub fn len(&self) -> usize {
        self.slices.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }
    pub fn reset(&mut self) {
        self.bytes.clear();
        self.slices.clear();
        self.table.clear();
        // The hasher's seed stays — clearing it would re-randomise and
        // invalidate any `StrId` callers cached across `reset()`.
    }

    /// Return the total bytes resident in this interner — the bytes blob
    /// plus the slice index plus an estimate of the table footprint.
    ///
    /// Used by `crate::stats` to track memory wins between revisions.
    pub fn heap_bytes(&self) -> usize {
        let bytes_cap = self.bytes.capacity();
        let slices_cap = self.slices.capacity() * core::mem::size_of::<(u32, u32)>();
        // Conservative table footprint: one slot per allocated capacity
        // entry, sized to `StrId`. hashbrown adds metadata (~1 byte / entry)
        // and load-factor headroom but doesn't expose either; for
        // regression-tracking purposes the dominant term is the bytes blob.
        let table_cap = self.table.capacity() * (core::mem::size_of::<StrId>() + 1);
        bytes_cap + slices_cap + table_cap
    }

    fn hash_bytes(&self, bytes: &[u8]) -> u64 {
        let mut h = self.hasher.build_hasher();
        h.write(bytes);
        h.finish()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Interner {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        // The `map` field is a derived index over the bytes blob — serialise
        // the canonical sequence of strings so the wire form stays minimal
        // and independent of HashMap iteration order.
        use serde::ser::SerializeSeq;
        let mut seq = ser.serialize_seq(Some(self.slices.len()))?;
        for i in 0..self.slices.len() {
            seq.serialize_element(self.get(i as StrId))?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Interner {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let strings: Vec<String> = serde::Deserialize::deserialize(de)?;
        let mut interner = Interner::default();
        interner.slices.reserve(strings.len());
        // Pre-size the table so the rehash callback below isn't called
        // during normal insertion.
        let bytes = &interner.bytes;
        let slices = &interner.slices;
        let hasher = &interner.hasher;
        interner.table.reserve(strings.len(), |&id| {
            let (off, len) = slices[id as usize];
            let mut h = hasher.build_hasher();
            h.write(&bytes[off as usize..off as usize + len as usize]);
            h.finish()
        });
        for s in &strings {
            // Re-intern through the canonical path so the lookup map and
            // bytes blob stay consistent with one another regardless of
            // duplicates encountered in malformed input.
            interner.intern(s);
        }
        Ok(interner)
    }
}

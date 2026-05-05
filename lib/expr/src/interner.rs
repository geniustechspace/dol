#[cfg(feature = "serde")]
use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashMap;

use crate::ids::StrId;

/// String interner with **content-addressed** [`StrId`]s.
///
/// Each [`StrId`] is the FNV-1a 32-bit hash of the string's bytes —
/// the same hash that backs the workspace's extension `Symbol` ids
/// (`lib/pipeline/src/extension.rs`, `lib/stream/src/extension.rs`).
/// Two interner instances therefore produce **identical** ids for the
/// same string, even across processes / machines, which lets ids be
/// reused as plan-cache keys, on-disk indices, and IoT idempotency
/// tokens without coordinating an in-memory registry.
///
/// Strings themselves still live in a single bump-allocated byte blob
/// (`bytes`), with a `(offset, len)` slot per id stored in `slots`.
/// Collisions in the 32-bit hash space are detected on insertion and
/// surfaced as [`InternError::Collision`]; for typical query workloads
/// (a few thousand identifiers) the birthday probability is negligible
/// (~2⁻²⁰ at 4 K strings), but we never silently fold two distinct
/// strings together — that would invalidate every property the rest of
/// the IR relies on.
///
/// The on-the-wire serde codec is a flat `Vec<String>` in
/// **canonical (sorted-by-id) order**, so two interners populated with
/// the same set of strings (in any order) serialise byte-for-byte
/// identically. Decoding re-interns each entry, so wire bytes that
/// happen to encode different strings under the same id will surface
/// the collision as a deserialisation error.
#[derive(Debug, Clone, Default)]
pub struct Interner {
    /// Concatenated UTF-8 bytes for every interned string.
    bytes: Vec<u8>,
    /// `id -> (offset, len)` into `bytes`. The id is the FNV-1a 32-bit
    /// hash of the slice; we store the slot directly so `get(id)` is a
    /// single hash-map lookup.
    slots: HashMap<StrId, (u32, u32)>,
}

/// Errors produced by [`Interner::try_intern`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InternError {
    /// Two distinct strings hashed to the same 32-bit id. Surface this
    /// to the caller rather than silently aliasing the strings.
    Collision {
        /// The colliding id.
        id: StrId,
    },
}

impl core::fmt::Display for InternError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InternError::Collision { id } => write!(f, "interner: hash collision on id {id:#010x}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InternError {}

impl Interner {
    /// Build an empty interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern `s`, returning a stable [`StrId`].
    ///
    /// # Panics
    ///
    /// Panics on a 32-bit FNV-1a collision (two distinct strings that
    /// share the same id). Use [`try_intern`](Self::try_intern) on any
    /// path that handles untrusted input.
    pub fn intern(&mut self, s: &str) -> StrId {
        match self.try_intern(s) {
            Ok(id) => id,
            Err(InternError::Collision { id }) => {
                panic!(
                    "dol-expr::Interner: 32-bit FNV-1a collision on id {id:#010x}; \
                     use try_intern on adversarial input"
                )
            }
        }
    }

    /// Intern `s`, returning a stable [`StrId`] — or [`InternError::Collision`]
    /// when a different string already occupies the same id.
    pub fn try_intern(&mut self, s: &str) -> Result<StrId, InternError> {
        let id = fnv1a_32(s.as_bytes());
        if let Some(&(off, len)) = self.slots.get(&id) {
            let stored = &self.bytes[off as usize..off as usize + len as usize];
            if stored == s.as_bytes() {
                return Ok(id);
            }
            return Err(InternError::Collision { id });
        }

        let off = self.bytes.len() as u32;
        let len = s.len() as u32;
        self.bytes.extend_from_slice(s.as_bytes());
        self.slots.insert(id, (off, len));
        Ok(id)
    }

    /// Retrieve a previously-interned string by its [`StrId`].
    ///
    /// # Panics
    ///
    /// Panics if `id` was not produced by this interner. Callers
    /// processing untrusted data should use [`get_opt`](Self::get_opt).
    pub fn get(&self, id: StrId) -> &str {
        self.get_opt(id)
            .expect("dol-expr::Interner::get: unknown StrId")
    }

    /// Retrieve a previously-interned string by its [`StrId`], returning
    /// `None` when the id was not produced by this interner.
    pub fn get_opt(&self, id: StrId) -> Option<&str> {
        let &(off, len) = self.slots.get(&id)?;
        let raw = &self.bytes[off as usize..off as usize + len as usize];
        // Every byte slice in `bytes` was appended from a `&str` in
        // `intern`, which guarantees valid UTF-8 at the slice boundaries.
        // We still go through `from_utf8` so the crate stays
        // `#![forbid(unsafe_code)]`-clean; the cost is a single
        // bounds-checked validation against trusted input.
        Some(core::str::from_utf8(raw).expect("interner stores only valid UTF-8"))
    }

    /// Return the [`StrId`] for `s` if it has already been interned.
    pub fn try_get(&self, s: &str) -> Option<StrId> {
        let id = fnv1a_32(s.as_bytes());
        let &(off, len) = self.slots.get(&id)?;
        let stored = &self.bytes[off as usize..off as usize + len as usize];
        (stored == s.as_bytes()).then_some(id)
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
    pub fn reset(&mut self) {
        self.bytes.clear();
        self.slots.clear();
    }

    /// Return the total bytes resident in this interner — the bytes blob
    /// plus an estimate of the slot table footprint.
    ///
    /// Used by `crate::stats` to track memory wins between revisions.
    pub fn heap_bytes(&self) -> usize {
        let bytes_cap = self.bytes.capacity();
        // hashbrown doesn't expose the underlying allocation size, so
        // approximate as one slot's worth per allocated capacity entry.
        let slots_cap = self.slots.capacity()
            * (core::mem::size_of::<StrId>() + core::mem::size_of::<(u32, u32)>());
        bytes_cap + slots_cap
    }
}

/// `const`-eval FNV-1a 32-bit hash. Stable; matches the spec basis/prime
/// and the [`Symbol`] convention used by extensions in
/// `lib/pipeline/src/extension.rs` and `lib/stream/src/extension.rs`.
///
/// [`Symbol`]: dol_core::ext::Symbol
const fn fnv1a_32(bytes: &[u8]) -> u32 {
    // FNV-1a 32-bit constants per the reference spec.
    let mut hash: u32 = 0x811c_9dc5;
    let prime: u32 = 0x0100_0193;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }
    hash
}

#[cfg(feature = "serde")]
impl serde::Serialize for Interner {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        // Serialise as a flat `Vec<String>` in canonical (sorted-by-id)
        // order so two interners with the same string set serialise
        // byte-for-byte identically regardless of insertion order.
        use serde::ser::SerializeSeq;
        let mut entries: Vec<(StrId, &str)> = self
            .slots
            .iter()
            .map(|(&id, &(off, len))| {
                let raw = &self.bytes[off as usize..off as usize + len as usize];
                let s = core::str::from_utf8(raw).expect("interner stores only valid UTF-8");
                (id, s)
            })
            .collect();
        entries.sort_unstable_by_key(|&(id, _)| id);
        let mut seq = ser.serialize_seq(Some(entries.len()))?;
        for (_, s) in &entries {
            seq.serialize_element(s)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Interner {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let strings: Vec<String> = serde::Deserialize::deserialize(de)?;
        let mut interner = Interner::default();
        for s in &strings {
            // Re-intern through the canonical path so the slot map and
            // bytes blob stay consistent with one another. A collision
            // in the wire payload (two distinct strings that hash to the
            // same id) is surfaced as a serde error rather than a panic.
            interner
                .try_intern(s)
                .map_err(<D::Error as serde::de::Error>::custom)?;
        }
        Ok(interner)
    }
}

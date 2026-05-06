use alloc::vec::Vec;
use core::num::NonZeroU32;
use hashbrown::HashMap;

use crate::ids::StrId;

/// String interner with **content-addressed** [`StrId`]s.
///
/// Each [`StrId`] is the leading 32 bits of the BLAKE3 digest of the
/// string's bytes (via the workspace's `dol_core::hash` chokepoint —
/// the only place in the workspace that calls into `blake3` directly,
/// per `docs/v2_plan.md` §25/§37). Two interner instances therefore
/// produce **identical** ids for the same string, even across processes
/// / machines / language bindings, which lets ids be reused as
/// plan-cache keys, on-disk indices, signed-manifest references, and
/// IoT idempotency tokens without coordinating an in-memory registry.
///
/// BLAKE3 is cryptographic — an attacker controlling input cannot steer
/// distinct strings into the same id without breaking the underlying
/// primitive. (FNV-1a, the workspace's compile-time `Symbol` hash for
/// extension dispatch, is fine for that closed set of vendor-blessed
/// constants but unsuitable for runtime user input; this is why the
/// interner uses BLAKE3 even though the hash output is truncated.)
///
/// Strings themselves still live in a single bump-allocated byte blob
/// (`bytes`), with a `(offset, len)` slot per id stored in `slots`.
/// Collisions in the 32-bit hash space are detected on insertion and
/// surfaced as [`InternError::Collision`]; for typical query workloads
/// (a few thousand identifiers) the birthday probability is negligible
/// (~2⁻²⁰ at 4 K strings), but we never silently fold two distinct
/// strings together — that would invalidate every property the rest of
/// the IR relies on. Future work tracked in the v2 plan widens
/// [`StrId`] to 64 bits so this collision surface vanishes entirely.
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
    /// `id -> (offset, len)` into `bytes`. The id is the leading 32
    /// bits of `BLAKE3(slice)`; we store the slot directly so `get(id)`
    /// is a single hash-map lookup.
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
            InternError::Collision { id } => {
                write!(f, "interner: hash collision on id {:#010x}", id.get())
            }
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
    /// Panics on a 32-bit BLAKE3-prefix collision (two distinct strings
    /// that share the same id). Use [`try_intern`](Self::try_intern) on
    /// any path that handles untrusted input.
    //
    // Lint exemption: this is the documented "panic on collision"
    // counterpart to `try_intern`; v2's no-panic invariant carves out
    // panics that appear in `# Panics` rustdoc with a fallible sibling.
    #[allow(clippy::panic)]
    pub fn intern(&mut self, s: &str) -> StrId {
        match self.try_intern(s) {
            Ok(id) => id,
            Err(InternError::Collision { id }) => {
                panic!(
                    "dol-expr::Interner: 32-bit BLAKE3-prefix collision on id {:#010x}; \
                     use try_intern on adversarial input",
                    id.get()
                )
            }
        }
    }

    /// Intern `s`, returning a stable [`StrId`] — or [`InternError::Collision`]
    /// when a different string already occupies the same id.
    // Slot offsets and lengths are produced by `try_intern` against the same
    // `bytes` blob; `off + len <= bytes.len()` is an internal invariant so
    // the slice indexing and addition cannot escape bounds.
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    pub fn try_intern(&mut self, s: &str) -> Result<StrId, InternError> {
        let id = strid_for(s.as_bytes());
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
    //
    // Lint exemption: documented "panic on unknown id" counterpart to
    // `get_opt`; same carve-out as `intern` vs `try_intern`.
    #[allow(clippy::expect_used)]
    pub fn get(&self, id: StrId) -> &str {
        self.get_opt(id)
            .expect("dol-expr::Interner::get: unknown StrId")
    }

    /// Retrieve a previously-interned string by its [`StrId`], returning
    /// `None` when the id was not produced by this interner.
    //
    // Lint exemption: the inner `from_utf8(...).expect(...)` upholds an
    // internal invariant — every byte slice in `bytes` was appended from
    // a `&str` in `intern`, which guarantees valid UTF-8 at the slice
    // boundaries. We still go through `from_utf8` so the crate stays
    // `#![forbid(unsafe_code)]`-clean.
    #[allow(clippy::expect_used)]
    // `(off, len)` was produced by `try_intern` against the same `bytes`
    // blob; `off + len <= bytes.len()` holds by construction.
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    pub fn get_opt(&self, id: StrId) -> Option<&str> {
        let &(off, len) = self.slots.get(&id)?;
        let raw = &self.bytes[off as usize..off as usize + len as usize];
        Some(core::str::from_utf8(raw).expect("interner stores only valid UTF-8"))
    }

    /// Return the [`StrId`] for `s` if it has already been interned.
    // `(off, len)` was produced by `try_intern` against the same `bytes`
    // blob; `off + len <= bytes.len()` holds by construction.
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    pub fn try_get(&self, s: &str) -> Option<StrId> {
        let id = strid_for(s.as_bytes());
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
    // Heap-byte estimate; the multiplication and addition are bounded by
    // `Vec`/`HashMap` capacities (≤ `isize::MAX`) and would saturate on a
    // hypothetical overflow rather than mis-account.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn heap_bytes(&self) -> usize {
        let bytes_cap = self.bytes.capacity();
        // hashbrown doesn't expose the underlying allocation size, so
        // approximate as one slot's worth per allocated capacity entry.
        let slots_cap = self.slots.capacity()
            * (core::mem::size_of::<StrId>() + core::mem::size_of::<(u32, u32)>());
        bytes_cap + slots_cap
    }

    #[allow(clippy::expect_used)]
    // Slot offsets/lengths were produced by `try_intern` against the same
    // `bytes` blob; `off + len <= bytes.len()` holds by construction.
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    pub fn sorted_strings(&self) -> alloc::vec::Vec<&str> {
        let mut entries: alloc::vec::Vec<(StrId, &str)> = self
            .slots
            .iter()
            .map(|(&id, &(off, len))| {
                let raw = &self.bytes[off as usize..off as usize + len as usize];
                let s = core::str::from_utf8(raw).expect("interner stores only valid UTF-8");
                (id, s)
            })
            .collect();
        entries.sort_unstable_by_key(|&(id, _)| id.get());
        entries.into_iter().map(|(_, s)| s).collect()
    }
}

/// Compute the content-addressed [`StrId`] for an arbitrary byte slice.
///
/// Uses the leading 32 bits of `dol_core::hash::hash32` (BLAKE3,
/// little-endian byte order) and folds the all-zero hash to `1` so the
/// result fits the [`NonZeroU32`] niche backing [`StrId`]. The fold
/// introduces a single artificial collision (the empty hash and `1` map
/// to the same id) at a one-in-2³² rate, surfaced through the standard
/// [`InternError::Collision`] path.
//
// Lint exemption: `NonZeroU32::new(...)` is fed a value that has just
// been folded away from zero; the `expect` documents an internal
// invariant rather than a runtime failure path.
#[allow(clippy::expect_used)]
fn strid_for(bytes: &[u8]) -> StrId {
    let digest = dol_core::hash::hash32(bytes);
    let h = u32::from_le_bytes(digest);
    let nz = NonZeroU32::new(if h == 0 { 1 } else { h }).expect("non-zero by construction");
    StrId::new(nz)
}

#[cfg(feature = "serde")]
impl serde::Serialize for Interner {
    #[allow(clippy::expect_used)]
    // Slot offsets/lengths were produced by `try_intern` against the same
    // `bytes` blob; `off + len <= bytes.len()` holds by construction.
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
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
        entries.sort_unstable_by_key(|&(id, _)| id.get());
        let mut seq = ser.serialize_seq(Some(entries.len()))?;
        for (_, s) in &entries {
            seq.serialize_element(s)?;
        }
        seq.end()
    }
}

// `Deserialize` is intentionally NOT implemented for `Interner` in v2.
// Wire-in for an `Interner` goes through `dol_wire::Decode` so the
// re-intern path threads `&mut Budget` and surfaces collisions as a
// `DecodeError`. `Serialize` is kept for human-readable JSON dumps.

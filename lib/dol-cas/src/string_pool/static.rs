//! `StaticStringPool` — fixed-size string interner for bare-metal targets.
//!
//! Stores up to `SLOTS` strings totalling at most `BYTES` bytes in
//! `static`/`const` storage. No allocator. `!Send + !Sync` — the
//! intended use is single-threaded `iot-min` deployments.
//!
//! Per `dol-rewrite-plan-v2.md` §7.4.

use core::fmt;

use dol_core::hash::fast64_seeded;

use crate::handle::{Lid, StrId};

/// Reason a string failed to intern into a [`StaticStringPool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StaticInternError {
    /// Slot table full (`SLOTS` exhausted).
    SlotsExhausted,
    /// Byte buffer full (the next string would overflow `BYTES`).
    BytesExhausted,
    /// xxHash3 fingerprint collision with a distinct existing string.
    /// The pre-existing handle is reported so callers can choose to
    /// re-key or refuse.
    HashCollision {
        /// Pre-existing handle.
        existing: StrId,
    },
}

impl fmt::Display for StaticInternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SlotsExhausted => f.write_str("StaticStringPool: slot table exhausted"),
            Self::BytesExhausted => f.write_str("StaticStringPool: byte buffer exhausted"),
            Self::HashCollision { .. } => f.write_str("StaticStringPool: hash collision"),
        }
    }
}

/// Slot record: `(offset, len, hash)` for one interned string.
///
/// Using `Option<...>` would force the whole entry into a niche-free
/// 16-byte layout; instead we use a separate `used` bit-vector
/// implicitly via `len == 0 && offset == 0` for unoccupied slots (no
/// pool ever issues a zero-length empty-offset entry because the
/// empty string is special-cased).
#[derive(Debug, Clone, Copy, Default)]
struct StaticSlot {
    offset: u32,
    len: u32,
    hash: u64,
}

/// Fixed-capacity, single-threaded string interner.
///
/// - `BYTES` is the total byte buffer size.
/// - `SLOTS` is the maximum number of interned strings.
///
/// Construction is `const`, so the pool can live in `static` storage.
/// `!Send + !Sync` is enforced by holding a `PhantomData<*const ()>`
/// in the struct.
#[derive(Debug)]
pub struct StaticStringPool<const BYTES: usize, const SLOTS: usize> {
    bytes: [u8; BYTES],
    bytes_used: usize,
    slots: [StaticSlot; SLOTS],
    slots_used: usize,
    hash_seed: u64,
    _not_send_sync: core::marker::PhantomData<*const ()>,
}

impl<const BYTES: usize, const SLOTS: usize> StaticStringPool<BYTES, SLOTS> {
    /// Construct an empty pool with the given xxHash3 seed.
    #[must_use]
    pub const fn new(hash_seed: u64) -> Self {
        Self {
            bytes: [0; BYTES],
            bytes_used: 0,
            slots: [StaticSlot {
                offset: 0,
                len: 0,
                hash: 0,
            }; SLOTS],
            slots_used: 0,
            hash_seed,
            _not_send_sync: core::marker::PhantomData,
        }
    }

    /// Intern `s`. Returns the existing handle if `s` is already
    /// present, or a fresh one.
    ///
    /// # Errors
    ///
    /// - [`StaticInternError::SlotsExhausted`] when the slot table
    ///   would overflow `SLOTS`.
    /// - [`StaticInternError::BytesExhausted`] when the byte buffer
    ///   would overflow `BYTES`.
    /// - [`StaticInternError::HashCollision`] on xxHash3 fingerprint
    ///   collision with a distinct existing string.
    pub fn intern(&mut self, s: &str) -> Result<StrId, StaticInternError> {
        let bytes = s.as_bytes();
        let hash = fast64_seeded(bytes, self.hash_seed);

        // Linear probe of the used slots. `SLOTS` is small by design
        // (a few dozen at most for `iot-min`); O(SLOTS) is fine.
        for i in 0..self.slots_used {
            let slot = self.slots.get(i).copied().unwrap_or_default();
            if slot.hash != hash {
                continue;
            }
            let head = match self.bytes.get(slot.offset as usize..) {
                Some(h) => h,
                None => return Err(StaticInternError::BytesExhausted),
            };
            let existing = match head.get(..slot.len as usize) {
                Some(e) => e,
                None => return Err(StaticInternError::BytesExhausted),
            };
            if existing == bytes {
                return Lid::from_index(i).ok_or(StaticInternError::SlotsExhausted);
            }
            let existing_id = Lid::from_index(i).ok_or(StaticInternError::SlotsExhausted)?;
            return Err(StaticInternError::HashCollision {
                existing: existing_id,
            });
        }

        // Insert.
        if self.slots_used >= SLOTS {
            return Err(StaticInternError::SlotsExhausted);
        }
        // `bytes_used + bytes.len()` must fit in BYTES and in u32.
        let new_used = match self.bytes_used.checked_add(bytes.len()) {
            Some(v) if v <= BYTES => v,
            _ => return Err(StaticInternError::BytesExhausted),
        };
        if self.bytes_used > u32::MAX as usize || bytes.len() > u32::MAX as usize {
            return Err(StaticInternError::BytesExhausted);
        }
        let dest = self
            .bytes
            .get_mut(self.bytes_used..new_used)
            .ok_or(StaticInternError::BytesExhausted)?;
        dest.copy_from_slice(bytes);

        let slot_idx = self.slots_used;
        let slot_ref = self
            .slots
            .get_mut(slot_idx)
            .ok_or(StaticInternError::SlotsExhausted)?;
        #[allow(clippy::cast_possible_truncation)]
        let offset_u32 = self.bytes_used as u32;
        #[allow(clippy::cast_possible_truncation)]
        let len_u32 = bytes.len() as u32;
        *slot_ref = StaticSlot {
            offset: offset_u32,
            len: len_u32,
            hash,
        };
        self.bytes_used = new_used;
        // `slot_idx < SLOTS` per the check above; increment cannot
        // overflow because SLOTS is a `usize`.
        #[allow(clippy::arithmetic_side_effects)]
        {
            self.slots_used += 1;
        }

        Lid::from_index(slot_idx).ok_or(StaticInternError::SlotsExhausted)
    }

    /// Resolve `id` to its interned string slice.
    #[must_use]
    pub fn get(&self, id: StrId) -> Option<&str> {
        let idx = id.index();
        if idx >= self.slots_used {
            return None;
        }
        let slot = self.slots.get(idx)?;
        let head = self.bytes.get(slot.offset as usize..)?;
        let body = head.get(..slot.len as usize)?;
        core::str::from_utf8(body).ok()
    }

    /// Number of strings currently interned.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.slots_used
    }

    /// `true` when nothing has been interned.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.slots_used == 0
    }

    /// Slot capacity (the `SLOTS` const parameter).
    #[must_use]
    pub const fn slot_capacity(&self) -> usize {
        SLOTS
    }

    /// Byte buffer capacity (the `BYTES` const parameter).
    #[must_use]
    pub const fn byte_capacity(&self) -> usize {
        BYTES
    }

    /// Bytes currently used.
    #[must_use]
    pub const fn bytes_used(&self) -> usize {
        self.bytes_used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_and_get_round_trip() {
        let mut pool: StaticStringPool<128, 8> = StaticStringPool::new(0);
        let a = pool.intern("hello").unwrap();
        let b = pool.intern("world").unwrap();
        assert_ne!(a, b);
        assert_eq!(pool.get(a), Some("hello"));
        assert_eq!(pool.get(b), Some("world"));
    }

    #[test]
    fn intern_dedupes_same_string() {
        let mut pool: StaticStringPool<64, 4> = StaticStringPool::new(0);
        let a = pool.intern("dup").unwrap();
        let b = pool.intern("dup").unwrap();
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn slots_exhausted() {
        let mut pool: StaticStringPool<128, 2> = StaticStringPool::new(0);
        pool.intern("a").unwrap();
        pool.intern("b").unwrap();
        assert_eq!(pool.intern("c"), Err(StaticInternError::SlotsExhausted));
    }

    #[test]
    fn bytes_exhausted() {
        let mut pool: StaticStringPool<3, 8> = StaticStringPool::new(0);
        pool.intern("ab").unwrap(); // 2 bytes
        // "cd" would require 4 bytes total; only 1 byte free.
        assert_eq!(pool.intern("cd"), Err(StaticInternError::BytesExhausted));
        // single-byte "c" fits exactly.
        pool.intern("c").unwrap();
        assert_eq!(pool.bytes_used(), 3);
    }

    #[test]
    fn capacities_match_const_params() {
        let pool: StaticStringPool<64, 4> = StaticStringPool::new(0);
        assert_eq!(pool.byte_capacity(), 64);
        assert_eq!(pool.slot_capacity(), 4);
        assert!(pool.is_empty());
    }
}

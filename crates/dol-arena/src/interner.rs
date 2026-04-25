use alloc::sync::Arc;
use alloc::vec::Vec;
use hashbrown::HashMap;

use crate::id::StrId;

/// String interner backed by `Arc<str>` so each unique string is stored in a
/// single shared allocation referenced from both the lookup map and the
/// id-lookup table.
///
/// Strings are categorised by an 8-bit `kind` byte that is packed into the
/// returned [`StrId`]. Lookups are partitioned by `kind` so two strings with
/// the same content but different kinds receive different ids.
#[derive(Debug, Clone, Default)]
pub struct Interner {
    strings: Vec<Arc<str>>,
    map: HashMap<(u8, Arc<str>), StrId>,
}

impl Interner {
    /// Create an empty interner.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-allocate space for `cap` unique strings.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            strings: Vec::with_capacity(cap),
            map: HashMap::with_capacity(cap),
        }
    }

    /// Intern `s` with the given `kind` byte. Returns the existing id if
    /// already interned, otherwise allocates a new one.
    pub fn intern(&mut self, s: &str, kind: u8) -> StrId {
        // Avoid an `Arc` allocation on the hot path when the string is
        // already interned. `HashMap::raw_entry_mut` would be cleaner but
        // requires a nightly feature; matching on contains_key is fine here
        // because the second lookup is amortised.
        if let Some(&id) = self.map.get(&(kind, Arc::<str>::from(s))) {
            return id;
        }
        let raw_index = self.strings.len() as u32;
        debug_assert!(raw_index <= StrId::MAX_INDEX, "interner overflow");
        let shared: Arc<str> = Arc::from(s);
        self.strings.push(Arc::clone(&shared));
        let id = StrId::new(raw_index, kind);
        self.map.insert((kind, shared), id);
        id
    }

    /// Resolve an interned id back to the original string. Panics if the id
    /// was not produced by this interner.
    #[inline]
    pub fn get(&self, id: StrId) -> &str {
        &self.strings[id.index() as usize]
    }

    /// Try to resolve `id` to its string, returning `None` if out of bounds.
    #[inline]
    pub fn try_get(&self, id: StrId) -> Option<&str> {
        self.strings.get(id.index() as usize).map(|s| s.as_ref())
    }

    /// Look up `s` (with `kind`) without inserting it.
    pub fn lookup(&self, s: &str, kind: u8) -> Option<StrId> {
        self.map.get(&(kind, Arc::<str>::from(s))).copied()
    }

    /// Number of unique strings interned.
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// `true` if nothing has been interned.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Drop every interned string. Capacity is retained.
    pub fn reset(&mut self) {
        self.strings.clear();
        self.map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_dedupes() {
        let mut i = Interner::new();
        let a = i.intern("hello", 0);
        let b = i.intern("hello", 0);
        let c = i.intern("hello", 1);
        assert_eq!(a, b);
        assert_ne!(a, c, "different kinds yield different ids");
        assert_eq!(i.len(), 2);
        assert_eq!(i.get(a), "hello");
        assert_eq!(i.get(c), "hello");
        assert_eq!(a.kind(), 0);
        assert_eq!(c.kind(), 1);
    }

    #[test]
    fn lookup_no_insert() {
        let mut i = Interner::new();
        assert!(i.lookup("x", 0).is_none());
        let id = i.intern("x", 0);
        assert_eq!(i.lookup("x", 0), Some(id));
        assert_eq!(i.len(), 1);
    }
}

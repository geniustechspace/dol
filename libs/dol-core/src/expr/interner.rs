//! String interner for the expression arena.
//!
//! [`Interner`] maps unique strings to compact [`StrId`] indices (u32),
//! eliminating duplicate heap allocations for repeatedly-used identifier
//! names (column names, table aliases, etc.).

use std::collections::HashMap;

/// A compact index into an [`Interner`]'s string table.
///
/// Two `StrId`s from the same interner compare equal if and only if the
/// underlying strings are equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StrId(pub(crate) u32);

/// A string interner: maps `&str` → [`StrId`], deduplicating on insertion.
///
/// Strings are stored as `Box<str>` in stable order; the index is their
/// position in the insertion-ordered `strings` vec.
pub struct Interner {
    strings: Vec<Box<str>>,
    map:     HashMap<Box<str>, StrId>,
}

impl Interner {
    /// Create an empty interner.
    pub fn new() -> Self {
        Self { strings: Vec::new(), map: HashMap::new() }
    }

    /// Intern a string, returning its [`StrId`].
    ///
    /// If the string was already interned the existing id is returned without
    /// any new allocation.
    pub fn intern(&mut self, s: &str) -> StrId {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = StrId(self.strings.len() as u32);
        let owned: Box<str> = s.into();
        self.strings.push(owned.clone());
        self.map.insert(owned, id);
        id
    }

    /// Resolve a [`StrId`] to its original string.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not produced by this interner.
    pub fn get(&self, id: StrId) -> &str {
        &self.strings[id.0 as usize]
    }

    /// Number of distinct strings interned.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns `true` if no strings have been interned yet.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

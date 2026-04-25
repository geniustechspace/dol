use std::collections::HashMap;
use std::sync::Arc;

use crate::ids::StrId;

/// String interner backed by `Arc<str>` so each unique string is stored in a
/// single shared heap allocation referenced from both the lookup map and the
/// id-lookup table.
#[derive(Debug, Clone, Default)]
pub struct Interner {
    strings: Vec<Arc<str>>,
    map: HashMap<Arc<str>, StrId>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> StrId {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = self.strings.len() as StrId;
        let shared: Arc<str> = Arc::from(s);
        self.strings.push(Arc::clone(&shared));
        self.map.insert(shared, id);
        id
    }

    pub fn get(&self, id: StrId) -> &str {
        &self.strings[id as usize]
    }

    pub fn try_get(&self, s: &str) -> Option<StrId> {
        self.map.get(s).copied()
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
    pub fn reset(&mut self) {
        self.strings.clear();
        self.map.clear();
    }
}

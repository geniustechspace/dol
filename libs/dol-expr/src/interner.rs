use std::collections::HashMap;

use crate::ids::StrId;

#[derive(Debug, Clone, Default)]
pub struct Interner {
    strings: Vec<Box<str>>,
    map:     HashMap<Box<str>, StrId>,
}

impl Interner {
    pub fn new() -> Self { Self::default() }

    pub fn intern(&mut self, s: &str) -> StrId {
        if let Some(&id) = self.map.get(s) { return id; }
        let id = self.strings.len() as StrId;
        let owned: Box<str> = s.into();
        self.strings.push(owned.clone());
        self.map.insert(owned, id);
        id
    }

    pub fn get(&self, id: StrId) -> &str { &self.strings[id as usize] }

    pub fn try_get(&self, s: &str) -> Option<StrId> {
        self.map.get(s).copied()
    }

    pub fn len(&self) -> usize { self.strings.len() }
    pub fn is_empty(&self) -> bool { self.strings.is_empty() }
    pub fn reset(&mut self) { self.strings.clear(); self.map.clear(); }
}

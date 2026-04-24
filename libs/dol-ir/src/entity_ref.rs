//! [`EntityRef`] — a reference to a model (table/collection/bucket).

/// A reference to a model (table/collection/bucket), with optional alias.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityRef {
    pub name:      String,
    pub namespace: Option<String>,
    pub alias:     Option<String>,
}

impl EntityRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), namespace: None, alias: None }
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

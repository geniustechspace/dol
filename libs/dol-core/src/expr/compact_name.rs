//! Shared compact name storage for well-known and custom function/operator names.

use core::fmt;

/// A name that's either a static string (well-known, zero-alloc) or an owned
/// heap string (custom/runtime).
///
/// Comparison and hashing operate on the string content regardless of variant.
#[derive(Debug, Clone)]
pub enum CompactName {
    /// A well-known name, stored without heap allocation.
    Static(&'static str),
    /// A custom/runtime name stored on the heap.
    Owned(Box<str>),
}

impl CompactName {
    /// Return the string content.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Owned(s) => s,
        }
    }
}

impl PartialEq for CompactName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CompactName {}

impl std::hash::Hash for CompactName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Display for CompactName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for CompactName {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl From<Box<str>> for CompactName {
    fn from(s: Box<str>) -> Self {
        Self::Owned(s)
    }
}

impl From<String> for CompactName {
    fn from(s: String) -> Self {
        Self::Owned(s.into_boxed_str())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CompactName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CompactName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::Owned(s.into_boxed_str()))
    }
}

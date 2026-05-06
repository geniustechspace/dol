//! Backend capability bitset.
//!
//! Each [`Operation`](crate::Operation) declares a set of features it
//! depends on; a backend declares a set of features it provides. `dol-check`
//! refuses any program whose required features aren't a subset of the
//! backend's provided features.
//!
//! Capabilities are an open enumeration encoded as a `u64` bitset. New bits
//! are appended; bits are never reused.

use core::fmt;
use core::ops::{BitAnd, BitOr, BitXor, Not};

/// Bitset of optional backend features.
///
/// Each bit represents a single capability. Combine via `|` (bitwise or).
/// Use [`BackendCapabilities::contains`] to test for presence.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BackendCapabilities(
    /// Raw 64-bit bitmask of enabled capabilities.
    pub u64,
);

macro_rules! cap_bits {
    ($( $(#[$m:meta])* $name:ident = $bit:expr ;)+) => {
        impl BackendCapabilities {
            $(
                $(#[$m])*
                pub const $name: BackendCapabilities = BackendCapabilities(1u64 << $bit);
            )+

            /// Empty set.
            pub const fn empty() -> Self { BackendCapabilities(0) }

            /// All known capabilities. Useful for tests and "do everything"
            /// in-memory backends.
            pub const ALL: BackendCapabilities = BackendCapabilities($(  (1u64 << $bit) | )+ 0);

            /// `true` if `self` contains every bit in `other`.
            pub const fn contains(self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }

            /// Bits in `self` that are missing from `other`.
            pub const fn difference(self, other: Self) -> Self {
                BackendCapabilities(self.0 & !other.0)
            }

            /// `true` iff no bits are set.
            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }
        }
    };
}

cap_bits! {
    /// Window functions (`ROW_NUMBER`, `RANK`, `OVER (...)`).
    WINDOW_FUNCTIONS = 0;
    /// Recursive CTEs (`WITH RECURSIVE`).
    RECURSIVE_CTE = 1;
    /// JSON path arrows (`->`, `->>`).
    JSON_ARROWS = 2;
    /// Vector-similarity indexes / operators.
    VECTOR_INDEX = 3;
    /// Geospatial predicates and indexes.
    GEOSPATIAL = 4;
    /// `MERGE` / `UPSERT` semantics.
    MERGE = 5;
    /// Row locking with `FOR UPDATE` / `FOR SHARE`.
    ROW_LOCKING = 6;
    /// `SKIP LOCKED` / `NOWAIT` lock hints.
    LOCK_SKIP_NOWAIT = 7;
    /// Streaming windows + watermarks (from `dol-stream`).
    STREAMING_WINDOWS = 8;
    /// Time-series ops (`time_bucket`, `gap_fill`, `locf`, …).
    TIME_SERIES = 9;
    /// Pipeline / dataflow programs (from `dol-pipeline`).
    PIPELINES = 10;
    /// Object-store statements (`PutObject`, `GetObject`, …).
    OBJECT_STORE = 11;
    /// Filesystem statements (`ReadFile`, `WriteFile`, …).
    FILE_IO = 12;
    /// Row-level security policies.
    POLICIES = 13;
    /// Open extension variant; backend agrees to consult its registry.
    EXTENSIONS = 14;
    /// `Replace` (full-overwrite) verb is supported.
    REPLACE_OP = 15;
    /// `Probe` (existence / metadata) verb is supported.
    PROBE_OP = 16;
    /// `Describe` (introspection) verb is supported.
    DESCRIBE_OP = 17;
    /// `Append` (append-only) verb is supported.
    APPEND_OP = 18;
    /// Multi-statement transactions are supported.
    MULTI_STATEMENT_TX = 19;
    /// Field-level masking policies.
    MASK_POLICIES = 20;
    /// Quotas / limits.
    QUOTAS = 21;
    /// Audit policies.
    AUDIT = 22;
    /// Opaque (un-resolved) schemas may be addressed.
    OPAQUE_SCHEMA = 23;
    /// `TargetKind::Blob` is supported.
    BLOB_TARGETS = 24;
    /// `TargetKind::FileTree` is supported.
    FILE_TREE_TARGETS = 25;
    /// `TargetKind::StreamTopic` is supported.
    STREAM_TARGETS = 26;
    /// `TargetKind::ApiResource` is supported.
    API_TARGETS = 27;
    /// Feature-gated [`Raw`](crate::operation::Operation::Raw) escape hatch
    /// is permitted.
    RAW_PASSTHROUGH = 28;
}

impl BitOr for BackendCapabilities {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl BitAnd for BackendCapabilities {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
impl BitXor for BackendCapabilities {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}
impl Not for BackendCapabilities {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl fmt::Debug for BackendCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BackendCapabilities(0x{:016x})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_difference() {
        let a = BackendCapabilities::WINDOW_FUNCTIONS | BackendCapabilities::JSON_ARROWS;
        let b = BackendCapabilities::WINDOW_FUNCTIONS;
        assert!(a.contains(b));
        assert!(!b.contains(a));
        assert_eq!(a.difference(b), BackendCapabilities::JSON_ARROWS);
    }

    #[test]
    fn empty_and_all_consistent() {
        assert!(BackendCapabilities::empty().is_empty());
        assert!(BackendCapabilities::ALL.contains(BackendCapabilities::EXTENSIONS));
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Open `CapabilityTag` / `CapabilitySet` vocabulary + `CapabilityCheck`
// ──────────────────────────────────────────────────────────────────────────
//
// The `u64` `BackendCapabilities` bitset above stays for the hot path.
// `CapabilityTag` is the open, forward-compatible vocabulary used by
// `dol-check` for diagnostics and by extensions to declare requirements
// without consuming a precious bit.

use alloc::vec::Vec;

extern crate alloc;

/// Open capability tag.
///
/// Tags have a stable string label. Built-in tags use a `&'static` label so
/// constants can be defined at compile time; user-minted tags carry an
/// owned `String`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CapabilityTag(
    /// Inner string: borrowed for built-ins, owned for extensions.
    alloc::borrow::Cow<'static, str>,
);

impl CapabilityTag {
    /// Build a tag from a `&'static str`. Used by the built-in constants.
    pub const fn builtin(s: &'static str) -> Self {
        Self(alloc::borrow::Cow::Borrowed(s))
    }

    /// Build an extension tag from an owned string.
    pub fn extension(s: alloc::string::String) -> Self {
        Self(alloc::borrow::Cow::Owned(s))
    }

    /// Window functions (`ROW_NUMBER`, `RANK`, `OVER (...)`).
    pub const WINDOW_FUNCTIONS: CapabilityTag = CapabilityTag::builtin("WINDOW_FUNCTIONS");
    /// Recursive CTEs (`WITH RECURSIVE`).
    pub const RECURSIVE_CTE: CapabilityTag = CapabilityTag::builtin("RECURSIVE_CTE");
    /// JSON path arrows (`->`, `->>`).
    pub const JSON_ARROWS: CapabilityTag = CapabilityTag::builtin("JSON_ARROWS");
    /// Vector-similarity indexes / operators.
    pub const VECTOR_INDEX: CapabilityTag = CapabilityTag::builtin("VECTOR_INDEX");
    /// Geospatial predicates and indexes.
    pub const GEOSPATIAL: CapabilityTag = CapabilityTag::builtin("GEOSPATIAL");
    /// `MERGE` / `UPSERT` semantics.
    pub const MERGE: CapabilityTag = CapabilityTag::builtin("MERGE");
    /// Row locking with `FOR UPDATE` / `FOR SHARE`.
    pub const ROW_LOCKING: CapabilityTag = CapabilityTag::builtin("ROW_LOCKING");
    /// `SKIP LOCKED` / `NOWAIT` lock hints.
    pub const LOCK_SKIP_NOWAIT: CapabilityTag = CapabilityTag::builtin("LOCK_SKIP_NOWAIT");
    /// Streaming windows + watermarks (from `dol-stream`).
    pub const STREAMING_WINDOWS: CapabilityTag = CapabilityTag::builtin("STREAMING_WINDOWS");
    /// Time-series ops (`time_bucket`, `gap_fill`, `locf`, …).
    pub const TIME_SERIES: CapabilityTag = CapabilityTag::builtin("TIME_SERIES");
    /// Pipeline / dataflow programs (from `dol-pipeline`).
    pub const PIPELINES: CapabilityTag = CapabilityTag::builtin("PIPELINES");
    /// Object-store statements (`PutObject`, `GetObject`, …).
    pub const OBJECT_STORE: CapabilityTag = CapabilityTag::builtin("OBJECT_STORE");
    /// Filesystem statements (`ReadFile`, `WriteFile`, …).
    pub const FILE_IO: CapabilityTag = CapabilityTag::builtin("FILE_IO");
    /// Row-level security policies.
    pub const POLICIES: CapabilityTag = CapabilityTag::builtin("POLICIES");
    /// Open extension variant; backend agrees to consult its registry.
    pub const EXTENSIONS: CapabilityTag = CapabilityTag::builtin("EXTENSIONS");
    /// `Replace` (full-overwrite) verb is supported.
    pub const REPLACE_OP: CapabilityTag = CapabilityTag::builtin("REPLACE_OP");
    /// `Probe` (existence / metadata) verb is supported.
    pub const PROBE_OP: CapabilityTag = CapabilityTag::builtin("PROBE_OP");
    /// `Describe` (introspection) verb is supported.
    pub const DESCRIBE_OP: CapabilityTag = CapabilityTag::builtin("DESCRIBE_OP");
    /// `Append` (append-only) verb is supported.
    pub const APPEND_OP: CapabilityTag = CapabilityTag::builtin("APPEND_OP");
    /// Multi-statement transactions are supported.
    pub const MULTI_STATEMENT_TX: CapabilityTag = CapabilityTag::builtin("MULTI_STATEMENT_TX");
    /// Field-level masking policies.
    pub const MASK_POLICIES: CapabilityTag = CapabilityTag::builtin("MASK_POLICIES");
    /// Quotas / limits.
    pub const QUOTAS: CapabilityTag = CapabilityTag::builtin("QUOTAS");
    /// Audit policies.
    pub const AUDIT: CapabilityTag = CapabilityTag::builtin("AUDIT");
    /// Opaque (un-resolved) schemas may be addressed.
    pub const OPAQUE_SCHEMA: CapabilityTag = CapabilityTag::builtin("OPAQUE_SCHEMA");
    /// `TargetKind::Blob` is supported.
    pub const BLOB_TARGETS: CapabilityTag = CapabilityTag::builtin("BLOB_TARGETS");
    /// `TargetKind::FileTree` is supported.
    pub const FILE_TREE_TARGETS: CapabilityTag = CapabilityTag::builtin("FILE_TREE_TARGETS");
    /// `TargetKind::StreamTopic` is supported.
    pub const STREAM_TARGETS: CapabilityTag = CapabilityTag::builtin("STREAM_TARGETS");
    /// `TargetKind::ApiResource` is supported.
    pub const API_TARGETS: CapabilityTag = CapabilityTag::builtin("API_TARGETS");
    /// Feature-gated [`Raw`](crate::operation::Operation::Raw) escape hatch.
    pub const RAW_PASSTHROUGH: CapabilityTag = CapabilityTag::builtin("RAW_PASSTHROUGH");

    /// Stable string label.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for CapabilityTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Open set of capability tags.
///
/// Unlike [`BackendCapabilities`], which is a fixed-size bitmask,
/// `CapabilitySet` can carry an unbounded number of tags (including
/// extension-specific tags) and is used for diagnostics and cross-crate
/// communication.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CapabilitySet {
    /// List of tags in insertion order; duplicates are rejected.
    tags: Vec<CapabilityTag>,
}

impl CapabilitySet {
    /// Create an empty capability set.
    pub const fn new() -> Self {
        Self { tags: Vec::new() }
    }

    /// Build a set from an iterator of tags, de-duplicating on insertion.
    pub fn build_from<I: IntoIterator<Item = CapabilityTag>>(it: I) -> Self {
        let mut s = Self::new();
        for t in it {
            s.insert(t);
        }
        s
    }

    /// Insert a tag. Returns `true` if the tag was newly added.
    pub fn insert(&mut self, tag: CapabilityTag) -> bool {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            true
        } else {
            false
        }
    }

    /// Check whether the set contains the given tag.
    pub fn contains(&self, tag: &CapabilityTag) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// `true` if no tags are present.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Number of tags in the set.
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Iterate over tags in insertion order.
    pub fn iter(&self) -> core::slice::Iter<'_, CapabilityTag> {
        self.tags.iter()
    }

    /// Tags in `self` that are not in `provided`.
    pub fn missing_from(&self, provided: &CapabilitySet) -> Vec<CapabilityTag> {
        self.tags
            .iter()
            .filter(|t| !provided.contains(t))
            .cloned()
            .collect()
    }
}

impl FromIterator<CapabilityTag> for CapabilitySet {
    fn from_iter<I: IntoIterator<Item = CapabilityTag>>(iter: I) -> Self {
        Self::build_from(iter)
    }
}

/// Structured capability check key produced by `dol-check`.
///
/// Used to render uniform diagnostics like
/// *"backend `pg` does not support `Append` on `TargetKind::StreamTopic`"*.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CapabilityCheck {
    /// The operation kind being checked.
    pub op: crate::operation::OpKind,
    /// The target kind, if relevant to the check.
    pub target: Option<crate::target::TargetKind>,
    /// The structural verb, if the operation is noun-shaped.
    pub verb: Option<crate::operation::StructuralVerb>,
}

impl CapabilityCheck {
    /// Create a check for a single operation kind.
    pub const fn new(op: crate::operation::OpKind) -> Self {
        Self {
            op,
            target: None,
            verb: None,
        }
    }

    /// Attach a target kind to the check.
    pub fn with_target(mut self, target: crate::target::TargetKind) -> Self {
        self.target = Some(target);
        self
    }

    /// Attach a structural verb to the check.
    pub fn with_verb(mut self, verb: crate::operation::StructuralVerb) -> Self {
        self.verb = Some(verb);
        self
    }
}

// ── Operation::required_capabilities() ────────────────────────────────────

impl crate::operation::Operation {
    /// Capability tags required by this operation. Backends compare this set
    /// to their declared `provides()` set; missing tags become diagnostics.
    pub fn required_capabilities(&self) -> CapabilitySet {
        use crate::operation::Operation as O;
        let mut set = CapabilitySet::new();

        // Verb-driven tags.
        match self {
            O::Replace(_) => {
                set.insert(CapabilityTag::REPLACE_OP);
            }
            O::Probe(_) => {
                set.insert(CapabilityTag::PROBE_OP);
            }
            O::Describe(_) => {
                set.insert(CapabilityTag::DESCRIBE_OP);
            }
            O::Append(_) => {
                set.insert(CapabilityTag::APPEND_OP);
            }
            O::Upsert(_) => {
                set.insert(CapabilityTag::MERGE);
            }
            O::Policy(_) => {
                set.insert(CapabilityTag::POLICIES);
            }
            O::Mask(_) => {
                set.insert(CapabilityTag::MASK_POLICIES);
            }
            O::Quota(_) => {
                set.insert(CapabilityTag::QUOTAS);
            }
            O::Audit(_) => {
                set.insert(CapabilityTag::AUDIT);
            }
            O::Tx(tx) => {
                if matches!(
                    **tx,
                    crate::operation::TxOp::Atomic { .. } | crate::operation::TxOp::Begin(_)
                ) {
                    set.insert(CapabilityTag::MULTI_STATEMENT_TX);
                }
            }
            O::Extension(_) => {
                set.insert(CapabilityTag::EXTENSIONS);
            }
            #[cfg(feature = "raw")]
            O::Raw(_) => {
                set.insert(CapabilityTag::RAW_PASSTHROUGH);
            }
            _ => {}
        }

        // Target-driven tags.
        if let Some(target) = self.primary_target() {
            use crate::target::TargetKind as TK;
            match &target.kind {
                TK::Blob => {
                    set.insert(CapabilityTag::BLOB_TARGETS);
                }
                TK::FileTree => {
                    set.insert(CapabilityTag::FILE_TREE_TARGETS);
                }
                TK::StreamTopic => {
                    set.insert(CapabilityTag::STREAM_TARGETS);
                }
                TK::ApiResource => {
                    set.insert(CapabilityTag::API_TARGETS);
                }
                _ => {}
            }
            if matches!(target.schema, crate::target::SchemaBinding::Opaque) {
                set.insert(CapabilityTag::OPAQUE_SCHEMA);
            }
        }

        set
    }
}

#[cfg(test)]
mod v2_tests {
    use super::*;
    use crate::operation::{Insert, InsertSource, Operation};
    use crate::target::{Locator, Symbol, Target, TargetKind};

    #[test]
    fn append_on_stream_topic_requires_append_and_stream_tags() {
        let op: Operation = crate::operation::Append {
            target: Target::new(TargetKind::StreamTopic, Locator::new(Symbol::from_hash(0))),
            source: InsertSource::Bindings,
            partition_key: None,
        }
        .into();
        let req = op.required_capabilities();
        assert!(req.contains(&CapabilityTag::APPEND_OP));
        assert!(req.contains(&CapabilityTag::STREAM_TARGETS));
    }

    #[test]
    fn plain_insert_has_no_required_tags() {
        let op: Operation = Insert {
            target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0)))
                .with_schema(crate::target::SchemaBinding::Inferred),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        assert!(op.required_capabilities().is_empty());
    }

    #[test]
    fn capability_check_builders() {
        let c = CapabilityCheck::new(crate::operation::OpKind::Append)
            .with_target(TargetKind::StreamTopic);
        assert_eq!(c.op, crate::operation::OpKind::Append);
        assert_eq!(c.target, Some(TargetKind::StreamTopic));
    }
}

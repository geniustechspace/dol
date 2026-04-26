//! Universal addressing primitives for the IR.
//!
//! Every [`Operation`](crate::operation::Operation) applies to a [`Target`].
//! A `Target` describes *what* is being addressed (`TargetKind`), *where* it
//! lives (`Locator`), an optional alias used by the surrounding query, and
//! how its schema is bound (`SchemaBinding`).
//!
//! See `docs/rfcs/0001-ir.md` for the full design.

use smallvec::SmallVec;

use crate::schema_ref::SchemaRef;

/// Interned name handle.
///
/// `Symbol` is a transparent newtype over [`dol_expr::ids::StrId`]. Every
/// name in an [`Operation`](crate::operation::Operation) — locator namespace,
/// locator name, locator path segment, target alias, role name, savepoint
/// label, custom target-kind tag — uses `Symbol` so addressing has no
/// per-operation allocations and equality is a single integer compare.
///
/// A `Symbol` is meaningful only relative to the [`dol_expr::Interner`] that
/// produced it. Resolving a `Symbol` to its string form requires the same
/// interner that interned it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Symbol(pub dol_expr::ids::StrId);

impl Symbol {
    /// Wrap a raw interner id.
    #[inline]
    pub const fn new(id: dol_expr::ids::StrId) -> Self {
        Self(id)
    }

    /// Underlying interner id.
    #[inline]
    pub const fn id(self) -> dol_expr::ids::StrId {
        self.0
    }

    /// Resolve the symbol against `interner`. Panics if the id is unknown,
    /// matching the existing `Interner::get` contract.
    #[inline]
    pub fn resolve(self, interner: &dol_expr::interner::Interner) -> &str {
        interner.get(self.0)
    }
}

impl From<dol_expr::ids::StrId> for Symbol {
    #[inline]
    fn from(id: dol_expr::ids::StrId) -> Self {
        Self(id)
    }
}

impl From<Symbol> for dol_expr::ids::StrId {
    #[inline]
    fn from(s: Symbol) -> Self {
        s.0
    }
}

/// Structured address of a target object.
///
/// `Locator` is data, not a string. The optional `namespace` covers SQL
/// schemas, KV namespaces, S3 buckets, file roots, and message-queue
/// vhosts. `name` is the primary identifier (table/collection/bucket/topic).
/// `path` covers nested document paths, S3 key segments, file-tree segments,
/// and topic partitions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Locator {
    /// Optional containing namespace (schema, bucket, vhost, …).
    pub namespace: Option<Symbol>,
    /// Primary name within the namespace.
    pub name: Symbol,
    /// Nested path segments (document path, key segments, partitions, …).
    pub path: SmallVec<[Symbol; 2]>,
}

impl Locator {
    /// Build a bare locator with just a `name`.
    #[inline]
    pub fn new(name: Symbol) -> Self {
        Self {
            namespace: None,
            name,
            path: SmallVec::new(),
        }
    }

    /// Set the namespace.
    #[inline]
    pub fn with_namespace(mut self, ns: Symbol) -> Self {
        self.namespace = Some(ns);
        self
    }

    /// Append a single path segment.
    #[inline]
    pub fn with_segment(mut self, seg: Symbol) -> Self {
        self.path.push(seg);
        self
    }

    /// Replace the path.
    #[inline]
    pub fn with_path<I: IntoIterator<Item = Symbol>>(mut self, path: I) -> Self {
        self.path = path.into_iter().collect();
        self
    }
}

/// What family of object a [`Target`] refers to.
///
/// Backends use `TargetKind` to decide how to lower the surrounding
/// [`Operation`](crate::operation::Operation). Some verbs are only meaningful
/// on certain kinds — capability checks pair `(OpKind, TargetKind)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TargetKind {
    /// Tabular relation (SQL table, dataframe).
    Relation,
    /// Document collection (Mongo, Couch, JSON store).
    Document,
    /// Key-value bucket.
    KeyValue,
    /// Object-store blob (S3, GCS, Azure Blob).
    Blob,
    /// Filesystem tree.
    FileTree,
    /// HTTP / RPC API resource.
    ApiResource,
    /// Stream or message-queue topic.
    StreamTopic,
    /// Computed or virtual target (view, transform, in-memory binding).
    Virtual,
    /// Backend-specific kind tagged by an interned name.
    Custom(Symbol),
}

/// How the schema of a [`Target`] is known.
///
/// `Opaque` is significant: it lets DOL represent operations against systems
/// where the schema is unknown, unenforced, dynamic, or intentionally
/// ignored. `dol-check` downgrades schema-dependent diagnostics to warnings
/// when it sees `Opaque`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SchemaBinding {
    /// Schema is declared and resolvable through the program's catalog.
    Declared(SchemaRef),
    /// Schema is not declared but can be inferred from data or the backend.
    Inferred,
    /// Schema is unknown / unenforced / intentionally ignored.
    Opaque,
}

/// Universal target descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Target {
    pub kind: TargetKind,
    pub locator: Locator,
    pub alias: Option<Symbol>,
    pub schema: SchemaBinding,
}

impl Target {
    /// Build a target with `Opaque` schema binding and no alias.
    #[inline]
    pub fn new(kind: TargetKind, locator: Locator) -> Self {
        Self {
            kind,
            locator,
            alias: None,
            schema: SchemaBinding::Opaque,
        }
    }

    /// Set the alias.
    #[inline]
    pub fn with_alias(mut self, alias: Symbol) -> Self {
        self.alias = Some(alias);
        self
    }

    /// Set the schema binding.
    #[inline]
    pub fn with_schema(mut self, schema: SchemaBinding) -> Self {
        self.schema = schema;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_builders() {
        let ns = Symbol::new(0);
        let name = Symbol::new(1);
        let seg = Symbol::new(2);

        let loc = Locator::new(name).with_namespace(ns).with_segment(seg);
        assert_eq!(loc.namespace, Some(ns));
        assert_eq!(loc.name, name);
        assert_eq!(loc.path.as_slice(), &[seg]);
    }

    #[test]
    fn target_defaults_to_opaque() {
        let t = Target::new(TargetKind::Relation, Locator::new(Symbol::new(7)));
        assert_eq!(t.schema, SchemaBinding::Opaque);
        assert!(t.alias.is_none());
    }

    #[test]
    fn symbol_round_trips_through_strid() {
        let s = Symbol::from(42u32);
        let raw: dol_expr::ids::StrId = s.into();
        assert_eq!(raw, 42);
        assert_eq!(s.id(), 42);
    }
}

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Symbol(pub dol_expr::ids::StrId);

impl Default for Symbol {
    /// A placeholder id (`StrId(1)`) used by test fixtures that don't
    /// care which name a locator resolves to. Not a meaningful "empty"
    /// name — production code should always construct symbols via
    /// `Interner::intern`.
    //
    // Lint exemption: `StrId::from_u32(1)` is infallible by construction
    // (1 is a valid `NonZeroU32`); the `expect` documents the invariant
    // without introducing a runtime failure path.
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self(dol_expr::ids::StrId::from_u32(1).expect("StrId(1) is non-zero"))
    }
}

impl Symbol {
    /// Wrap a raw interner id.
    #[inline]
    pub const fn new(id: dol_expr::ids::StrId) -> Self {
        Self(id)
    }

    /// Build a [`Symbol`] from a 32-bit content hash (e.g. the FNV-1a
    /// digest used by extension `*_SYMBOL` constants). Folds the
    /// all-zero hash to `1` so the result fits the `NonZeroU32` niche
    /// that backs `StrId`.
    ///
    /// `const` so extension crates can publish their `Symbol` ids as
    /// `pub const X: Symbol = Symbol::from_hash(fnv1a_32(NAME.as_bytes()));`.
    //
    // Lint exemption: the `panic!` arm is unreachable — `raw` is folded to
    // `1` when `hash == 0`, so the input to `from_u32` is always non-zero.
    // The arm exists because `const fn` cannot use `?` or `.expect()`; it
    // documents the invariant without introducing a runtime failure path.
    #[allow(clippy::panic)]
    #[inline]
    pub const fn from_hash(hash: u32) -> Self {
        let raw = if hash == 0 { 1 } else { hash };
        // Safety-equivalent: `raw` is non-zero by construction; using
        // `from_u32` keeps the crate `#![forbid(unsafe_code)]`-clean.
        match dol_expr::ids::StrId::from_u32(raw) {
            Some(id) => Self(id),
            None => panic!("Symbol::from_hash: non-zero u32 unexpectedly None"),
        }
    }

    /// Underlying interner id.
    #[inline]
    pub const fn id(self) -> dol_expr::ids::StrId {
        self.0
    }

    /// Resolve the symbol against `interner`. Panics if the id is
    /// unknown; callers handling untrusted ids should use
    /// [`Interner::get_opt`](dol_expr::interner::Interner::get_opt)
    /// directly.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SchemaBinding {
    /// Schema is declared and resolvable through the program's catalog.
    Declared(SchemaRef),
    /// Schema is not declared but can be inferred from data or the backend.
    Inferred,
    /// Schema is unknown / unenforced / intentionally ignored.
    Opaque,
}

/// Universal target descriptor.
///
/// Combines a [`TargetKind`], [`Locator`], optional alias, and
/// [`SchemaBinding`] into a single addressable entity.
///
/// # Examples
///
/// ```
/// use dol_ir::target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
///
/// // SQL table "public.users" with alias "u"
/// let t = Target::new(
///     TargetKind::Relation,
///     Locator::new(Symbol::from_hash(1)).with_namespace(Symbol::from_hash(0)),
/// )
/// .with_alias(Symbol::from_hash(2))
/// .with_schema(SchemaBinding::Inferred);
///
/// assert_eq!(t.kind, TargetKind::Relation);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Target {
    /// What family of object this target refers to.
    pub kind: TargetKind,
    /// Structured address (namespace, name, path).
    pub locator: Locator,
    /// Optional alias used by the surrounding query (e.g. `FROM users AS u`).
    pub alias: Option<Symbol>,
    /// How the schema of this target is known.
    pub schema: SchemaBinding,
}

impl Target {
    /// Build a target with `Inferred` schema binding and no alias.
    ///
    /// `Inferred` is the friendlier default for builders that don't yet know
    /// the program's catalog: backends and `dol-check` are free to derive
    /// the schema from the data or backend metadata. Pass
    /// [`with_schema(SchemaBinding::Opaque)`](Self::with_schema) explicitly
    /// for sinks that intentionally ignore the schema.
    #[inline]
    pub fn new(kind: TargetKind, locator: Locator) -> Self {
        Self {
            kind,
            locator,
            alias: None,
            schema: SchemaBinding::Inferred,
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
        let ns = Symbol::from_hash(0);
        let name = Symbol::from_hash(1);
        let seg = Symbol::from_hash(2);

        let loc = Locator::new(name).with_namespace(ns).with_segment(seg);
        assert_eq!(loc.namespace, Some(ns));
        assert_eq!(loc.name, name);
        assert_eq!(loc.path.as_slice(), &[seg]);
    }

    #[test]
    fn target_defaults_to_inferred() {
        let t = Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(7)));
        assert_eq!(t.schema, SchemaBinding::Inferred);
        assert!(t.alias.is_none());
    }

    #[test]
    fn target_explicit_opaque() {
        let t = Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(7)))
            .with_schema(SchemaBinding::Opaque);
        assert_eq!(t.schema, SchemaBinding::Opaque);
    }

    #[test]
    fn symbol_round_trips_through_strid() {
        let raw: dol_expr::ids::StrId = dol_expr::ids::StrId::from_u32(42).unwrap();
        let s = Symbol::new(raw);
        let back: dol_expr::ids::StrId = s.into();
        assert_eq!(back, raw);
        assert_eq!(s.id(), raw);
    }
}

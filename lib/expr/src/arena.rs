use alloc::vec::Vec;

use hashbrown::HashMap;
use smallvec::SmallVec;

use dol_core::strings::StrId;
use dol_core::{Id, Literal};

use crate::expr::{DeleteNode, ExprNode, InsertNode, Order, QueryNode, UpdateNode, UpsertNode};
use crate::ids::{
    CaseId, CompositeId, DeleteId, FieldId, FuncId, InsertId, LiteralId, NodeId, QueryId, SpanId,
    UpdateId, UpsertId, WindowId,
};

/// Allocate `item` in `vec` and return its [`Id<Tag>`].
///
/// `expect` is used for the >4 G overflow case: every arena pool is
/// `Vec`-backed, and a 4 G expression-node arena would already exhaust
/// 32 GiB of RAM at the smallest node size. The panic exists as a
/// design-time error; production callers cannot realistically reach it.
//
// Lint exemption: this is the documented "design-time error" arm. v2's
// no-panic invariant carves out documented-invariant panics; the
// equivalent fallible accessor would be a private wrapper that no public
// caller could trigger.
#[inline]
#[track_caller]
#[allow(clippy::expect_used)]
fn alloc_in<T, Tag: ?Sized>(vec: &mut Vec<T>, item: T) -> Id<Tag> {
    let idx = vec.len();
    vec.push(item);
    Id::from_index(idx).expect("dol-expr: arena pool overflow (>= 4 G entries)")
}

/// Borrow the slot at `id` from `vec`, panicking on out-of-range — same
/// "design-time error" contract as the public `ExprArena::get_*`
/// accessors.
#[inline]
#[track_caller]
// Ids are produced by `push` and stored alongside the same `vec`; an
// out-of-range index is a programmer error documented as such on the
// callers (`ExprArena::get_*`).
#[allow(clippy::indexing_slicing)]
fn get_in<T, Tag: ?Sized>(vec: &[T], id: Id<Tag>) -> &T {
    &vec[id.index()]
}

// ─── FieldStep / FieldNode ───────────────────────────────────────────────────

/// A single traversal step inside a [`FieldNode`] path.
///
/// Steps allow a [`crate::expr::ExprOp::Field`] to express arbitrarily deep navigation
/// through JSON/object structures or arrays, independent of the backing store:
///
/// | Step | Postgres JSONB              | REST / document |
/// |---|---------------------------------|-----------------|
/// | `Key("name")` | `col->>'name'`     | `.name`         |
/// | `Index(0)`    | `col->>0`          | `[0]`           |
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum FieldStep {
    /// Named key access: `.key`, `->>'key'`, `["key"]`.
    Key(StrId),
    /// Positional array index: `[n]`, `->>n`.
    Index(u32),
}

/// Payload for [`crate::expr::ExprOp::Field`], stored in `ExprArena::fields`.
///
/// A `Field` is a leaf reference: a named attribute optionally anchored on
/// a container [`crate::expr::ExprOp::Namespace`] (whose dotted address is interned as
/// `namespace`), with an optional traversal chain that follows the leaf
/// (e.g. JSON key / index access).
///
/// # Examples
///
/// | `namespace` | `name`           | `steps`                | Rendered (Postgres)         |
/// |-------------|------------------|------------------------|-----------------------------|
/// | `None`      | `"id"`           | `[]`                   | `id`                        |
/// | `Some("u")` | `"id"`           | `[]`                   | `u.id`                      |
/// | `None`      | `"profile_json"` | `[Key("name")]`        | `profile_json->>'name'`     |
/// | `Some("u")` | `"data"`         | `[Key("x"), Index(0)]` | `u.data->'x'->>0`           |
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FieldNode {
    /// Optional container address (interned dotted path); `None` means an
    /// unanchored leaf reference at the current scope.
    pub namespace: Option<StrId>,
    /// Leaf attribute name.
    pub name: StrId,
    /// Traversal chain that follows the leaf (empty = bare leaf reference).
    pub steps: SmallVec<[FieldStep; 4]>,
}

// ─── Pooled payload structs ───────────────────────────────────────────────────

/// Payload for [`crate::expr::ExprOp::Func`], stored in `ExprArena::funcs`.
///
/// Moved out of the enum variant to keep `ExprNode` ≤ 32 bytes: the
/// two fields (`name: u32` + 4-byte alignment gap + 24-byte `SmallVec`)
/// would otherwise push the variant to 32 bytes of *payload*, which
/// combined with the discriminant word exceeds the target.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FuncNode {
    pub name: StrId,
    pub args: SmallVec<[NodeId; 4]>,
}

/// Discriminator for the unified [`CompositeNode`] container.
///
/// Three structural shapes share one side-pool:
///
/// * [`CompositeKind::Array`] — homogeneous list literal (`[1, 2, 3]`),
///   item keys are `None`.
/// * [`CompositeKind::Object`] — keyed object literal
///   (`{ "x": 1, "y": 2 }`), every item key is `Some(StrId)`.
/// * [`CompositeKind::Tuple`] — fixed-arity row literal
///   (`(a, b, c)` in `IN (a, b, c)`), keys are `None` but the kind
///   distinguishes the row form from a true list literal.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum CompositeKind {
    Array = 0,
    Object = 1,
    Tuple = 2,
}

impl CompositeKind {
    /// Convert from a raw `u8` (e.g. wire byte).
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Array,
            1 => Self::Object,
            2 => Self::Tuple,
            _ => return None,
        })
    }
}

/// Payload for [`crate::expr::ExprOp::Composite`], stored in
/// `ExprArena::composites`.
///
/// Collapses the v2-prototype `ObjectLit`, `ArrayLit`, and `InList`
/// side-pools onto a single record. Each item is a
/// `(Option<StrId>, NodeId)` pair so an `Object` can stash its keys
/// alongside its values without a parallel vector. For `Array` and
/// `Tuple`, the key is always `None`.
///
/// The `Tuple` variant is what lets `IN (a, b, c)` lower to
/// `In { probe, collection: Composite::Tuple(a, b, c) }` cleanly: the
/// row form is structurally distinct from a true array literal.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CompositeNode {
    /// Which structural shape this is (Array / Object / Tuple).
    pub kind: CompositeKind,
    /// Items in source order. Object items carry a `Some(StrId)` key;
    /// Array / Tuple items carry `None`.
    pub items: SmallVec<[(Option<StrId>, NodeId); 4]>,
}

/// Payload for [`crate::expr::ExprOp::Window`], stored in `ExprArena::windows`.
///
/// Two `SmallVec` fields (each 24 bytes) plus `func: StrId` total 52+ bytes
/// of payload — pooled to keep `ExprNode` ≤ 32 bytes.
///
/// `frame` carries the optional `ROWS/RANGE BETWEEN ...` clause; lowering
/// preserves it so backends can render frames faithfully without
/// round-trip loss.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WindowNode {
    pub func: StrId,
    pub partition: SmallVec<[NodeId; 4]>,
    pub order: SmallVec<[(NodeId, Order); 2]>,
    /// Optional frame specification (`ROWS/RANGE BETWEEN start [AND end]`).
    pub frame: Option<crate::tree::WindowFrame>,
}

/// Payload for [`crate::expr::ExprOp::Case`], stored in `ExprArena::cases`.
///
/// `SmallVec<[(NodeId, NodeId); 4]>` has a 32-byte inline buffer, making the
/// variant payload 36+ bytes — pooled to keep `ExprNode` ≤ 32 bytes.
///
/// `else_` is `Option<NodeId>` (4 bytes via niche): `None` for an absent
/// `ELSE` branch, replacing the previous `NULL_NODE: NodeId = u32::MAX`
/// sentinel.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CaseNode {
    pub branches: SmallVec<[(NodeId, NodeId); 4]>,
    pub else_: Option<NodeId>,
}

// ─── SpanTable ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// Sparse association of [`Span`]s to the [`NodeId`]s that produced them.
///
/// Earlier revisions stored only `Vec<Span>`, indexed by `SpanId`, and
/// silently discarded the [`NodeId`] argument supplied to
/// [`ExprArena::attach_span`]. As soon as any allocator other than
/// `attach_span` ran in between, the positional id and the requested
/// `NodeId` diverged and downstream tooling (diagnostics, IDE-style error
/// reporting) could not safely resolve a node back to its source range.
///
/// The table now keeps one `(NodeId, Span)` entry per call. `get(SpanId)`
/// preserves the original by-position lookup, and the new
/// [`SpanTable::get_for`] resolves a span by its owning `NodeId` (a small
/// linear scan; spans are sparse on real plans). Both the wire form and
/// the public push/get signatures stay the same.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SpanTable {
    /// Spans, in insertion order.
    spans: Vec<Span>,
    /// Owning `NodeId` for each span at the same index.
    owners: Vec<NodeId>,
}

impl SpanTable {
    /// Append `span` and record it as belonging to `owner`.
    ///
    /// Returns the [`SpanId`] (positional index) of the new entry.
    //
    // Lint exemption: same documented "design-time error" carve-out as
    // `alloc_in`; reaching the >4 G case requires hundreds of GiB of
    // span entries.
    #[allow(clippy::expect_used)]
    pub fn push(&mut self, owner: NodeId, span: Span) -> SpanId {
        let idx = self.spans.len();
        self.spans.push(span);
        self.owners.push(owner);
        SpanId::from_index(idx).expect("dol-expr: SpanTable overflow")
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.spans.get(id.index())
    }

    /// Resolve a [`NodeId`] to its most-recently-attached [`Span`], if any.
    ///
    /// Returns `None` for nodes with no recorded span. Walks back-to-front
    /// so the latest attachment wins when callers re-attach spans during
    /// rewriting.
    ///
    /// **Complexity:** `O(n)` in the number of recorded spans. Spans are
    /// expected to be sparse on real plans (parsers attach one span per
    /// produced node, optimisers do not attach spans at all), so a linear
    /// scan is intentional. Workloads that need dense `O(1)` lookup
    /// should build a side-index from `(NodeId, SpanId)` pairs.
    pub fn get_for(&self, owner: NodeId) -> Option<&Span> {
        self.owners
            .iter()
            .rev()
            .position(|&o| o == owner)
            // `rev_idx` < `self.spans.len()` (returned by `position`); the
            // arithmetic mirrors that bound and the indexing is safe.
            .map(|rev_idx| {
                #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
                &self.spans[self.spans.len() - 1 - rev_idx]
            })
    }

    /// Total number of recorded spans.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Whether no spans have been recorded.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn spans_slice(&self) -> &[Span] {
        &self.spans
    }
    pub fn owners_slice(&self) -> &[NodeId] {
        &self.owners
    }
}

// ─── Capacity hints ──────────────────────────────────────────────────────────

/// Pre-allocation hints for [`ExprArena::with_capacity`].
///
/// All fields default to `0` so callers may set only the pools they care
/// about. The arena uses these to call `Vec::with_capacity` per pool,
/// avoiding a flurry of small reallocations during build.
///
/// Sensible defaults for an interactive query plan are exposed via
/// [`Capacity::small`] and [`Capacity::medium`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Capacity {
    pub nodes: usize,
    pub fields: usize,
    pub funcs: usize,
    /// Pre-alloc for the unified array / object / tuple side-pool
    /// (replaces the legacy `obj_lits`, `array_lits`, `in_lists`
    /// fields).
    pub composites: usize,
    pub windows: usize,
    pub cases: usize,
    pub queries: usize,
    pub inserts: usize,
    pub updates: usize,
    pub deletes: usize,
    pub upserts: usize,
    pub lits: usize,
}

impl Capacity {
    /// Heuristic sizing for short DSL expressions (≈ filter on one table).
    pub fn small() -> Self {
        Self {
            nodes: 32,
            fields: 8,
            funcs: 4,
            lits: 8,
            ..Self::default()
        }
    }

    /// Heuristic sizing for a typical SELECT with joins / aggregates.
    pub fn medium() -> Self {
        Self {
            nodes: 256,
            fields: 64,
            funcs: 32,
            lits: 64,
            queries: 1,
            composites: 4,
            cases: 4,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ExprArena {
    nodes: Vec<ExprNode>,
    span_table: SpanTable,
    /// Pooled literal values — lookup by [`LiteralId`].
    lits: Vec<Literal<'static>>,
    /// Pooled function-call payloads — lookup by [`FuncId`].
    funcs: Vec<FuncNode>,
    /// Pooled array / object / tuple composite literals — lookup by
    /// [`CompositeId`]. Replaces the legacy `obj_lits`, `array_lits`,
    /// and `in_lists` pools.
    composites: Vec<CompositeNode>,
    /// Pooled window-function payloads — lookup by [`WindowId`].
    windows: Vec<WindowNode>,
    /// Pooled CASE expression payloads — lookup by [`CaseId`].
    cases: Vec<CaseNode>,
    /// Pooled SELECT/query statement payloads — lookup by [`QueryId`].
    queries: Vec<QueryNode>,
    /// Pooled INSERT statement payloads — lookup by [`InsertId`].
    inserts: Vec<InsertNode>,
    /// Pooled UPDATE statement payloads — lookup by [`UpdateId`].
    updates: Vec<UpdateNode>,
    /// Pooled DELETE statement payloads — lookup by [`DeleteId`].
    deletes: Vec<DeleteNode>,
    /// Pooled UPSERT statement payloads — lookup by [`UpsertId`].
    upserts: Vec<UpsertNode>,
    /// Pooled field-reference payloads — lookup by [`FieldId`].
    fields: Vec<FieldNode>,
    /// Optional structural-dedup index for [`Self::intern_node`].
    ///
    /// `None` until the first `intern_node` call. Lazily initialised so
    /// the bulk single-shot `alloc` path on a one-off lowering does not
    /// pay for the hash table. Skipped by `Serialize` because it is
    /// recoverable from the `nodes` slice.
    #[cfg_attr(feature = "serde", serde(skip))]
    node_index: Option<HashMap<ExprNode, NodeId>>,
}

impl ExprArena {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an arena with pre-allocated pool capacities.
    ///
    /// Calling this avoids the small-reallocation traffic that otherwise
    /// dominates the build path for trivial expressions, especially the
    /// `nodes`, `fields`, and `lits` pools that grow fastest.
    pub fn with_capacity(cap: Capacity) -> Self {
        Self {
            nodes: Vec::with_capacity(cap.nodes),
            span_table: SpanTable::default(),
            lits: Vec::with_capacity(cap.lits),
            funcs: Vec::with_capacity(cap.funcs),
            composites: Vec::with_capacity(cap.composites),
            windows: Vec::with_capacity(cap.windows),
            cases: Vec::with_capacity(cap.cases),
            queries: Vec::with_capacity(cap.queries),
            inserts: Vec::with_capacity(cap.inserts),
            updates: Vec::with_capacity(cap.updates),
            deletes: Vec::with_capacity(cap.deletes),
            upserts: Vec::with_capacity(cap.upserts),
            fields: Vec::with_capacity(cap.fields),
            node_index: None,
        }
    }

    /// Total resident heap bytes across every pool — sum of each `Vec`'s
    /// capacity in bytes. Excludes the side data inside each pooled struct
    /// (e.g. a `SmallVec` overflow allocation) which would require
    /// recursing per-node; callers that need a tighter accounting should
    /// reach for `crate::stats::arena_stats`.
    // `usize` byte-count summation: every term is a `Vec` capacity bounded by
    // `isize::MAX` and the total is a heap-byte estimate, not a security
    // boundary; saturating at `usize::MAX` would still be a useful answer.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn heap_bytes(&self) -> usize {
        use core::mem::size_of;
        self.nodes.capacity() * size_of::<ExprNode>()
            + self.lits.capacity() * size_of::<Literal<'static>>()
            + self.funcs.capacity() * size_of::<FuncNode>()
            + self.composites.capacity() * size_of::<CompositeNode>()
            + self.windows.capacity() * size_of::<WindowNode>()
            + self.cases.capacity() * size_of::<CaseNode>()
            + self.queries.capacity() * size_of::<QueryNode>()
            + self.inserts.capacity() * size_of::<InsertNode>()
            + self.updates.capacity() * size_of::<UpdateNode>()
            + self.deletes.capacity() * size_of::<DeleteNode>()
            + self.upserts.capacity() * size_of::<UpsertNode>()
            + self.fields.capacity() * size_of::<FieldNode>()
            + self.span_table.spans.capacity() * size_of::<Span>()
            + self.span_table.owners.capacity() * size_of::<NodeId>()
    }

    // ── ExprNode pool ────────────────────────────────────────────────────────

    /// Append a packed [`ExprNode`] to the hot pool and return its
    /// [`NodeId`]. Most call sites should reach for the typed
    /// constructors on [`ExprNode`] (e.g. [`ExprNode::bin`]) to keep
    /// the raw `a`/`b`/`c` layout out of consumer code.
    pub fn alloc(&mut self, node: ExprNode) -> NodeId {
        let id = alloc_in(&mut self.nodes, node);
        // Keep the optional dedup index in sync so a later
        // `intern_node` for the same shape can find this entry. We
        // record on first-seen — duplicates that arrive via `alloc`
        // are intentionally kept distinct because the caller chose
        // the non-dedup entry point.
        if let Some(map) = self.node_index.as_mut() {
            map.entry(node).or_insert(id);
        }
        id
    }

    /// Allocate `node` if no structurally-equal node already exists in
    /// the hot pool, otherwise return the existing [`NodeId`].
    ///
    /// This is the **opt-in** dedup path. Bulk lowering uses
    /// [`Self::alloc`] (no hash cost); rewrites and CSE-style passes
    /// use this entry point so that common subexpressions collapse
    /// onto shared `NodeId`s.
    ///
    /// Structural equality is byte-for-byte over the 16-byte packed
    /// node. Two nodes are equal iff their `(op, flags, aux, a, b, c)`
    /// tuples match — which is the right semantics because side-pool
    /// referents (`FieldId`, `LiteralId`, …) are themselves keyed by
    /// content via their own pools.
    ///
    /// The dedup index is built lazily on first call and grows
    /// alongside the `nodes` pool from there. Calling [`Self::alloc`]
    /// after `intern_node` keeps the index in sync (the
    /// newly-allocated node is recorded so a subsequent `intern_node`
    /// can dedup against it).
    pub fn intern_node(&mut self, node: ExprNode) -> NodeId {
        // Lazily seed the dedup index from the existing `nodes` pool
        // so any nodes already allocated via the bulk path participate
        // in dedup from this point onward.
        let map = self.node_index.get_or_insert_with(|| {
            let mut map: HashMap<ExprNode, NodeId> = HashMap::with_capacity(self.nodes.len());
            for (idx, n) in self.nodes.iter().enumerate() {
                if let Some(id) = NodeId::from_index(idx) {
                    map.entry(*n).or_insert(id);
                }
            }
            map
        });
        if let Some(&existing) = map.get(&node) {
            return existing;
        }
        let id = alloc_in(&mut self.nodes, node);
        map.insert(node, id);
        id
    }

    // ── Typed `alloc_*` helpers — sugar for `alloc(ExprNode::*)`. ─────
    //
    // These mirror the constructors on `ExprNode` 1:1 and exist so
    // callers don't have to chain `arena.alloc(ExprNode::bin(op, l, r))`
    // everywhere. They keep the call sites readable now that the node
    // is opaque-ish (op + 14 bytes).

    /// Allocate a [`crate::expr::ExprOp::Bin`] node.
    pub fn alloc_bin(&mut self, op: crate::expr::BinOp, lhs: NodeId, rhs: NodeId) -> NodeId {
        self.alloc(ExprNode::bin(op, lhs, rhs))
    }
    /// Allocate a [`crate::expr::ExprOp::Una`] node.
    pub fn alloc_una(&mut self, op: crate::expr::UnaryOp, operand: NodeId) -> NodeId {
        self.alloc(ExprNode::una(op, operand))
    }
    /// Allocate a [`crate::expr::ExprOp::Field`] node referring to
    /// `id` in the field-payload pool.
    pub fn alloc_field_ref(&mut self, id: FieldId) -> NodeId {
        self.alloc(ExprNode::field(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Param`] node with positional
    /// index `0` (the legacy unindexed default).
    pub fn alloc_param(&mut self) -> NodeId {
        self.alloc(ExprNode::param(0))
    }
    /// Allocate a [`crate::expr::ExprOp::Lit`] node.
    pub fn alloc_lit_ref(&mut self, id: LiteralId) -> NodeId {
        self.alloc(ExprNode::lit(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Namespace`] node.
    pub fn alloc_namespace(&mut self, id: StrId) -> NodeId {
        self.alloc(ExprNode::namespace(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Composite`] reference node.
    /// `id` points into `ExprArena::composites`.
    pub fn alloc_composite_ref(&mut self, id: CompositeId) -> NodeId {
        self.alloc(ExprNode::composite(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Func`] node.
    pub fn alloc_func_ref(&mut self, id: FuncId) -> NodeId {
        self.alloc(ExprNode::func(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Agg`] node.
    pub fn alloc_agg(&mut self, func: StrId, expr: NodeId, distinct: bool) -> NodeId {
        self.alloc(ExprNode::agg(func, expr, distinct))
    }
    /// Allocate a [`crate::expr::ExprOp::Window`] node.
    pub fn alloc_window_ref(&mut self, id: WindowId) -> NodeId {
        self.alloc(ExprNode::window(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Cast`] node.
    pub fn alloc_cast(&mut self, expr: NodeId, to: StrId) -> NodeId {
        self.alloc(ExprNode::cast(expr, to))
    }
    /// Allocate a [`crate::expr::ExprOp::Case`] node.
    pub fn alloc_case_ref(&mut self, id: CaseId) -> NodeId {
        self.alloc(ExprNode::case(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Alias`] node.
    pub fn alloc_alias(&mut self, expr: NodeId, name: StrId) -> NodeId {
        self.alloc(ExprNode::alias(expr, name))
    }
    /// Allocate a [`crate::expr::ExprOp::In`] node. `collection` is a
    /// [`NodeId`] referring to the right-hand side; its opcode
    /// discriminates the form (Composite / Query / Param / Field).
    pub fn alloc_in(&mut self, probe: NodeId, collection: NodeId) -> NodeId {
        self.alloc(ExprNode::in_(probe, collection))
    }
    /// Allocate a [`crate::expr::ExprOp::Exists`] node.
    pub fn alloc_exists(&mut self, sub: NodeId) -> NodeId {
        self.alloc(ExprNode::exists(sub))
    }
    /// Allocate a [`crate::expr::ExprOp::Between`] node.
    pub fn alloc_between(&mut self, expr: NodeId, lo: NodeId, hi: NodeId) -> NodeId {
        self.alloc(ExprNode::between(expr, lo, hi))
    }
    /// Allocate a [`crate::expr::ExprOp::Query`] node.
    pub fn alloc_query_ref(&mut self, id: QueryId) -> NodeId {
        self.alloc(ExprNode::query(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Insert`] node.
    pub fn alloc_insert_ref(&mut self, id: InsertId) -> NodeId {
        self.alloc(ExprNode::insert(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Update`] node.
    pub fn alloc_update_ref(&mut self, id: UpdateId) -> NodeId {
        self.alloc(ExprNode::update(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Delete`] node.
    pub fn alloc_delete_ref(&mut self, id: DeleteId) -> NodeId {
        self.alloc(ExprNode::delete(id))
    }
    /// Allocate a [`crate::expr::ExprOp::Upsert`] node.
    pub fn alloc_upsert_ref(&mut self, id: UpsertId) -> NodeId {
        self.alloc(ExprNode::upsert(id))
    }

    /// Retrieve an [`ExprNode`] by its [`NodeId`].
    ///
    /// **Design-time error contract:** the [`NodeId`] must originate from
    /// this same arena. Out-of-range ids reflect a programmer mistake (a
    /// stale id from another arena, or a freed id), not a runtime input,
    /// and panic with a `#[track_caller]` location for fast debugging.
    /// All sibling `get_*` accessors follow the same contract.
    #[track_caller]
    pub fn get(&self, id: NodeId) -> &ExprNode {
        get_in(&self.nodes, id)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Allocated capacity (not length) of the hot `ExprNode` pool.
    ///
    /// Exposed for `crate::stats::snapshot` so it can split arena bytes
    /// into "hot pool" vs "side pools" without recomputing per-pool sizes.
    pub fn nodes_capacity(&self) -> usize {
        self.nodes.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Attach `span` to `id`, returning the positional [`SpanId`].
    ///
    /// Use [`SpanTable::get_for`] (via [`ExprArena::span_table`]) to recover
    /// the span belonging to a given [`NodeId`]. Multiple spans may be
    /// attached to the same node (e.g. on rewrite); the latest attachment
    /// wins for `get_for` lookups.
    pub fn attach_span(&mut self, id: NodeId, span: Span) -> SpanId {
        self.span_table.push(id, span)
    }

    /// Borrow the [`SpanTable`] for `get`/`get_for` lookups.
    pub fn span_table(&self) -> &SpanTable {
        &self.span_table
    }

    // ── Literal pool ─────────────────────────────────────────────────────────

    pub fn alloc_lit(&mut self, lit: Literal<'static>) -> LiteralId {
        alloc_in(&mut self.lits, lit)
    }
    #[track_caller]
    pub fn get_lit(&self, id: LiteralId) -> &Literal<'static> {
        get_in(&self.lits, id)
    }

    // ── FuncNode pool ─────────────────────────────────────────────────────────

    pub fn alloc_func(&mut self, func: FuncNode) -> FuncId {
        alloc_in(&mut self.funcs, func)
    }
    #[track_caller]
    pub fn get_func(&self, id: FuncId) -> &FuncNode {
        get_in(&self.funcs, id)
    }

    // ── CompositeNode pool ────────────────────────────────────────────────────

    pub fn alloc_composite(&mut self, comp: CompositeNode) -> CompositeId {
        alloc_in(&mut self.composites, comp)
    }
    #[track_caller]
    pub fn get_composite(&self, id: CompositeId) -> &CompositeNode {
        get_in(&self.composites, id)
    }

    // ── WindowNode pool ───────────────────────────────────────────────────────

    pub fn alloc_window(&mut self, win: WindowNode) -> WindowId {
        alloc_in(&mut self.windows, win)
    }
    #[track_caller]
    pub fn get_window(&self, id: WindowId) -> &WindowNode {
        get_in(&self.windows, id)
    }

    // ── CaseNode pool ─────────────────────────────────────────────────────────

    pub fn alloc_case(&mut self, case: CaseNode) -> CaseId {
        alloc_in(&mut self.cases, case)
    }
    #[track_caller]
    pub fn get_case(&self, id: CaseId) -> &CaseNode {
        get_in(&self.cases, id)
    }

    // ── (Removed: `InListNode` pool — collapsed into `composites` above.) ─────

    // ── QueryNode pool ────────────────────────────────────────────────────────

    pub fn alloc_query(&mut self, query: QueryNode) -> QueryId {
        alloc_in(&mut self.queries, query)
    }
    #[track_caller]
    pub fn get_query(&self, id: QueryId) -> &QueryNode {
        get_in(&self.queries, id)
    }

    // ── InsertNode pool ───────────────────────────────────────────────────────

    pub fn alloc_insert(&mut self, node: InsertNode) -> InsertId {
        alloc_in(&mut self.inserts, node)
    }
    #[track_caller]
    pub fn get_insert(&self, id: InsertId) -> &InsertNode {
        get_in(&self.inserts, id)
    }

    // ── UpdateNode pool ───────────────────────────────────────────────────────

    pub fn alloc_update(&mut self, node: UpdateNode) -> UpdateId {
        alloc_in(&mut self.updates, node)
    }
    #[track_caller]
    pub fn get_update(&self, id: UpdateId) -> &UpdateNode {
        get_in(&self.updates, id)
    }

    // ── DeleteNode pool ───────────────────────────────────────────────────────

    pub fn alloc_delete(&mut self, node: DeleteNode) -> DeleteId {
        alloc_in(&mut self.deletes, node)
    }
    #[track_caller]
    pub fn get_delete(&self, id: DeleteId) -> &DeleteNode {
        get_in(&self.deletes, id)
    }

    // ── UpsertNode pool ───────────────────────────────────────────────────────

    pub fn alloc_upsert(&mut self, node: UpsertNode) -> UpsertId {
        alloc_in(&mut self.upserts, node)
    }
    #[track_caller]
    pub fn get_upsert(&self, id: UpsertId) -> &UpsertNode {
        get_in(&self.upserts, id)
    }

    // ── FieldNode pool ────────────────────────────────────────────────────────

    pub fn alloc_field(&mut self, field: FieldNode) -> FieldId {
        alloc_in(&mut self.fields, field)
    }
    #[track_caller]
    pub fn get_field(&self, id: FieldId) -> &FieldNode {
        get_in(&self.fields, id)
    }

    pub fn nodes_slice(&self) -> &[ExprNode] {
        &self.nodes
    }
    pub fn lits_slice(&self) -> &[Literal<'static>] {
        &self.lits
    }
    pub fn funcs_slice(&self) -> &[FuncNode] {
        &self.funcs
    }
    pub fn composites_slice(&self) -> &[CompositeNode] {
        &self.composites
    }
    pub fn windows_slice(&self) -> &[WindowNode] {
        &self.windows
    }
    pub fn cases_slice(&self) -> &[CaseNode] {
        &self.cases
    }
    pub fn queries_slice(&self) -> &[QueryNode] {
        &self.queries
    }
    pub fn inserts_slice(&self) -> &[InsertNode] {
        &self.inserts
    }
    pub fn updates_slice(&self) -> &[UpdateNode] {
        &self.updates
    }
    pub fn deletes_slice(&self) -> &[DeleteNode] {
        &self.deletes
    }
    pub fn upserts_slice(&self) -> &[UpsertNode] {
        &self.upserts
    }
    pub fn fields_slice(&self) -> &[FieldNode] {
        &self.fields
    }
    pub fn span_table_ref(&self) -> &SpanTable {
        &self.span_table
    }
}

#[cfg(test)]
mod intern_tests {
    use super::*;
    use crate::expr::BinOp;

    #[test]
    fn intern_node_collapses_structurally_equal_nodes() {
        let mut arena = ExprArena::new();
        // Build `lit(1)` twice and confirm `intern_node` returns the
        // same id.
        let lit = arena.alloc_lit(Literal::Int32(1));
        let lit_node1 = arena.intern_node(ExprNode::lit(lit));
        let lit_node2 = arena.intern_node(ExprNode::lit(lit));
        assert_eq!(lit_node1, lit_node2, "literal-ref nodes dedup");

        // Distinct shape ⇒ distinct id.
        let lit2 = arena.alloc_lit(Literal::Int32(2));
        let lit_node3 = arena.intern_node(ExprNode::lit(lit2));
        assert_ne!(lit_node1, lit_node3);
    }

    #[test]
    fn alloc_then_intern_finds_alloc_node() {
        let mut arena = ExprArena::new();
        let lit = arena.alloc_lit(Literal::Int32(7));
        // First allocation goes via the bulk path.
        let alloc_id = arena.alloc(ExprNode::lit(lit));
        // The dedup index isn't built yet; calling `intern_node` must
        // seed the index from the existing pool and return the
        // already-allocated id rather than appending a new one.
        let intern_id = arena.intern_node(ExprNode::lit(lit));
        assert_eq!(alloc_id, intern_id);
        assert_eq!(arena.len(), 1, "no duplicate node was appended");
    }

    #[test]
    fn intern_then_alloc_keeps_index_in_sync() {
        let mut arena = ExprArena::new();
        let lit_a = arena.alloc_lit(Literal::Int32(1));
        let lit_b = arena.alloc_lit(Literal::Int32(2));

        // Force index init via intern.
        let _seed = arena.intern_node(ExprNode::lit(lit_a));
        // Now `alloc` should still update the index.
        let alloc_b = arena.alloc(ExprNode::lit(lit_b));
        // intern of the same shape should return the alloc'd id.
        let intern_b = arena.intern_node(ExprNode::lit(lit_b));
        assert_eq!(alloc_b, intern_b);
    }

    #[test]
    fn intern_node_dedups_bin_subtree() {
        let mut arena = ExprArena::new();
        // Two structurally-identical `a + 1` subtrees should share
        // node ids when built through `intern_node`.
        let lit1 = arena.alloc_lit(Literal::Int32(1));
        let field_id = arena.alloc_field(FieldNode {
            namespace: None,
            name: StrId::from_u32(1).unwrap(),
            steps: SmallVec::new(),
        });
        let a_node = arena.intern_node(ExprNode::field(field_id));
        let lit_node = arena.intern_node(ExprNode::lit(lit1));
        let bin1 = arena.intern_node(ExprNode::bin(BinOp::Add, a_node, lit_node));

        let a_node2 = arena.intern_node(ExprNode::field(field_id));
        let lit_node2 = arena.intern_node(ExprNode::lit(lit1));
        let bin2 = arena.intern_node(ExprNode::bin(BinOp::Add, a_node2, lit_node2));

        assert_eq!(a_node, a_node2);
        assert_eq!(lit_node, lit_node2);
        assert_eq!(bin1, bin2, "structurally-equal `a + 1` collapses");
    }
}

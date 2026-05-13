//! Raw [`ExprArena`] — a `DynPool<ExprNode>`-backed flat node store
//! with an optional structural-dedup index.
//!
//! Per `dol-rewrite-plan-v2.md` §8.1 + §8.4 (dedup half).
//!
//! ## Two construction modes
//!
//! - [`ExprArena::push`] — raw, append-only. Always allocates a fresh
//!   slot, returns a fresh [`NodeId`]. Use when callers control
//!   uniqueness themselves (e.g. when each node already references
//!   distinct child ids).
//! - [`ExprArena::intern_node`] — checks the dedup index first.
//!   Returns an existing [`NodeId`] when the byte-identical node is
//!   already present, otherwise pushes and records the new id. The
//!   index is keyed by [`dol_core::hash::fast64`] over the 16 raw
//!   bytes of [`ExprNode`]; collisions are handled by comparing the
//!   actual node bytes after a hash hit.
//!
//! The two modes interoperate: nodes inserted via `push` are never
//! recorded in the dedup index (so a later `intern_node` of the same
//! value will allocate a new slot), but they remain reachable by id
//! and may be referenced by interned children. Mixing modes is
//! intentionally unsurprising — the dedup index is a pure cache that
//! never causes existing ids to disappear.

#[cfg(feature = "std")]
use dol_core::hash::fast64;

#[cfg(feature = "std")]
use dol_cas::handle::NodeId;
#[cfg(feature = "std")]
use dol_cas::pool::{ArenaStorage, DynPool};

#[cfg(feature = "std")]
use hashbrown::HashMap;

#[cfg(feature = "std")]
use super::funcs::FuncRegistry;
#[cfg(feature = "std")]
use super::literals::LiteralPool;
#[cfg(feature = "std")]
use super::node::ExprNode;
#[cfg(feature = "std")]
use super::paths::PathPool;
#[cfg(feature = "std")]
use super::slab::OperandSlab;

/// Flat arena of [`ExprNode`]s addressed by [`NodeId`], plus the four
/// **side pools** that carry payloads too large to inline in the
/// 16-byte node record.
///
/// Backed by [`DynPool<ExprNode>`](dol_cas::pool::DynPool); `Send +
/// Sync` since `ExprNode` is `Pod`. The arena is **monotonically
/// growing** — once a node is pushed its [`NodeId`] is stable for
/// the lifetime of the arena.
///
/// ## Embedded side pools (M3c-δ₂b prereq #5)
///
/// The recursive `lower(Expr<'a>)` pass (plan §8.4) needs handles
/// into four side pools whenever it lowers a non-trivial variant:
///
/// - [`literals`](Self::literals): `Expr::Lit` → `ExprNode::lit_ref(LiteralId)`
///   (plan §8.1 line 1095).
/// - [`funcs`](Self::funcs): `Expr::Call` → `ExprNode::func_ref(FuncId)`
///   (plan §8.1 line 1097).
/// - [`operands`](Self::operands): `Expr::Seq` / `Map` / `Call` /
///   `Match` argument lists park here as
///   [`OperandSpan`](super::slab::OperandSpan) `(offset, len)` pairs.
/// - [`paths`](Self::paths): `Expr::Ref(Path<Name>)` lowers to a
///   [`PathId`](dol_cas::handle::PathId) into the path pool, since the
///   variable-length path cannot be inlined into the 16-byte node.
///
/// Embedding the four pools means `lower` can keep the documented
/// signature
///
/// ```text
/// fn lower(&Expr<'_>, &mut ExprArena, &StringPool, &mut Budget)
///     -> Result<NodeId, LowerError>
/// ```
///
/// without growing into a five- or six-argument function (plan §8.4
/// line 1376). Each pool is exposed through a `&self` and `&mut self`
/// accessor so callers may pre-populate or inspect them outside the
/// `lower` pipeline as well.
#[cfg(feature = "std")]
#[derive(Debug, Default, Clone)]
pub struct ExprArena {
    pool: DynPool<ExprNode>,
    /// Lazy structural-dedup index: `fast64(bytes(node)) -> NodeId`.
    ///
    /// Built on first `intern_node` call; never populated by `push`.
    /// `fast64` collisions are resolved by comparing the raw 16-byte
    /// `ExprNode` slabs of the candidate and the existing entry.
    /// On a true collision the candidate wins a fresh slot — the
    /// index simply forgets the older entry. (Both ids remain valid;
    /// future `intern_node` of the older value will then re-allocate.
    /// `fast64` collisions on identical 16-byte inputs are vanishingly
    /// rare; this strategy is the simple, panic-free choice.)
    dedup: HashMap<u64, NodeId>,
    /// Carrier for `Expr::Lit` payloads (push-only in this slice).
    literals: LiteralPool,
    /// Carrier for `Expr::Call` function definitions (name-keyed
    /// dedup; first-registration-wins on conflicts).
    funcs: FuncRegistry,
    /// Carrier for variadic operand sequences (`Expr::Seq` /
    /// `Expr::Map` / `Expr::Call::args` / `Expr::Match::arms`).
    operands: OperandSlab,
    /// Carrier for lowered `Expr::Ref` paths
    /// (`Path<StrId>` keyed by [`PathId`](dol_cas::handle::PathId)).
    paths: PathPool,
}

#[cfg(feature = "std")]
impl ExprArena {
    /// Construct an empty arena (and four empty side pools).
    #[must_use]
    pub fn new() -> Self {
        Self {
            pool: DynPool::new(),
            dedup: HashMap::new(),
            literals: LiteralPool::new(),
            funcs: FuncRegistry::new(),
            operands: OperandSlab::new(),
            paths: PathPool::new(),
        }
    }

    /// Pre-allocate capacity for `cap` nodes (and the same capacity
    /// in the dedup index — the upper bound on distinct entries).
    ///
    /// Side pools start empty; reserve their capacity on demand via
    /// the dedicated `*_mut()` accessors when known up front.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pool: DynPool::with_capacity(cap),
            dedup: HashMap::with_capacity(cap),
            literals: LiteralPool::new(),
            funcs: FuncRegistry::new(),
            operands: OperandSlab::new(),
            paths: PathPool::new(),
        }
    }

    // ─── Side-pool accessors ────────────────────────────────────────

    /// Borrow the literal carrier (`Expr::Lit` payloads).
    #[must_use]
    #[inline]
    pub fn literals(&self) -> &LiteralPool {
        &self.literals
    }

    /// Mutably borrow the literal carrier — used by `lower` to intern
    /// `Expr::Lit` and by tests/builders to seed payloads.
    #[inline]
    pub fn literals_mut(&mut self) -> &mut LiteralPool {
        &mut self.literals
    }

    /// Borrow the function registry (`Expr::Call` definitions).
    #[must_use]
    #[inline]
    pub fn funcs(&self) -> &FuncRegistry {
        &self.funcs
    }

    /// Mutably borrow the function registry.
    #[inline]
    pub fn funcs_mut(&mut self) -> &mut FuncRegistry {
        &mut self.funcs
    }

    /// Borrow the variadic operand slab.
    #[must_use]
    #[inline]
    pub fn operands(&self) -> &OperandSlab {
        &self.operands
    }

    /// Mutably borrow the variadic operand slab.
    #[inline]
    pub fn operands_mut(&mut self) -> &mut OperandSlab {
        &mut self.operands
    }

    /// Borrow the path pool (`Expr::Ref` payloads).
    #[must_use]
    #[inline]
    pub fn paths(&self) -> &PathPool {
        &self.paths
    }

    /// Mutably borrow the path pool.
    #[inline]
    pub fn paths_mut(&mut self) -> &mut PathPool {
        &mut self.paths
    }

    /// Push a node and return its [`NodeId`].
    ///
    /// Returns `None` only when the arena would exceed
    /// [`u32::MAX`] nodes (the [`NodeId`] niche limit).
    ///
    /// The dedup index is **not** consulted or updated. Use
    /// [`Self::intern_node`] for structural sharing.
    pub fn push(&mut self, node: ExprNode) -> Option<NodeId> {
        self.pool.push_id(node)
    }

    /// Intern a node — return its existing id if a byte-identical
    /// node is already in the arena, otherwise push and record it.
    ///
    /// Returns `None` only when the arena would exceed [`u32::MAX`]
    /// nodes (the [`NodeId`] niche limit).
    pub fn intern_node(&mut self, node: ExprNode) -> Option<NodeId> {
        let bytes: [u8; 16] = bytemuck::cast(node);
        let key = fast64(&bytes);
        if let Some(&existing) = self.dedup.get(&key) {
            // Confirm byte equality — `fast64` is not collision-free.
            if let Some(prev) = self.pool.get_by_id(existing) {
                if prev == &node {
                    return Some(existing);
                }
            }
            // Collision (or stale entry): fall through and allocate
            // a fresh slot, then overwrite the index entry below.
        }
        let id = self.pool.push_id(node)?;
        self.dedup.insert(key, id);
        Some(id)
    }

    /// Borrow a node by id.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&ExprNode> {
        self.pool.get_by_id(id)
    }

    /// Number of nodes in the arena.
    ///
    /// This is the total number of slots — interned and `push`ed
    /// alike — not the number of distinct dedup entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// `true` when the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }

    /// Number of entries currently tracked in the dedup index.
    /// Always `<= len()`. Test/diagnostic helper.
    #[must_use]
    pub fn dedup_len(&self) -> usize {
        self.dedup.len()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::expr::node::ExprNode;
    use crate::expr::ops::{BinOp, UnaryOp};
    use dol_cas::handle::Lid;

    fn nid(i: u32) -> NodeId {
        Lid::from_u32(i).unwrap()
    }

    #[test]
    fn empty_arena_round_trip() {
        let a = ExprArena::new();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
        assert_eq!(a.dedup_len(), 0);
    }

    #[test]
    fn push_and_get() {
        let mut a = ExprArena::new();
        let n0 = ExprNode::param(0);
        let n1 = ExprNode::param(1);
        let id0 = a.push(n0).unwrap();
        let id1 = a.push(n1).unwrap();
        assert_ne!(id0, id1);
        assert_eq!(a.len(), 2);
        assert_eq!(a.get(id0), Some(&n0));
        assert_eq!(a.get(id1), Some(&n1));
    }

    #[test]
    fn ids_are_dense_and_one_based() {
        let mut a = ExprArena::new();
        let id0 = a.push(ExprNode::wildcard()).unwrap();
        let id1 = a.push(ExprNode::count_all()).unwrap();
        assert_eq!(id0.index(), 0);
        assert_eq!(id1.index(), 1);
        assert_eq!(id0.get(), 1);
        assert_eq!(id1.get(), 2);
    }

    #[test]
    fn nested_node_references() {
        // Build (a + b) where a, b are wildcard sentinels — the arena
        // doesn't enforce semantic correctness, only structural.
        let mut arena = ExprArena::new();
        let left = arena.push(ExprNode::wildcard()).unwrap();
        let right = arena.push(ExprNode::wildcard()).unwrap();
        let sum = arena.push(ExprNode::bin(BinOp::Add, left, right)).unwrap();
        let (op, l, r) = arena.get(sum).unwrap().as_bin().unwrap();
        assert_eq!(op, BinOp::Add);
        assert_eq!(l, left);
        assert_eq!(r, right);
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let a = ExprArena::new();
        assert!(a.get(nid(1)).is_none());
    }

    // ─── intern_node / dedup ─────────────────────────────────────────

    #[test]
    fn intern_returns_same_id_for_byte_identical_node() {
        let mut a = ExprArena::new();
        let n = ExprNode::param(7);
        let id1 = a.intern_node(n).unwrap();
        let id2 = a.intern_node(n).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(a.len(), 1);
        assert_eq!(a.dedup_len(), 1);
    }

    #[test]
    fn intern_distinct_nodes_get_distinct_ids() {
        let mut a = ExprArena::new();
        let id1 = a.intern_node(ExprNode::param(1)).unwrap();
        let id2 = a.intern_node(ExprNode::param(2)).unwrap();
        let id3 = a.intern_node(ExprNode::wildcard()).unwrap();
        let id4 = a.intern_node(ExprNode::count_all()).unwrap();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id3, id4);
        assert_eq!(a.len(), 4);
        assert_eq!(a.dedup_len(), 4);
    }

    #[test]
    fn intern_dedups_structural_subtrees() {
        // (x + y) where both children are interned wildcards — the
        // shared `wildcard()` constant collapses to a single id.
        let mut a = ExprArena::new();
        let l1 = a.intern_node(ExprNode::wildcard()).unwrap();
        let l2 = a.intern_node(ExprNode::wildcard()).unwrap();
        assert_eq!(l1, l2);
        let sum = a.intern_node(ExprNode::bin(BinOp::Add, l1, l2)).unwrap();
        // Re-intern the same binary should find the same id.
        let sum2 = a.intern_node(ExprNode::bin(BinOp::Add, l1, l2)).unwrap();
        assert_eq!(sum, sum2);
        // Different operator → different id, even with same operands.
        let prod = a.intern_node(ExprNode::bin(BinOp::Mul, l1, l2)).unwrap();
        assert_ne!(sum, prod);
        // Total: 1 wildcard + 1 add + 1 mul.
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn flag_difference_makes_distinct_dedup_keys() {
        use crate::expr::flags::NodeFlags;
        let mut a = ExprArena::new();
        let mut n0 = ExprNode::unary(UnaryOp::Not, nid(1));
        let mut n1 = ExprNode::unary(UnaryOp::Not, nid(1));
        n1.set_flags(NodeFlags::new().with_negated(true));
        let id0 = a.intern_node(n0).unwrap();
        let id1 = a.intern_node(n1).unwrap();
        assert_ne!(id0, id1);
        // And re-interning n0 still finds the original.
        let id0_again = a.intern_node(n0).unwrap();
        assert_eq!(id0, id0_again);
        // Touch the binding to keep clippy happy on no-op mutation.
        n0.set_flags(n0.flags());
    }

    #[test]
    fn push_does_not_populate_dedup_index() {
        let mut a = ExprArena::new();
        let n = ExprNode::param(5);
        let pushed = a.push(n).unwrap();
        // Now intern the same value: dedup is cold so we get a *new*
        // slot. This is the documented mode-mixing behaviour.
        let interned = a.intern_node(n).unwrap();
        assert_ne!(pushed, interned);
        assert_eq!(a.len(), 2);
        assert_eq!(a.dedup_len(), 1);
        // But re-interning hits the cache.
        let again = a.intern_node(n).unwrap();
        assert_eq!(again, interned);
    }

    // ─── Embedded side pools (M3c-δ₂b prereq #5) ─────────────────────

    #[test]
    fn fresh_arena_has_empty_side_pools() {
        let a = ExprArena::new();
        assert!(a.literals().is_empty());
        assert!(a.funcs().is_empty());
        assert!(a.operands().is_empty());
        assert!(a.paths().is_empty());
    }

    #[test]
    fn with_capacity_does_not_pre_populate_side_pools() {
        let a = ExprArena::with_capacity(64);
        assert!(a.literals().is_empty());
        assert!(a.funcs().is_empty());
        assert!(a.operands().is_empty());
        assert!(a.paths().is_empty());
    }

    #[test]
    fn literal_pool_mutation_visible_through_accessors() {
        use dol_core::literal::Literal;
        let mut a = ExprArena::new();
        let id = a.literals_mut().intern(&Literal::Int64(42)).unwrap();
        // Read-only accessor sees the same id and payload.
        assert_eq!(a.literals().len(), 1);
        assert_eq!(a.literals().get(id), Some(&Literal::Int64(42)));
    }

    #[test]
    fn func_registry_mutation_visible_through_accessors() {
        use crate::expr::meta::{Arity, FuncDef, FuncKind};
        let mut a = ExprArena::new();
        let def = FuncDef::new_static("LENGTH", Arity::Exact(1), FuncKind::Scalar);
        let id = a.funcs_mut().intern(&def).unwrap();
        assert_eq!(a.funcs().len(), 1);
        assert_eq!(a.funcs().get(id), Some(&def));
        // Re-intern by name dedups (FuncRegistry contract).
        let id2 = a.funcs_mut().intern(&def).unwrap();
        assert_eq!(id, id2);
        assert_eq!(a.funcs().len(), 1);
    }

    #[test]
    fn operand_slab_mutation_visible_through_accessors() {
        let mut a = ExprArena::new();
        let span = a.operands_mut().push_span(&[1, 2, 3]).unwrap();
        assert_eq!(a.operands().get(span), Some(&[1u32, 2, 3][..]));
    }

    #[test]
    fn path_pool_mutation_visible_through_accessors() {
        use dol_cas::string_pool::StringPool;
        use dol_core::budget::Budget;
        use dol_core::config::BudgetConfig;
        use dol_core::path::Path;
        use dol_core::strings::Name;

        let mut a = ExprArena::new();
        let strings = StringPool::standard();
        let mut budget = Budget::from_config(&BudgetConfig::standard());
        let lowered =
            crate::expr::lower::lower_path(&Path::<Name>::new("users"), &strings, &mut budget)
                .unwrap();

        let id = a.paths_mut().intern(&lowered).unwrap();
        assert_eq!(a.paths().len(), 1);
        assert_eq!(a.paths().get(id), Some(&lowered));
        // PathPool is structurally deduped — same path → same id.
        let id2 = a.paths_mut().intern(&lowered).unwrap();
        assert_eq!(id, id2);
        assert_eq!(a.paths().len(), 1);
    }

    #[test]
    fn clone_deep_copies_side_pools() {
        use dol_core::literal::Literal;
        let mut a = ExprArena::new();
        let lit_id = a.literals_mut().intern(&Literal::Bool(true)).unwrap();
        let span = a.operands_mut().push_span(&[7, 8]).unwrap();

        let b = a.clone();
        assert_eq!(b.literals().len(), 1);
        assert_eq!(b.literals().get(lit_id), Some(&Literal::Bool(true)));
        assert_eq!(b.operands().get(span), Some(&[7u32, 8][..]));

        // Mutating the clone must not affect the original.
        let mut b = b;
        let _extra = b.literals_mut().intern(&Literal::Bool(false)).unwrap();
        assert_eq!(a.literals().len(), 1);
        assert_eq!(b.literals().len(), 2);
    }

    #[test]
    fn default_constructor_matches_new() {
        let a = ExprArena::new();
        let b = ExprArena::default();
        assert_eq!(a.len(), b.len());
        assert!(a.literals().is_empty() && b.literals().is_empty());
        assert!(a.funcs().is_empty() && b.funcs().is_empty());
        assert!(a.operands().is_empty() && b.operands().is_empty());
        assert!(a.paths().is_empty() && b.paths().is_empty());
    }
}

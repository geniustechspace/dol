//! Lowering — `Expr<'a>` → `ExprArena` and `Path<Name>` → `Path<StrId>`.
//!
//! Per `dol-rewrite-plan-v2.md` §8.4. The lowering pass is the
//! single, authorised bridge between the user-facing tree DSL and the
//! flat arena form that backends consume. It does three things:
//!
//! 1. Converts the recursive [`Expr<'a>`](super::tree::Expr) tree into
//!    the flat [`ExprArena`](super::arena::ExprArena) representation.
//! 2. Converts [`Path<Name>`] segments to [`Path<StrId>`] by interning
//!    names into a [`StringPool`] (this file).
//! 3. Threads `&mut Budget` at every recursive descent to bound work.
//!
//! ## Status — M3c-δ₂a
//!
//! - [`lower_path`] (this slice) is the only authorised site that
//!   converts `Path<Name>` into `Path<StrId>`. No other code path in
//!   the workspace should perform that conversion — backends and the
//!   `lower(Expr)` pass both call into it.
//! - [`LowerError`] is the final error shape from plan §8.4 line 1396;
//!   the recursive `lower(Expr<'a>) → ExprArena` will reuse it with no
//!   variant changes.
//!
//! The recursive [`lower`] pass is deferred to M3c-δ₂b because it
//! needs a variadic operand slab (for `Expr::Call`, `Expr::Seq`,
//! `Expr::Map`, `Expr::Match`, `Expr::Scoped`, …) plus
//! `LiteralPool` / `FuncRegistry` infrastructure that has no carrier
//! in M3a–c yet. The plan places that work in M3c-δ₂b alongside the
//! `compute_hash` integration in §8.5.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use dol_cas::handle::{NodeId, StrId};
use dol_cas::string_pool::{InternError, StringPool};
use dol_core::budget::{Budget, BudgetExceeded};
use dol_core::literal::Literal;
use dol_core::path::Path;
use dol_core::strings::Name;

use crate::expr::arena::ExprArena;
use crate::expr::contexts::{ContextPoolError, LoweredContext, LoweredOrderByExpr};
use crate::expr::funcs::FuncRegistryError;
use crate::expr::literals::LiteralPoolError;
use crate::expr::meta::OpDef;
use crate::expr::node::ExprNode;
use crate::expr::ops::BinOp;
use crate::expr::paths::PathPoolError;
use crate::expr::slab::OperandSlabError;
use crate::expr::tree::Expr;
use crate::expr::types::TypePoolError;

/// First-violation enum returned by every lowering routine. Variants
/// surface the underlying primitive failure (budget, intern, arena)
/// rather than wrapping it behind a generic "lower failed" code, so
/// callers can route each class to the right diagnostic.
///
/// Final shape per `dol-rewrite-plan-v2.md` §8.4 line 1396; the
/// recursive `lower(Expr)` pass extends it with the side-pool
/// capacity variants and the unknown-binary-operator variant the tree
/// → arena translation may surface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LowerError {
    /// A [`Budget`] cap was exhausted mid-descent. Wraps the original
    /// [`BudgetExceeded`] so the caller learns *which* cap (depth /
    /// nodes / bytes) was the first to break.
    BudgetExceeded(BudgetExceeded),
    /// Interning a string segment into the [`StringPool`] failed.
    /// Carries the underlying [`InternError`] (hash collision /
    /// capacity exceeded / pool poisoned).
    InternFailed(InternError),
    /// The [`ExprArena`] refused to allocate a node because its slot
    /// table would overflow `u32::MAX`.
    ArenaOverflow,
    /// One of the `ExprArena` side pools (`literals` / `funcs` /
    /// `operands` / `paths` / `types` / `contexts`) refused to
    /// allocate because its slot table would overflow `u32::MAX`.
    /// All side-pool errors share the same shape (`CapacityExceeded`)
    /// and collapse into this variant.
    SidePoolOverflow,
    /// A `Binary { op: OpDef, … }` carried an [`OpDef`] whose name
    /// does not appear in [`OpDef::well_known`] — there is no
    /// wire-side [`BinOp`] to lower it to. Carries the offending name
    /// so callers can surface a precise diagnostic.
    UnknownBinOp(String),
}

impl core::fmt::Display for LowerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BudgetExceeded(inner) => write!(f, "lower: {inner}"),
            Self::InternFailed(inner) => write!(f, "lower: {inner}"),
            Self::ArenaOverflow => f.write_str("lower: arena overflow"),
            Self::SidePoolOverflow => f.write_str("lower: side pool overflow"),
            Self::UnknownBinOp(name) => write!(f, "lower: unknown binary operator '{name}'"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LowerError {}

impl From<BudgetExceeded> for LowerError {
    #[inline]
    fn from(err: BudgetExceeded) -> Self {
        Self::BudgetExceeded(err)
    }
}

impl From<InternError> for LowerError {
    #[inline]
    fn from(err: InternError) -> Self {
        Self::InternFailed(err)
    }
}

impl From<LiteralPoolError> for LowerError {
    #[inline]
    fn from(_: LiteralPoolError) -> Self {
        Self::SidePoolOverflow
    }
}

impl From<FuncRegistryError> for LowerError {
    #[inline]
    fn from(_: FuncRegistryError) -> Self {
        Self::SidePoolOverflow
    }
}

impl From<OperandSlabError> for LowerError {
    #[inline]
    fn from(_: OperandSlabError) -> Self {
        Self::SidePoolOverflow
    }
}

impl From<PathPoolError> for LowerError {
    #[inline]
    fn from(_: PathPoolError) -> Self {
        Self::SidePoolOverflow
    }
}

impl From<TypePoolError> for LowerError {
    #[inline]
    fn from(_: TypePoolError) -> Self {
        Self::SidePoolOverflow
    }
}

impl From<ContextPoolError> for LowerError {
    #[inline]
    fn from(_: ContextPoolError) -> Self {
        Self::SidePoolOverflow
    }
}

/// Lower a tree-DSL [`Expr<'a>`] into the flat [`ExprArena`],
/// returning the [`NodeId`] of the root.
///
/// This is the **single, authorised** lowering entry — the recursive
/// counterpart to [`lower_path`]. Per `dol-rewrite-plan-v2.md` §8.4
/// line 1376 it threads:
///
/// - `arena`: receives every freshly allocated [`ExprNode`] plus
///   payloads in the side pools (`literals` / `funcs` / `operands` /
///   `paths` / `types` / `contexts`).
/// - `strings`: the [`StringPool`] used to intern every textual
///   segment (path segments, map keys, function names, label names).
/// - `budget`: charged one [`Budget::node`] per allocated arena node
///   plus the per-segment charges levied by [`lower_path`]. Depth is
///   charged once per recursive descent into a non-leaf variant.
///
/// The arena nodes are allocated through
/// [`ExprArena::intern_node`] so structurally identical subtrees
/// collapse to a single [`NodeId`] — the cornerstone of the
/// downstream `compute_hash` walker (§8.5).
///
/// # Errors
///
/// Returns the first failure encountered:
///
/// - [`LowerError::BudgetExceeded`] — depth / nodes / bytes cap hit.
/// - [`LowerError::InternFailed`] — a string segment could not be
///   interned into `strings`.
/// - [`LowerError::ArenaOverflow`] — the arena's node table is full.
/// - [`LowerError::SidePoolOverflow`] — one of the side pools
///   refused a new entry.
/// - [`LowerError::UnknownBinOp`] — an [`OpDef`] used a name that
///   does not appear in [`OpDef::well_known`].
pub fn lower(
    expr: &Expr<'_>,
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    // Charge one depth per descent — leaves still go through this
    // function so the cap is uniformly enforced.
    budget.depth()?;
    let id = match expr {
        // ── Leaves ────────────────────────────────────────────────
        Expr::Param => alloc_node(arena, ExprNode::param(0))?,
        Expr::Wildcard => alloc_node(arena, ExprNode::wildcard())?,
        Expr::CountAll => alloc_node(arena, ExprNode::count_all())?,
        Expr::Lit(lit) => lower_lit(lit, arena)?,
        Expr::Ref(path) => lower_ref(path, arena, strings, budget)?,

        // ── Operations ────────────────────────────────────────────
        Expr::Binary { left, op, right } => {
            lower_binary(left, op, right, arena, strings, budget)?
        }
        Expr::Unary { op, expr } => {
            let operand = lower(expr, arena, strings, budget)?;
            alloc_node(arena, ExprNode::unary(*op, operand))?
        }

        // ── Variadic / structural ─────────────────────────────────
        Expr::Seq(elements) => lower_seq(elements, arena, strings, budget)?,
        Expr::Map(entries) => lower_map(entries, arena, strings, budget)?,
        Expr::Call { func, args } => lower_call(func, args, arena, strings, budget)?,
        Expr::Cast { expr, target_type } => {
            let operand = lower(expr, arena, strings, budget)?;
            let type_id = arena.types_mut().intern(target_type)?;
            alloc_node(arena, ExprNode::cast(operand, type_id))?
        }
        Expr::Match { arms, fallback } => {
            lower_match(arms, fallback.as_deref(), arena, strings, budget)?
        }
        Expr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = lower(cond, arena, strings, budget)?;
            let t = lower(then_expr, arena, strings, budget)?;
            let e = lower(else_expr, arena, strings, budget)?;
            alloc_node(arena, ExprNode::if_then_else(c, t, e))?
        }
        Expr::InRange { expr, low, high } => {
            let e = lower(expr, arena, strings, budget)?;
            let lo = lower(low, arena, strings, budget)?;
            let hi = lower(high, arena, strings, budget)?;
            alloc_node(arena, ExprNode::in_range(e, lo, hi))?
        }
        Expr::MemberOf { expr, set } => lower_member_of(expr, set, arena, strings, budget)?,
        Expr::Label { expr, name } => {
            let e = lower(expr, arena, strings, budget)?;
            let str_id = intern_name(name, strings)?;
            alloc_node(arena, ExprNode::label(e, str_id))?
        }
        Expr::Scoped { expr, context } => {
            lower_scoped(expr, context, arena, strings, budget)?
        }
    };
    Ok(id)
}

/// Allocate a node into the arena via the structural-dedup
/// [`ExprArena::intern_node`] gate, translating an arena overflow
/// into [`LowerError::ArenaOverflow`].
#[inline]
fn alloc_node(arena: &mut ExprArena, node: ExprNode) -> Result<NodeId, LowerError> {
    arena
        .intern_node(node)
        .ok_or(LowerError::ArenaOverflow)
}

/// Intern a [`Name`] into the [`StringPool`], returning the resulting
/// [`StrId`].
#[inline]
fn intern_name(name: &Name, strings: &StringPool) -> Result<StrId, LowerError> {
    Ok(strings.intern(name.as_str())?)
}

fn lower_lit(lit: &Literal<'_>, arena: &mut ExprArena) -> Result<NodeId, LowerError> {
    let id = arena.literals_mut().intern(lit)?;
    alloc_node(arena, ExprNode::lit_ref(id))
}

fn lower_ref(
    path: &Path<Name>,
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let lowered = lower_path(path, strings, budget)?;
    let id = arena.paths_mut().intern(&lowered)?;
    alloc_node(arena, ExprNode::path_ref(id))
}

fn lower_binary(
    left: &Expr<'_>,
    op: &OpDef,
    right: &Expr<'_>,
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let bin = bin_op_from_def(op).ok_or_else(|| LowerError::UnknownBinOp(op.name().into()))?;
    let l = lower(left, arena, strings, budget)?;
    let r = lower(right, arena, strings, budget)?;
    alloc_node(arena, ExprNode::bin(bin, l, r))
}

/// Translate a tree-side [`OpDef`] (carrier of a name + category) to
/// the wire-side [`BinOp`] enum that ships inside an
/// [`ExprNode`](super::node::ExprNode). The mapping is name-based and
/// covers exactly the well-known catalogue published by
/// [`OpDef::well_known`].
fn bin_op_from_def(def: &OpDef) -> Option<BinOp> {
    Some(match def.name() {
        OpDef::EQ => BinOp::Eq,
        OpDef::NE => BinOp::Ne,
        OpDef::LT => BinOp::Lt,
        OpDef::GT => BinOp::Gt,
        OpDef::LE => BinOp::Le,
        OpDef::GE => BinOp::Ge,
        OpDef::ADD => BinOp::Add,
        OpDef::SUB => BinOp::Sub,
        OpDef::MUL => BinOp::Mul,
        OpDef::DIV => BinOp::Div,
        OpDef::MOD => BinOp::Rem,
        OpDef::AND => BinOp::And,
        OpDef::OR => BinOp::Or,
        OpDef::BIT_AND => BinOp::BitAnd,
        OpDef::BIT_OR => BinOp::BitOr,
        OpDef::BIT_XOR => BinOp::BitXor,
        OpDef::SHIFT_LEFT => BinOp::Shl,
        OpDef::SHIFT_RIGHT => BinOp::Shr,
        OpDef::CONCAT => BinOp::Concat,
        OpDef::LIKE => BinOp::Like,
        OpDef::ILIKE => BinOp::ILike,
        _ => return None,
    })
}

fn lower_seq(
    elements: &[Expr<'_>],
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let mut slots: Vec<u32> = Vec::with_capacity(elements.len());
    for el in elements {
        let id = lower(el, arena, strings, budget)?;
        slots.push(id.get());
    }
    let span = arena.operands_mut().push_span(&slots)?;
    alloc_node(arena, ExprNode::seq(span))
}

fn lower_map(
    entries: &[(Name, Expr<'_>)],
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    // Two slots per entry: [key_str_id, value_node_id, …]
    let pair_capacity = entries.len().saturating_mul(2);
    let mut slots: Vec<u32> = Vec::with_capacity(pair_capacity);
    for (name, value) in entries {
        let key_id = intern_name(name, strings)?;
        let val_id = lower(value, arena, strings, budget)?;
        slots.push(key_id.get());
        slots.push(val_id.get());
    }
    let span = arena.operands_mut().push_span(&slots)?;
    alloc_node(arena, ExprNode::map(span))
}

fn lower_call(
    func: &crate::expr::meta::FuncDef,
    args: &[Expr<'_>],
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let func_id = arena.funcs_mut().intern(func)?;
    let mut slots: Vec<u32> = Vec::with_capacity(args.len());
    for a in args {
        let id = lower(a, arena, strings, budget)?;
        slots.push(id.get());
    }
    let span = arena.operands_mut().push_span(&slots)?;
    alloc_node(arena, ExprNode::call(func_id, span))
}

fn lower_match(
    arms: &[(Expr<'_>, Expr<'_>)],
    fallback: Option<&Expr<'_>>,
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    // Two slots per arm: [cond, result, …]
    let pair_capacity = arms.len().saturating_mul(2);
    let mut slots: Vec<u32> = Vec::with_capacity(pair_capacity);
    for (cond, result) in arms {
        let c = lower(cond, arena, strings, budget)?;
        let r = lower(result, arena, strings, budget)?;
        slots.push(c.get());
        slots.push(r.get());
    }
    let span = arena.operands_mut().push_span(&slots)?;
    let fallback_id = match fallback {
        Some(e) => Some(lower(e, arena, strings, budget)?),
        None => None,
    };
    alloc_node(arena, ExprNode::match_arms(span, fallback_id))
}

fn lower_member_of(
    expr: &Expr<'_>,
    set: &[Expr<'_>],
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let e = lower(expr, arena, strings, budget)?;
    let mut slots: Vec<u32> = Vec::with_capacity(set.len());
    for s in set {
        let id = lower(s, arena, strings, budget)?;
        slots.push(id.get());
    }
    let span = arena.operands_mut().push_span(&slots)?;
    alloc_node(arena, ExprNode::member_of(e, span))
}

fn lower_scoped(
    inner: &Expr<'_>,
    context: &crate::expr::context::Context<'_>,
    arena: &mut ExprArena,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let inner_id = lower(inner, arena, strings, budget)?;
    let mut partition_by: Vec<NodeId> = Vec::with_capacity(context.partition_by.len());
    for k in &context.partition_by {
        partition_by.push(lower(k, arena, strings, budget)?);
    }
    let mut order_by: Vec<LoweredOrderByExpr> = Vec::with_capacity(context.order_by.len());
    for o in &context.order_by {
        let expr_id = lower(&o.expr, arena, strings, budget)?;
        order_by.push(LoweredOrderByExpr {
            expr: expr_id,
            dir: o.dir,
            nulls: o.nulls,
        });
    }
    let lowered_ctx = LoweredContext {
        partition_by,
        order_by,
        frame: context.frame,
    };
    let ctx_id = arena.contexts_mut().intern(&lowered_ctx)?;
    alloc_node(arena, ExprNode::scoped(inner_id, ctx_id))
}

/// Lower a [`Path<Name>`] to [`Path<StrId>`] by interning each segment
/// into `strings`.
///
/// This is the **single, authorised** site where `Path<Name>` becomes
/// `Path<StrId>` (per plan §8.4 line 1389). No other code in the
/// workspace should perform that conversion: the recursive
/// `lower(Expr)` pass calls into this function for `Expr::Ref`, and
/// backends consume the resulting `Path<StrId>` directly.
///
/// # Budget accounting
///
/// One [`Budget::node`] charge per interned segment (target + optional
/// namespace + each field segment). Paths are flat — there is no
/// recursion — so no [`Budget::depth`] charge is taken.
///
/// # Errors
///
/// - [`LowerError::BudgetExceeded`] if `budget.node()` runs out of
///   headroom before every segment is interned.
/// - [`LowerError::InternFailed`] if the underlying
///   [`StringPool::intern`] reports a hash collision, capacity
///   exceeded, or a poisoned lock.
pub fn lower_path(
    path: &Path<Name>,
    strings: &StringPool,
    budget: &mut Budget,
) -> Result<Path<StrId>, LowerError> {
    // Target first — every path has exactly one target.
    budget.node()?;
    let target_id = strings.intern(path.target_segment().as_str())?;
    let mut out: Path<StrId> = Path::from_segment(target_id);

    // Optional namespace.
    if let Some(ns) = path.namespace_segment() {
        budget.node()?;
        let ns_id = strings.intern(ns.as_str())?;
        out = out.namespace(ns_id);
    }

    // Field chain — preserves order.
    for field in path.field_segments() {
        budget.node()?;
        let field_id = strings.intern(field.as_str())?;
        out = out.get(field_id);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use dol_core::config::BudgetConfig;
    use dol_core::path::StringResolver;

    fn fresh_budget() -> Budget {
        Budget::from_config(&BudgetConfig::standard())
    }

    #[test]
    fn target_only_path_lowers() {
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let p = Path::<Name>::new("users");

        let lowered = lower_path(&p, &pool, &mut budget).unwrap();

        assert!(!lowered.has_namespace());
        assert!(!lowered.has_fields());
        let resolver: &(dyn StringResolver + 'static) = &pool;
        assert_eq!(lowered.resolve_target(resolver), "users");
    }

    #[test]
    fn namespace_and_fields_lower_in_order() {
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let p = Path::<Name>::new("orders")
            .namespace("api")
            .get("customer")
            .get("email");

        let lowered = lower_path(&p, &pool, &mut budget).unwrap();
        let resolver: &(dyn StringResolver + 'static) = &pool;

        assert_eq!(lowered.resolve_namespace(resolver), Some("api"));
        assert_eq!(lowered.resolve_target(resolver), "orders");
        let fields: Vec<&str> = lowered.resolve_fields(resolver).collect();
        assert_eq!(fields, alloc::vec!["customer", "email"]);
    }

    #[test]
    fn duplicate_segments_share_a_strid() {
        // Interning is content-addressed, so the same name appearing
        // twice in a path lowers to the same StrId both times. This
        // is the property `lower(Expr)` will rely on for cheap
        // structural equality of paths inside the arena.
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let p = Path::<Name>::new("a").get("a");

        let lowered = lower_path(&p, &pool, &mut budget).unwrap();
        let target = *lowered.target_segment();
        let field = lowered.field_segments()[0];
        assert_eq!(target, field);
    }

    #[test]
    fn segment_count_matches_node_charge() {
        // Budget accounting: target + namespace + N fields = N+2 nodes.
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let before = budget.nodes;
        let p = Path::<Name>::new("t").namespace("ns").get("f1").get("f2");
        lower_path(&p, &pool, &mut budget).unwrap();
        let after = budget.nodes;
        assert_eq!(before - after, 4);
    }

    #[test]
    fn budget_exhaustion_is_reported_as_lower_error() {
        // A budget that can only afford the target charge errors out
        // when the namespace would push it past zero nodes.
        let pool = StringPool::standard();
        let mut budget = Budget::from_config(&BudgetConfig {
            max_depth: 8,
            max_nodes: 1,
            max_bytes: 1024,
        });
        let p = Path::<Name>::new("t").namespace("ns");

        let err = lower_path(&p, &pool, &mut budget).unwrap_err();
        match err {
            LowerError::BudgetExceeded(BudgetExceeded::Nodes) => {}
            other => panic!("expected BudgetExceeded(Nodes), got {other:?}"),
        }
    }

    #[test]
    fn round_trip_through_string_pool_resolver() {
        // A `Path<Name>` lowered through the pool and resolved back
        // via the pool's `StringResolver` impl yields the original
        // strings. This pins the contract that `lower_path` is the
        // only point at which the segment type changes.
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let p = Path::<Name>::new("orders")
            .namespace("api")
            .get("billing")
            .get("zip");

        let lowered = lower_path(&p, &pool, &mut budget).unwrap();
        let resolver: &(dyn StringResolver + 'static) = &pool;

        assert_eq!(lowered.resolve_namespace(resolver), p.namespace_str());
        assert_eq!(lowered.resolve_target(resolver), p.target_str());
        let lowered_fields: Vec<&str> = lowered.resolve_fields(resolver).collect();
        let original_fields: Vec<&str> = p.field_strs().collect();
        assert_eq!(lowered_fields, original_fields);
    }

    #[test]
    fn lower_error_display_threads_inner_message() {
        let e = LowerError::BudgetExceeded(BudgetExceeded::Nodes);
        let s = alloc::format!("{e}");
        assert!(s.starts_with("lower: "));
        assert!(s.contains("nodes"));
    }

    // ─── Recursive `lower(Expr<'a>) → NodeId` tests ──────────────

    use crate::expr::arena::ExprArena;
    use crate::expr::context::Context;
    use crate::expr::frame::{Boundary, Frame};
    use crate::expr::meta::{Arity, FuncDef, FuncKind, OpCategory};
    use crate::expr::node::ExprNode;
    use crate::expr::ops::{BinOp, OpFamily, UnaryOp};
    use crate::expr::order::{NullsOrder, OrderByExpr, SortDirection};
    use crate::expr::tree::Expr;
    use dol_core::data_type::DataType;
    use dol_core::literal::Literal;

    fn fresh_arena() -> ExprArena {
        ExprArena::new()
    }

    fn family_of(arena: &ExprArena, id: NodeId) -> OpFamily {
        arena.get(id).unwrap().family().unwrap()
    }

    #[test]
    fn lower_param_writes_param_node() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let id = lower(&Expr::Param, &mut arena, &pool, &mut budget).unwrap();
        assert_eq!(family_of(&arena, id), OpFamily::Param);
    }

    #[test]
    fn lower_wildcard_and_count_all() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let w = lower(&Expr::Wildcard, &mut arena, &pool, &mut budget).unwrap();
        let c = lower(&Expr::CountAll, &mut arena, &pool, &mut budget).unwrap();
        assert_eq!(family_of(&arena, w), OpFamily::Wildcard);
        assert_eq!(family_of(&arena, c), OpFamily::CountAll);
        assert_ne!(w, c);
    }

    #[test]
    fn lower_lit_interns_into_literal_pool() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let id = lower(
            &Expr::Lit(Literal::Int64(7)),
            &mut arena,
            &pool,
            &mut budget,
        )
        .unwrap();
        let node = arena.get(id).unwrap();
        let lit_id = node.as_lit_ref().expect("expected LitRef");
        assert_eq!(arena.literals().get(lit_id), Some(&Literal::Int64(7)));
    }

    #[test]
    fn lower_ref_routes_through_lower_path_and_path_pool() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let id = lower(
            &Expr::Ref(Path::<Name>::new("users").get("email")),
            &mut arena,
            &pool,
            &mut budget,
        )
        .unwrap();
        let node = arena.get(id).unwrap();
        let path_id = node.as_path_ref().expect("expected PathRef");
        let lowered = arena.paths().get(path_id).unwrap();
        assert!(lowered.has_fields());
    }

    #[test]
    fn lower_binary_translates_well_known_op_def() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Binary {
            left: Box::new(Expr::Lit(Literal::Int64(1))),
            op: OpDef::new_static(OpDef::ADD, OpCategory::Arithmetic),
            right: Box::new(Expr::Lit(Literal::Int64(2))),
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (op, _, _) = arena.get(id).unwrap().as_bin().unwrap();
        assert_eq!(op, BinOp::Add);
    }

    #[test]
    fn lower_binary_unknown_op_def_returns_unknown_bin_op_error() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Binary {
            left: Box::new(Expr::Param),
            op: OpDef::new_static("MADE_UP", OpCategory::Arithmetic),
            right: Box::new(Expr::Param),
        };
        let err = lower(&e, &mut arena, &pool, &mut budget).unwrap_err();
        match err {
            LowerError::UnknownBinOp(name) => assert_eq!(name, "MADE_UP"),
            other => panic!("expected UnknownBinOp, got {other:?}"),
        }
    }

    #[test]
    fn lower_unary_recurses_and_writes_unary_node() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(Expr::Wildcard),
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (op, child) = arena.get(id).unwrap().as_unary().unwrap();
        assert_eq!(op, UnaryOp::Not);
        assert_eq!(family_of(&arena, child), OpFamily::Wildcard);
    }

    #[test]
    fn lower_seq_parks_operands_in_slab() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Seq(alloc::vec![
            Expr::Lit(Literal::Int64(1)),
            Expr::Lit(Literal::Int64(2)),
            Expr::Lit(Literal::Int64(3)),
        ]);
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let span = arena.get(id).unwrap().as_seq().unwrap();
        let slots = arena.operands().get(span).unwrap();
        assert_eq!(slots.len(), 3);
    }

    #[test]
    fn lower_map_parks_alternating_key_value_slots() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Map(alloc::vec![
            (Name::Static("a"), Expr::Lit(Literal::Int64(1))),
            (Name::Static("b"), Expr::Lit(Literal::Int64(2))),
        ]);
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let span = arena.get(id).unwrap().as_map().unwrap();
        let slots = arena.operands().get(span).unwrap();
        // Two pairs → four slots, even-length invariant.
        assert_eq!(slots.len(), 4);
        // Keys and values alternate; key slots are valid StrIds.
        assert!(slots[0] != 0 && slots[2] != 0);
    }

    #[test]
    fn lower_call_registers_func_and_args() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let f = FuncDef::new_static("LENGTH", Arity::Exact(1), FuncKind::Scalar);
        let e = Expr::Call {
            func: f.clone(),
            args: alloc::vec![Expr::Ref(Path::<Name>::new("name"))],
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (func_id, span) = arena.get(id).unwrap().as_call().unwrap();
        assert_eq!(arena.funcs().get(func_id), Some(&f));
        let slots = arena.operands().get(span).unwrap();
        assert_eq!(slots.len(), 1);
    }

    #[test]
    fn lower_cast_interns_target_type() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Cast {
            expr: Box::new(Expr::Lit(Literal::Int64(1))),
            target_type: DataType::Int32,
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (operand, type_id) = arena.get(id).unwrap().as_cast().unwrap();
        assert_eq!(arena.types().get(type_id), Some(&DataType::Int32));
        assert!(arena.get(operand).is_some());
    }

    #[test]
    fn lower_match_with_and_without_fallback() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        // With fallback.
        let m = Expr::Match {
            arms: alloc::vec![(
                Expr::Lit(Literal::Bool(true)),
                Expr::Lit(Literal::Int64(1))
            )],
            fallback: Some(Box::new(Expr::Lit(Literal::Int64(0)))),
        };
        let id = lower(&m, &mut arena, &pool, &mut budget).unwrap();
        let (span, fallback) = arena.get(id).unwrap().as_match().unwrap();
        assert!(fallback.is_some());
        assert_eq!(arena.operands().get(span).unwrap().len(), 2);

        // Without fallback.
        let m2 = Expr::Match {
            arms: alloc::vec![(
                Expr::Lit(Literal::Bool(true)),
                Expr::Lit(Literal::Int64(7))
            )],
            fallback: None,
        };
        let id2 = lower(&m2, &mut arena, &pool, &mut budget).unwrap();
        let (_, fallback2) = arena.get(id2).unwrap().as_match().unwrap();
        assert!(fallback2.is_none());
    }

    #[test]
    fn lower_if_writes_three_operand_node() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::If {
            cond: Box::new(Expr::Lit(Literal::Bool(true))),
            then_expr: Box::new(Expr::Lit(Literal::Int64(1))),
            else_expr: Box::new(Expr::Lit(Literal::Int64(2))),
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (_, t, _) = arena.get(id).unwrap().as_if().unwrap();
        assert!(arena.get(t).is_some());
    }

    #[test]
    fn lower_in_range_writes_three_operand_node() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::InRange {
            expr: Box::new(Expr::Ref(Path::<Name>::new("age"))),
            low: Box::new(Expr::Lit(Literal::Int64(18))),
            high: Box::new(Expr::Lit(Literal::Int64(99))),
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (_, _, _) = arena.get(id).unwrap().as_in_range().unwrap();
    }

    #[test]
    fn lower_member_of_writes_set_span() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::MemberOf {
            expr: Box::new(Expr::Ref(Path::<Name>::new("status"))),
            set: alloc::vec![
                Expr::Lit(Literal::String("a".into())),
                Expr::Lit(Literal::String("b".into())),
            ],
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (_, span) = arena.get(id).unwrap().as_member_of().unwrap();
        assert_eq!(arena.operands().get(span).unwrap().len(), 2);
    }

    #[test]
    fn lower_label_interns_name() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let e = Expr::Label {
            expr: Box::new(Expr::Lit(Literal::Int64(1))),
            name: Name::Static("alias"),
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (_, name_id) = arena.get(id).unwrap().as_label().unwrap();
        let resolver: &(dyn StringResolver + 'static) = &pool;
        let name_str = resolver.resolve_str(name_id).unwrap();
        assert_eq!(name_str, "alias");
    }

    #[test]
    fn lower_scoped_lowers_partition_order_and_frame() {
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let ctx = Context {
            partition_by: alloc::vec![Expr::Ref(Path::<Name>::new("user"))],
            order_by: alloc::vec![OrderByExpr {
                expr: Expr::Ref(Path::<Name>::new("ts")),
                dir: SortDirection::Desc,
                nulls: NullsOrder::First,
            }],
            frame: Some(Frame::rows(
                Boundary::unbounded_preceding(),
                Boundary::Current,
            )),
        };
        let e = Expr::Scoped {
            expr: Box::new(Expr::CountAll),
            context: ctx,
        };
        let id = lower(&e, &mut arena, &pool, &mut budget).unwrap();
        let (inner, ctx_id) = arena.get(id).unwrap().as_scoped().unwrap();
        assert_eq!(family_of(&arena, inner), OpFamily::CountAll);
        let lowered_ctx = arena.contexts().get(ctx_id).unwrap();
        assert_eq!(lowered_ctx.partition_by.len(), 1);
        assert_eq!(lowered_ctx.order_by.len(), 1);
        assert_eq!(lowered_ctx.order_by[0].dir, SortDirection::Desc);
        assert_eq!(lowered_ctx.order_by[0].nulls, NullsOrder::First);
        assert!(lowered_ctx.frame.is_some());
    }

    #[test]
    fn lower_then_intern_dedups_structurally_equal_subtrees() {
        // Two interns of the same Param write the same node id —
        // proves the lowerer routes through `intern_node`.
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let a = lower(&Expr::Wildcard, &mut arena, &pool, &mut budget).unwrap();
        let b = lower(&Expr::Wildcard, &mut arena, &pool, &mut budget).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn lower_charges_depth_per_descent() {
        // A 3-level expression ((a + b) * c) should consume at least
        // 3 depth ticks because every recursive descent charges one.
        let mut arena = fresh_arena();
        let pool = StringPool::standard();
        let mut budget = Budget::from_config(&BudgetConfig {
            max_depth: 2,
            max_nodes: 1024,
            max_bytes: 1 << 20,
        });
        let e = Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Param),
                op: OpDef::new_static(OpDef::ADD, OpCategory::Arithmetic),
                right: Box::new(Expr::Param),
            }),
            op: OpDef::new_static(OpDef::MUL, OpCategory::Arithmetic),
            right: Box::new(Expr::Param),
        };
        let err = lower(&e, &mut arena, &pool, &mut budget).unwrap_err();
        assert!(matches!(
            err,
            LowerError::BudgetExceeded(BudgetExceeded::Depth)
        ));
    }

    #[test]
    fn cross_arena_structural_equivalence_yields_identical_content_hash() {
        // The same Expr lowered into two arenas (with different
        // ambient pools) yields the same content-hash digest.
        use crate::expr::walk::content_hash;
        use dol_cas::content_index::ContentIndex;

        let pool = StringPool::standard();
        let mut budget1 = fresh_budget();
        let mut arena1 = ExprArena::new();
        let e1 = Expr::Binary {
            left: Box::new(Expr::Ref(Path::<Name>::new("x"))),
            op: OpDef::new_static(OpDef::ADD, OpCategory::Arithmetic),
            right: Box::new(Expr::Lit(Literal::Int64(1))),
        };
        let id1 = lower(&e1, &mut arena1, &pool, &mut budget1).unwrap();

        // Allocate noise nodes first so ids differ between arenas.
        let mut arena2 = ExprArena::new();
        let _ = arena2.intern_node(ExprNode::wildcard()).unwrap();
        let _ = arena2.intern_node(ExprNode::count_all()).unwrap();
        let mut budget2 = fresh_budget();
        let e2 = Expr::Binary {
            left: Box::new(Expr::Ref(Path::<Name>::new("x"))),
            op: OpDef::new_static(OpDef::ADD, OpCategory::Arithmetic),
            right: Box::new(Expr::Lit(Literal::Int64(1))),
        };
        let id2 = lower(&e2, &mut arena2, &pool, &mut budget2).unwrap();
        assert_ne!(id1, id2);

        let mut idx1 = ContentIndex::new();
        let mut idx2 = ContentIndex::new();
        let mut b = fresh_budget();
        let h1 = content_hash(&arena1, id1, &mut idx1, &mut b).unwrap();
        let mut b = fresh_budget();
        let h2 = content_hash(&arena2, id2, &mut idx2, &mut b).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn distinct_subtrees_have_distinct_content_hashes() {
        use crate::expr::walk::content_hash;
        use dol_cas::content_index::ContentIndex;

        let pool = StringPool::standard();
        let mut budget = fresh_budget();
        let mut arena = ExprArena::new();
        let id_seq = lower(
            &Expr::Seq(alloc::vec![Expr::Param, Expr::Param]),
            &mut arena,
            &pool,
            &mut budget,
        )
        .unwrap();
        let id_seq_three = lower(
            &Expr::Seq(alloc::vec![Expr::Param, Expr::Param, Expr::Param]),
            &mut arena,
            &pool,
            &mut budget,
        )
        .unwrap();

        let mut idx = ContentIndex::new();
        let mut b = fresh_budget();
        let h_two = content_hash(&arena, id_seq, &mut idx, &mut b).unwrap();
        let h_three = content_hash(&arena, id_seq_three, &mut idx, &mut b).unwrap();
        assert_ne!(h_two, h_three);
    }
}

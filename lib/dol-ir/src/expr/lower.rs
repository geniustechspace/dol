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

use dol_cas::handle::StrId;
use dol_cas::string_pool::{InternError, StringPool};
use dol_core::budget::{Budget, BudgetExceeded};
use dol_core::path::Path;
use dol_core::strings::Name;

/// First-violation enum returned by every lowering routine. Variants
/// surface the underlying primitive failure (budget, intern, arena)
/// rather than wrapping it behind a generic "lower failed" code, so
/// callers can route each class to the right diagnostic.
///
/// Final shape per `dol-rewrite-plan-v2.md` §8.4 line 1396; the
/// upcoming `lower(Expr)` pass will reuse this enum unchanged.
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
    /// The [`ExprArena`](super::arena::ExprArena) refused to allocate
    /// a node because its slot table would overflow `u32::MAX`. Only
    /// produced by the deferred `lower(Expr)` pass; reserved here so
    /// callers can match exhaustively from day one.
    ArenaOverflow,
}

impl core::fmt::Display for LowerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BudgetExceeded(inner) => write!(f, "lower: {inner}"),
            Self::InternFailed(inner) => write!(f, "lower: {inner}"),
            Self::ArenaOverflow => f.write_str("lower: arena overflow"),
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
}

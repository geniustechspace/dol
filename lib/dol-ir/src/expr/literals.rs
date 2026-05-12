//! `LiteralPool` — typed-arena wrapper that maps [`LiteralId`] to
//! [`Literal<'static>`].
//!
//! Per `dol-rewrite-plan-v2.md` §3.3 ("side pools") and §8.1 line 1095
//! ([`ExprNode::lit_ref(LiteralId)`](super::node::ExprNode::lit_ref)).
//!
//! ## Role in the lowering pipeline
//!
//! `Expr::Lit(Literal<'a>)` (the tree-DSL form) lowers to
//! `ExprNode::lit_ref(LiteralId)` (the arena form). The pool is the
//! carrier that holds the actual literal payloads keyed by id, so the
//! arena nodes can stay 16 bytes wide and the lifetime `'a` of the
//! input tree can be erased (every interned literal is copied into a
//! `Literal<'static>` via [`Literal::into_static`]).
//!
//! ## What this slice ships (M3c-δ₂b prereq #1)
//!
//! - Push-only intern API. Each [`LiteralPool::intern`] call allocates
//!   a fresh slot — there is no structural dedup yet.
//! - Typed [`LiteralId`] handles via [`DynPool::push_id`].
//!
//! ## Out of scope (future slice)
//!
//! - Content-addressed dedup keyed by `fast128`. Literal does not
//!   implement `Hash` (because of `f64`) and the in-memory layout
//!   spans many feature-gated variants, so dedup needs its own
//!   slice with a deliberate stable byte serialisation.
//! - `to_cid()`-style cross-process content addresses.

extern crate alloc;

use dol_cas::handle::LiteralId;
use dol_cas::pool::{ArenaStorage, DynPool};
use dol_core::literal::Literal;

/// First-violation enum returned by [`LiteralPool::intern`]. A
/// dedicated enum (rather than `Option<LiteralId>`) so future variants
/// (capacity overflow on `StaticPool`, dedup-index errors, …) compose
/// cleanly with `?`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LiteralPoolError {
    /// The pool's slot table would overflow [`u32::MAX`] — the
    /// [`LiteralId`] niche limit. Same shape as
    /// [`crate::expr::lower::LowerError::ArenaOverflow`] for the
    /// `ExprArena`.
    CapacityExceeded,
}

impl core::fmt::Display for LiteralPoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("literal pool: capacity exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LiteralPoolError {}

/// Typed arena of [`Literal<'static>`] addressed by [`LiteralId`].
///
/// Backed by [`DynPool<Literal<'static>>`]. `Send + Sync` since
/// [`Literal<'static>`] is `Send + Sync` (all variants own their
/// payloads or are POD).
///
/// The pool is **append-only**: once an id is handed out it is stable
/// for the lifetime of the pool, and existing entries are never
/// mutated or removed.
#[derive(Debug, Clone)]
pub struct LiteralPool {
    items: DynPool<Literal<'static>>,
}

impl Default for LiteralPool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl LiteralPool {
    /// Construct an empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: DynPool::new(),
        }
    }

    /// Pre-allocate capacity for `cap` literals.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: DynPool::with_capacity(cap),
        }
    }

    /// Intern a literal — allocates a fresh slot and returns its
    /// [`LiteralId`]. Borrowed payloads (`Literal::Str(&'a str)` etc.)
    /// are promoted to owned form via [`Literal::into_static`] so the
    /// pool can be `'static`.
    ///
    /// This slice does **not** perform content-addressed dedup;
    /// repeated intern calls with the same value allocate distinct
    /// ids. A future slice will key dedup on a stable byte
    /// serialisation of the literal.
    ///
    /// # Errors
    ///
    /// Returns [`LiteralPoolError::CapacityExceeded`] when the pool
    /// would exceed [`u32::MAX`] entries.
    pub fn intern(&mut self, lit: &Literal<'_>) -> Result<LiteralId, LiteralPoolError> {
        let owned = lit.clone().into_static();
        self.items
            .push_id::<dol_cas::handle::tags::LiteralTag>(owned)
            .ok_or(LiteralPoolError::CapacityExceeded)
    }

    /// Borrow a literal by id.
    #[must_use]
    pub fn get(&self, id: LiteralId) -> Option<&Literal<'static>> {
        self.items.get_by_id(id)
    }

    /// Number of literals currently in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` when the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::Cow;

    #[test]
    fn empty_pool_round_trip() {
        let pool = LiteralPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn intern_returns_a_typed_literal_id() {
        let mut pool = LiteralPool::new();
        let id = pool.intern(&Literal::Bool(true)).unwrap();
        // `LiteralId` is `Lid<LiteralTag>`; first slot is index 0 / id 1.
        assert_eq!(id.index(), 0);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn intern_does_not_dedup_yet() {
        // Document the M3c-δ₂b-prereq invariant: this slice is
        // push-only. A future slice will add structural dedup keyed
        // on a stable byte serialisation.
        let mut pool = LiteralPool::new();
        let a = pool.intern(&Literal::Bool(true)).unwrap();
        let b = pool.intern(&Literal::Bool(true)).unwrap();
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn distinct_values_get_distinct_ids() {
        let mut pool = LiteralPool::new();
        let a = pool.intern(&Literal::Null).unwrap();
        let b = pool.intern(&Literal::Bool(false)).unwrap();
        let c = pool.intern(&Literal::Bool(true)).unwrap();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn get_round_trips_an_interned_literal() {
        let mut pool = LiteralPool::new();
        let id = pool.intern(&Literal::Null).unwrap();
        assert_eq!(pool.get(id), Some(&Literal::Null));
    }

    #[test]
    fn borrowed_string_is_promoted_to_owned() {
        // The pool stores `Literal<'static>`; the borrowed `Str`
        // input must be promoted via `into_static` so the entry
        // outlives any borrow held during `intern`.
        let mut pool = LiteralPool::new();
        let s: alloc::string::String = "ephemeral".into();
        let id = {
            let borrowed = Literal::String(Cow::Borrowed(s.as_str()));
            pool.intern(&borrowed).unwrap()
        };
        // Drop the source string; the pool entry must still be valid
        // because the value was cloned into static form.
        drop(s);
        let got = pool.get(id).unwrap();
        match got {
            Literal::String(Cow::Owned(text)) => assert_eq!(text.as_str(), "ephemeral"),
            other => panic!("expected owned String, got {other:?}"),
        }
    }

    #[test]
    fn ids_are_dense_and_one_based_via_lid() {
        // Sanity-check the `Lid` contract — the first id has index 0
        // (NonZeroU32 inner = 1), and ids are dense.
        let mut pool = LiteralPool::new();
        let id0 = pool.intern(&Literal::Null).unwrap();
        let id1 = pool.intern(&Literal::Bool(false)).unwrap();
        let id2 = pool.intern(&Literal::Bool(true)).unwrap();
        assert_eq!(id0.index(), 0);
        assert_eq!(id1.index(), 1);
        assert_eq!(id2.index(), 2);
    }

    #[test]
    fn unknown_id_returns_none() {
        let pool = LiteralPool::new();
        let bogus: LiteralId = dol_cas::handle::Lid::from_u32(99).unwrap();
        assert!(pool.get(bogus).is_none());
    }

    #[test]
    fn pool_error_display_is_human_readable() {
        let s = alloc::format!("{}", LiteralPoolError::CapacityExceeded);
        assert!(s.contains("capacity"));
    }
}

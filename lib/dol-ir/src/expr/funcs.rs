//! `FuncRegistry` — typed-arena wrapper that maps [`FuncId`] to
//! [`FuncDef`], deduplicated by function name.
//!
//! Per `dol-rewrite-plan-v2.md` §3.3 ("side pools") and §8.1 lines
//! 1097 ([`ExprNode::func_ref(FuncId)`](super::node::ExprNode::func_ref))
//! / §8.2 ([`Expr::Call { func: FuncDef, … }`](super::tree)).
//!
//! ## Role in the lowering pipeline
//!
//! `Expr::Call { func, args }` (tree-DSL) lowers to
//! `ExprNode::func_ref(FuncId)` (arena form). The registry is the
//! carrier that holds the [`FuncDef`] (name + arity + kind) keyed by
//! the typed [`FuncId`] handle so the arena node can stay 16 bytes
//! wide.
//!
//! ## What this slice ships (M3c-δ₂b prereq #2)
//!
//! - Name-keyed structural dedup. Two calls to [`FuncRegistry::intern`]
//!   with [`FuncDef`]s sharing the same name return the **same**
//!   [`FuncId`]; the original [`FuncDef`] (with its declared arity /
//!   kind) is preserved — later interns of a same-named entry do
//!   **not** overwrite arity / kind.
//! - Typed [`FuncId`] handles via [`DynPool::push_id`].
//!
//! ## Why dedup is in this slice (vs. push-only `LiteralPool`)
//!
//! A function in DOL is identified by its name (`LENGTH`, `COUNT`,
//! …). Multiple `Expr::Call` nodes invoking the same function must
//! lower to the **same** [`FuncId`]; otherwise the structural-dedup
//! property of the surrounding `ExprArena` collapses — two equivalent
//! `LENGTH(x)` calls would receive different `ExprNode::func_ref`
//! payloads.
//!
//! Dedup keys on the function's name (as `Box<str>`). Because
//! [`FuncDef::name`] returns the [`&str`] view shared by both
//! [`dol_core::strings::Name::Static`] and `Name::Owned` storage,
//! interning `FuncDef::new_static("UPPER", …)` and
//! `FuncDef::custom_with("UPPER", …)` returns the **same**
//! [`FuncId`].
//!
//! ## Out of scope (future slices)
//!
//! - Cross-process content addresses (`to_cid`) for func names.
//! - Conflict detection when two interns disagree on arity / kind
//!   for the same name — for now the first registration wins and a
//!   later registration is silently ignored. A diagnostic-aware
//!   variant can ride the larger `lower(Expr)` slice.

extern crate alloc;

use alloc::boxed::Box;

use hashbrown::HashMap;

use dol_cas::handle::FuncId;
use dol_cas::pool::{ArenaStorage, DynPool};

use super::meta::FuncDef;

/// Errors returned by [`FuncRegistry::intern`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FuncRegistryError {
    /// The pool's slot table would overflow [`u32::MAX`] — the
    /// [`FuncId`] niche limit. Mirrors the
    /// [`crate::expr::literals::LiteralPoolError::CapacityExceeded`]
    /// and [`crate::expr::lower::LowerError::ArenaOverflow`] shapes
    /// so the upcoming recursive lowerer can compose them with `?`.
    CapacityExceeded,
}

impl core::fmt::Display for FuncRegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("func registry: capacity exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FuncRegistryError {}

/// Typed arena of [`FuncDef`]s addressed by [`FuncId`], with
/// name-keyed structural dedup.
///
/// Backed by [`DynPool<FuncDef>`]. `Send + Sync` since `FuncDef` is
/// `Send + Sync`. The registry is **append-only**: once a [`FuncId`]
/// is handed out it is stable for the registry's lifetime.
#[derive(Debug, Clone)]
pub struct FuncRegistry {
    items: DynPool<FuncDef>,
    /// `name -> FuncId` dedup index. Keyed on `Box<str>` so lookups
    /// can use `&str` borrows directly. Names are content-equal
    /// regardless of whether the source `FuncDef` used
    /// [`dol_core::strings::Name::Static`] or `Name::Owned`.
    index: HashMap<Box<str>, FuncId>,
}

impl Default for FuncRegistry {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl FuncRegistry {
    /// Construct an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: DynPool::new(),
            index: HashMap::new(),
        }
    }

    /// Pre-allocate capacity for `cap` distinct function entries.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: DynPool::with_capacity(cap),
            index: HashMap::with_capacity(cap),
        }
    }

    /// Intern a function definition — returns the existing [`FuncId`]
    /// when a [`FuncDef`] with the same name is already registered,
    /// otherwise pushes a fresh slot and records it in the dedup
    /// index.
    ///
    /// The **first** registration of a name wins — repeated interns
    /// with the same name but different arity / kind are silently
    /// reduced to the original. Callers that need strict-conflict
    /// reporting can pre-check via [`Self::get_by_name`].
    ///
    /// # Errors
    ///
    /// Returns [`FuncRegistryError::CapacityExceeded`] when the
    /// registry would exceed [`u32::MAX`] distinct entries.
    pub fn intern(&mut self, func: &FuncDef) -> Result<FuncId, FuncRegistryError> {
        if let Some(&existing) = self.index.get(func.name()) {
            return Ok(existing);
        }
        let id = self
            .items
            .push_id::<dol_cas::handle::tags::FuncTag>(func.clone())
            .ok_or(FuncRegistryError::CapacityExceeded)?;
        self.index.insert(func.name().into(), id);
        Ok(id)
    }

    /// Borrow a function definition by id.
    #[must_use]
    pub fn get(&self, id: FuncId) -> Option<&FuncDef> {
        self.items.get_by_id(id)
    }

    /// Look up a function by name without inserting.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<FuncId> {
        self.index.get(name).copied()
    }

    /// Number of distinct function entries in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` when the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// Borrow `FuncDef`'s name as `&str` through its public accessor.
// The dedup index is keyed on `Box<str>` so lookups consume `&str`
// borrows directly without any helper.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::meta::{Arity, FuncKind};

    fn count_func() -> FuncDef {
        FuncDef::new_static("COUNT", Arity::Exact(1), FuncKind::Aggregate)
    }
    fn length_func() -> FuncDef {
        FuncDef::new_static("LENGTH", Arity::Exact(1), FuncKind::Scalar)
    }

    #[test]
    fn empty_registry_round_trip() {
        let reg = FuncRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.get_by_name("COUNT").is_none());
    }

    #[test]
    fn intern_dedups_by_name() {
        let mut reg = FuncRegistry::new();
        let a = reg.intern(&count_func()).unwrap();
        let b = reg.intern(&count_func()).unwrap();
        assert_eq!(a, b);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn distinct_names_get_distinct_ids() {
        let mut reg = FuncRegistry::new();
        let a = reg.intern(&count_func()).unwrap();
        let b = reg.intern(&length_func()).unwrap();
        assert_ne!(a, b);
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn first_registration_wins_on_conflict() {
        // Document the M3c-δ₂b-prereq policy: when a name is interned
        // twice with disagreeing arity / kind, the original entry
        // is preserved unchanged. A future strict-conflict slice
        // can promote this into a `FuncRegistryError::Conflict`.
        let mut reg = FuncRegistry::new();
        let id = reg.intern(&count_func()).unwrap();
        let conflicting =
            FuncDef::new_static("COUNT", Arity::Exact(0), FuncKind::Scalar);
        let id2 = reg.intern(&conflicting).unwrap();
        assert_eq!(id, id2);
        let got = reg.get(id).unwrap();
        // Original arity/kind preserved:
        assert_eq!(got.arity(), Arity::Exact(1));
        assert_eq!(got.kind(), FuncKind::Aggregate);
    }

    #[test]
    fn get_round_trips_an_interned_def() {
        let mut reg = FuncRegistry::new();
        let id = reg.intern(&count_func()).unwrap();
        let got = reg.get(id).unwrap();
        assert_eq!(got.name(), "COUNT");
        assert_eq!(got.arity(), Arity::Exact(1));
        assert_eq!(got.kind(), FuncKind::Aggregate);
    }

    #[test]
    fn get_by_name_returns_existing_id() {
        let mut reg = FuncRegistry::new();
        let id = reg.intern(&length_func()).unwrap();
        assert_eq!(reg.get_by_name("LENGTH"), Some(id));
        assert!(reg.get_by_name("MISSING").is_none());
    }

    #[test]
    fn static_and_owned_name_dedup_together() {
        // `Name::Static("FOO") == Name::Owned("FOO".into())` per
        // `dol_core::strings::Name`. The registry must respect that
        // content-equality so DSL builders that emit `FuncDef::custom`
        // dedup against `FuncDef::new_static` of the same name.
        let mut reg = FuncRegistry::new();
        let stat = FuncDef::new_static("UPPER", Arity::Exact(1), FuncKind::Scalar);
        let dynm: FuncDef =
            FuncDef::custom_with("UPPER", Arity::Exact(1), FuncKind::Scalar);
        let a = reg.intern(&stat).unwrap();
        let b = reg.intern(&dynm).unwrap();
        assert_eq!(a, b);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn ids_are_dense_and_one_based_via_lid() {
        let mut reg = FuncRegistry::new();
        let id0 = reg.intern(&count_func()).unwrap();
        let id1 = reg.intern(&length_func()).unwrap();
        assert_eq!(id0.index(), 0);
        assert_eq!(id1.index(), 1);
    }

    #[test]
    fn unknown_id_returns_none() {
        let reg = FuncRegistry::new();
        let bogus: FuncId = dol_cas::handle::Lid::from_u32(99).unwrap();
        assert!(reg.get(bogus).is_none());
    }

    #[test]
    fn error_display_is_human_readable() {
        let s = alloc::format!("{}", FuncRegistryError::CapacityExceeded);
        assert!(s.contains("capacity"));
    }
}

//! `TypePool` — typed-arena wrapper that maps [`TypeId`] to
//! [`DataType`].
//!
//! Per `dol-rewrite-plan-v2.md` §3.3 ("side pools") and §8.4 (lowering
//! of `Expr::Cast`).
//!
//! ## Role in the lowering pipeline
//!
//! `Expr::Cast { expr, target_type: DataType }` (the tree-DSL form)
//! lowers to `ExprNode::cast(NodeId, TypeId)` (the arena form). The
//! pool is the carrier that holds the actual [`DataType`] payloads
//! keyed by id, so the arena nodes can stay 16 bytes wide.
//!
//! ## What this slice ships
//!
//! - Push-only intern API mirroring [`LiteralPool`](crate::expr::literals::LiteralPool):
//!   each [`TypePool::intern`] call allocates a fresh slot — no
//!   structural dedup yet.
//! - Typed [`TypeId`] handles via [`DynPool::push_id`].
//!
//! ## Why dedup is deferred
//!
//! [`DataType`] derives `PartialEq + Eq` but not `Hash` (composite
//! variants own `Box<DataType>` / `Vec<DataType>` / `Box<str>` and a
//! stable hash needs a deliberate canonical byte serialisation that
//! has not yet shipped). Push-only matches the
//! [`LiteralPool`](crate::expr::literals::LiteralPool) policy and is
//! sound — `ExprArena::intern_node` keys on the raw 16-byte node
//! image, so two `Cast` calls into the same `DataType` *will* receive
//! distinct `TypeId`s and therefore distinct interned `Cast` nodes,
//! but never panic or collapse incorrectly.

extern crate alloc;

use dol_cas::handle::TypeId;
use dol_cas::pool::{ArenaStorage, DynPool};
use dol_core::data_type::DataType;

/// Errors returned by [`TypePool::intern`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypePoolError {
    /// The pool's slot table would overflow [`u32::MAX`] — the
    /// [`TypeId`] niche limit. Mirrors the
    /// [`LiteralPoolError::CapacityExceeded`](crate::expr::literals::LiteralPoolError),
    /// [`FuncRegistryError::CapacityExceeded`](crate::expr::funcs::FuncRegistryError),
    /// [`OperandSlabError::CapacityExceeded`](crate::expr::slab::OperandSlabError),
    /// [`PathPoolError::CapacityExceeded`](crate::expr::paths::PathPoolError),
    /// and [`LowerError::ArenaOverflow`](crate::expr::lower::LowerError)
    /// shapes so the recursive lowerer composes them uniformly.
    CapacityExceeded,
}

impl core::fmt::Display for TypePoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("type pool: capacity exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TypePoolError {}

/// Typed arena of [`DataType`] addressed by [`TypeId`]. Append-only;
/// once an id is handed out it is stable for the lifetime of the pool.
#[derive(Debug, Clone)]
pub struct TypePool {
    items: DynPool<DataType>,
}

impl Default for TypePool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl TypePool {
    /// Construct an empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: DynPool::new(),
        }
    }

    /// Pre-allocate capacity for `cap` types.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: DynPool::with_capacity(cap),
        }
    }

    /// Intern a [`DataType`] — clones the input and allocates a fresh
    /// slot. No structural dedup in this slice (see module docs).
    ///
    /// # Errors
    ///
    /// Returns [`TypePoolError::CapacityExceeded`] when the pool would
    /// exceed [`u32::MAX`] entries.
    pub fn intern(&mut self, ty: &DataType) -> Result<TypeId, TypePoolError> {
        self.items
            .push_id::<dol_cas::handle::tags::TypeTag>(ty.clone())
            .ok_or(TypePoolError::CapacityExceeded)
    }

    /// Borrow a type by id.
    #[must_use]
    pub fn get(&self, id: TypeId) -> Option<&DataType> {
        self.items.get_by_id(id)
    }

    /// Number of types currently in the pool.
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

    #[test]
    fn empty_pool_round_trip() {
        let pool = TypePool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn intern_and_get() {
        let mut pool = TypePool::new();
        let id = pool.intern(&DataType::Int32).unwrap();
        assert_eq!(pool.get(id), Some(&DataType::Int32));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn distinct_types_get_distinct_ids() {
        let mut pool = TypePool::new();
        let a = pool.intern(&DataType::Int32).unwrap();
        let b = pool.intern(&DataType::Int64).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn intern_does_not_dedup_yet() {
        // Documents the push-only policy mirroring LiteralPool.
        let mut pool = TypePool::new();
        let a = pool.intern(&DataType::Int32).unwrap();
        let b = pool.intern(&DataType::Int32).unwrap();
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn pool_error_display_is_human_readable() {
        let s = alloc::format!("{}", TypePoolError::CapacityExceeded);
        assert!(s.contains("capacity"));
    }
}

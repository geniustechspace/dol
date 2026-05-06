//! Storage-layer trait shapes (`KvStore`, `Catalog`) — per
//! `docs/v2_plan.md` §71-72.
//!
//! These traits mark the **only** seam between the IR layer and a concrete
//! backend (RocksDB, sled, an in-memory store, an MCU flash partition).
//! Backends depend on `lib/ir` for these traits and on `lib/wire` for
//! serialisation; nothing in `lib/ir` depends on `lib/expr` or
//! `lib/pipeline`, which keeps the dependency DAG strictly acyclic
//! (workspace `Cargo.toml` enforces this comment-wise; `cargo deny` is
//! the runtime gate).
//!
//! Both traits are intentionally minimal:
//!
//! - **No transactions in this trait surface.** Cross-key atomicity is
//!   modelled at a higher layer ([`crate::Operation::Tx`] in the IR);
//!   each `KvStore` impl decides how to serve a transactional verb (a
//!   write batch on RocksDB, a memory snapshot for the in-memory impl,
//!   etc.).
//! - **No iteration semantics baked in.** Range scans live in the
//!   capability set ([`crate::CapabilityTag`]); a flash-resident MCU
//!   backend reports `Scan` as unsupported and the planner refuses any
//!   plan that needs it.
//! - **Borrowed-or-owned values.** `Get` returns
//!   `Result<Option<Self::Value>, _>` with `Self::Value: AsRef<[u8]>` so
//!   embedded impls can hand back a stack array and server impls can
//!   hand back a `Vec<u8>` without forcing one or the other.
//! - **Budget-threaded.** Every recursive entry point takes
//!   `&mut Budget`, matching the workspace-wide v2 invariant.
//!
//! Concrete implementations live in `backends/*` once the first one
//! lands; this module only exists to give those backends a stable target.
//!
//! # Example
//!
//! ```
//! use dol_core::policy::{Budget, Limits};
//! use dol_ir::store::{KvError, KvStore};
//!
//! #[derive(Default)]
//! struct MemStore(std::collections::BTreeMap<Vec<u8>, Vec<u8>>);
//!
//! impl KvStore for MemStore {
//!     type Value = Vec<u8>;
//!     type Error = core::convert::Infallible;
//!
//!     fn get(&self, key: &[u8], _budget: &mut Budget) -> Result<Option<Self::Value>, KvError<Self::Error>> {
//!         Ok(self.0.get(key).cloned())
//!     }
//!     fn put(&mut self, key: &[u8], value: &[u8], _budget: &mut Budget) -> Result<(), KvError<Self::Error>> {
//!         self.0.insert(key.to_vec(), value.to_vec());
//!         Ok(())
//!     }
//!     fn delete(&mut self, key: &[u8], _budget: &mut Budget) -> Result<(), KvError<Self::Error>> {
//!         self.0.remove(key);
//!         Ok(())
//!     }
//! }
//!
//! let mut store = MemStore::default();
//! let mut budget = Budget::new(Limits::host());
//! store.put(b"a", b"1", &mut budget).unwrap();
//! let v = store.get(b"a", &mut budget).unwrap().unwrap();
//! assert_eq!(v, b"1");
//! ```

use core::fmt;

use dol_core::policy::{Budget, BudgetError};

use crate::SchemaCatalog;

/// Errors produced by a [`KvStore`] or [`Catalog`] operation.
///
/// The `E` parameter carries the backend-specific cause; `Budget` and
/// `NotFound` are common across every backend so the planner can match
/// on them generically.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KvError<E> {
    /// The supplied [`Budget`] was exhausted before the operation
    /// completed. Always thread `&mut Budget` so the caller sees the
    /// remaining quota.
    Budget(BudgetError),
    /// The requested key was absent. Distinguished from `get → Ok(None)`
    /// only on operations where absence is itself an error
    /// (e.g. [`Catalog::load`]).
    NotFound,
    /// Backend-specific failure — I/O error, connection drop, schema
    /// version mismatch, …
    Backend(E),
}

impl<E: fmt::Display> fmt::Display for KvError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Budget(e) => write!(f, "kv: budget exhausted ({e})"),
            Self::NotFound => f.write_str("kv: key not found"),
            Self::Backend(e) => write!(f, "kv: backend error: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug + fmt::Display> std::error::Error for KvError<E> {}

impl<E> From<BudgetError> for KvError<E> {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}

/// Minimal byte-addressable key/value interface every backend must
/// implement.
///
/// Implementations decide their own value-borrowing story via
/// [`Self::Value`] (`Vec<u8>` on hosts, `heapless::Vec<u8, N>` on MCUs,
/// `&'a [u8]` on memory-mapped backends). Keys are always borrowed.
///
/// Higher-level capabilities (range scans, conditional puts, secondary
/// indices, …) are modelled in [`crate::CapabilitySet`] and are not part
/// of this trait so the smallest possible MCU backend can implement
/// `KvStore` in ~50 lines.
pub trait KvStore {
    /// The value type returned by [`Self::get`]. `AsRef<[u8]>` lets
    /// callers consume the bytes without forcing a particular owner.
    type Value: AsRef<[u8]>;

    /// Backend-specific failure cause (`Infallible` for in-memory impls).
    type Error: fmt::Debug;

    /// Look up `key`. Returns `Ok(None)` for clean misses; reserve
    /// [`KvError::NotFound`] for layers that distinguish absence as an
    /// error (e.g. [`Catalog`]).
    fn get(
        &self,
        key: &[u8],
        budget: &mut Budget,
    ) -> Result<Option<Self::Value>, KvError<Self::Error>>;

    /// Store `value` under `key`, replacing any existing entry.
    fn put(
        &mut self,
        key: &[u8],
        value: &[u8],
        budget: &mut Budget,
    ) -> Result<(), KvError<Self::Error>>;

    /// Remove `key`. A missing key is *not* an error — backends that
    /// need stricter semantics build them on top.
    fn delete(&mut self, key: &[u8], budget: &mut Budget) -> Result<(), KvError<Self::Error>>;
}

/// Persistence interface for the [`SchemaCatalog`] that every IR
/// [`crate::Program`] carries.
///
/// A [`Catalog`] is intentionally *not* a [`KvStore`] specialised to
/// catalog bytes — backends often want to keep the catalog in a
/// dedicated namespace (separate column family, separate flash page,
/// metadata sidecar) and serialise it through `dol-wire`'s `Signed<T>`
/// envelope rather than as opaque bytes.
///
/// Concrete impls typically delegate to a [`KvStore`] under the hood and
/// add the wire-encode / wire-decode + signature verification layer.
pub trait Catalog {
    /// Backend-specific failure cause.
    type Error: fmt::Debug;

    /// Load and return the persisted [`SchemaCatalog`], or
    /// [`KvError::NotFound`] if no catalog has been stored yet.
    fn load(&self, budget: &mut Budget) -> Result<SchemaCatalog, KvError<Self::Error>>;

    /// Persist `catalog`, replacing any previously stored value.
    /// Implementations should treat this as atomic from a reader's
    /// perspective — no partial-catalog reads.
    fn store(
        &mut self,
        catalog: &SchemaCatalog,
        budget: &mut Budget,
    ) -> Result<(), KvError<Self::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use alloc::vec::Vec;
    use dol_core::policy::{Budget, Limits};

    #[derive(Default)]
    struct MemStore {
        inner: BTreeMap<Vec<u8>, Vec<u8>>,
    }

    impl KvStore for MemStore {
        type Value = Vec<u8>;
        type Error = core::convert::Infallible;

        fn get(
            &self,
            key: &[u8],
            _budget: &mut Budget,
        ) -> Result<Option<Self::Value>, KvError<Self::Error>> {
            Ok(self.inner.get(key).cloned())
        }
        fn put(
            &mut self,
            key: &[u8],
            value: &[u8],
            _budget: &mut Budget,
        ) -> Result<(), KvError<Self::Error>> {
            self.inner.insert(key.to_vec(), value.to_vec());
            Ok(())
        }
        fn delete(&mut self, key: &[u8], _budget: &mut Budget) -> Result<(), KvError<Self::Error>> {
            self.inner.remove(key);
            Ok(())
        }
    }

    #[test]
    fn put_get_delete_roundtrip() {
        let mut s = MemStore::default();
        let mut b = Budget::new(Limits::host());
        s.put(b"k", b"v", &mut b).unwrap();
        assert_eq!(s.get(b"k", &mut b).unwrap().as_deref(), Some(&b"v"[..]));
        s.delete(b"k", &mut b).unwrap();
        assert!(s.get(b"k", &mut b).unwrap().is_none());
    }

    #[test]
    fn missing_key_returns_ok_none() {
        let s = MemStore::default();
        let mut b = Budget::new(Limits::host());
        assert!(s.get(b"absent", &mut b).unwrap().is_none());
    }

    #[test]
    fn budget_error_converts() {
        let e: KvError<core::convert::Infallible> = BudgetError::Nodes.into();
        assert!(matches!(e, KvError::Budget(BudgetError::Nodes)));
    }
}

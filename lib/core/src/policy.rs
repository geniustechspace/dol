//! # `policy` — resource limits and traversal budgets.
//!
//! `dol-core` ships two complementary primitives that every recursive,
//! allocating, or untrusted-input-driven path in the DOL stack is expected
//! to thread:
//!
//! - [`Limits`] — a static, [`Copy`] description of the largest input the
//!   caller is willing to accept (max nodes, max depth, max bytes, max
//!   string length). Constructed once and shared by reference.
//! - [`Budget`] — a mutable, single-traversal counter seeded from a
//!   [`Limits`]. Decoders, validators, lowerers, and walkers call
//!   [`Budget::tick`] / [`Budget::charge`] / [`Budget::descend`] on every
//!   unit of work. The first violation returns [`BudgetError`].
//!
//! The split exists because limits are policy (the *what*) while budgets
//! are bookkeeping (the *how much remains*). One [`Limits`] can spawn many
//! independent [`Budget`]s; a [`Budget`] never outlives a single
//! traversal.
//!
//! ## Presets
//!
//! Three [`const fn`] presets cover the common deployment shapes:
//!
//! - [`Limits::iot`] — tight ceilings sized for `thumbv7em` / `riscv32imac`
//!   class devices (a few hundred nodes, a few KiB).
//! - [`Limits::host`] — generous ceilings for desktop / server use
//!   (millions of nodes, tens of MiB).
//! - [`Limits::fuzz`] — extremely tight ceilings for fuzz harnesses; chosen
//!   so that a single iteration still terminates within the libfuzzer
//!   per-run budget even for adversarial inputs.
//!
//! ## `no_std`
//!
//! Both types are `Copy`, contain only `usize` / `u32` fields, and are
//! `no_std + alloc`-clean. They never allocate.

use core::fmt;

/// Static caps on the largest input a producer is willing to process.
///
/// `Limits` is a passive description. The active accounting lives in
/// [`Budget`], which seeds itself from a `Limits`.
///
/// All fields are `usize` so they compose with allocation sizes and
/// container lengths without conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Limits {
    /// Maximum number of distinct nodes (AST, IR, wire frames) that may
    /// be materialised in a single traversal.
    pub max_nodes: usize,

    /// Maximum nesting depth (e.g. parser recursion, IR tree height).
    /// Guards against stack-blowup attacks on recursive validators.
    pub max_depth: u32,

    /// Maximum cumulative bytes a traversal is allowed to allocate or
    /// deserialise.
    pub max_bytes: usize,

    /// Maximum length, in bytes, of a single string / identifier.
    pub max_str_bytes: usize,
}

impl Limits {
    /// Tight preset for embedded MCU-class targets (`thumbv7em`,
    /// `riscv32imac`).
    #[must_use]
    pub const fn iot() -> Self {
        Self {
            max_nodes: 1_024,
            max_depth: 16,
            max_bytes: 16 * 1024,
            max_str_bytes: 256,
        }
    }

    /// Generous preset for desktop / server hosts.
    #[must_use]
    pub const fn host() -> Self {
        Self {
            max_nodes: 8_000_000,
            max_depth: 1_024,
            max_bytes: 64 * 1024 * 1024,
            max_str_bytes: 1024 * 1024,
        }
    }

    /// Extra-tight preset for fuzz harnesses; chosen so a single
    /// libfuzzer iteration always terminates promptly.
    #[must_use]
    pub const fn fuzz() -> Self {
        Self {
            max_nodes: 256,
            max_depth: 8,
            max_bytes: 4 * 1024,
            max_str_bytes: 128,
        }
    }

    /// Effectively-unbounded preset: every cap is set to the maximum
    /// representable value on the field's type (`usize::MAX` /
    /// `u32::MAX`).
    ///
    /// Used by entry points that explicitly opt out of bounding (e.g.
    /// the unbounded `lower_expr` in `dol-expr`) so the same `Budget`
    /// machinery can drive both the bounded and unbounded paths without
    /// branching on `Option<&mut Budget>`. Production code that handles
    /// untrusted input should use [`Self::iot`] or [`Self::host`]
    /// instead.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            max_nodes: usize::MAX,
            max_depth: u32::MAX,
            max_bytes: usize::MAX,
            max_str_bytes: usize::MAX,
        }
    }
}

impl Default for Limits {
    /// Same as [`Limits::host`].
    fn default() -> Self {
        Self::host()
    }
}

/// Reason a [`Budget`] refused further work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum BudgetError {
    /// `max_nodes` exceeded.
    Nodes,
    /// `max_depth` exceeded.
    Depth,
    /// `max_bytes` exceeded.
    Bytes,
    /// `max_str_bytes` exceeded.
    StrBytes,
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nodes => f.write_str("node budget exhausted"),
            Self::Depth => f.write_str("depth budget exhausted"),
            Self::Bytes => f.write_str("byte budget exhausted"),
            Self::StrBytes => f.write_str("string-byte budget exhausted"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BudgetError {}

/// Mutable per-traversal accounting derived from a [`Limits`].
///
/// A `Budget` is created once at the entry point of a traversal (decode,
/// validate, lower, fold, …) and mutated as work happens. Every recursive
/// or fanout-prone callee should accept `&mut Budget` and call the
/// matching debit method exactly once per unit of work.
///
/// Methods return `Result<(), BudgetError>` rather than panicking; the
/// caller is expected to propagate the error and convert it to whatever
/// diagnostic / wire error its layer uses.
#[derive(Clone, Debug)]
pub struct Budget {
    limits: Limits,
    nodes: usize,
    depth: u32,
    bytes: usize,
}

impl Budget {
    /// Create a fresh budget that will refuse work beyond `limits`.
    #[must_use]
    pub const fn new(limits: Limits) -> Self {
        Self {
            limits,
            nodes: 0,
            depth: 0,
            bytes: 0,
        }
    }

    /// Borrow the underlying [`Limits`].
    #[inline]
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Charge `n` nodes against the node budget.
    ///
    /// Use `tick(1)` for the common one-node-per-call case.
    #[inline]
    pub fn tick(&mut self, n: usize) -> Result<(), BudgetError> {
        let next = self.nodes.saturating_add(n);
        if next > self.limits.max_nodes {
            Err(BudgetError::Nodes)
        } else {
            self.nodes = next;
            Ok(())
        }
    }

    /// Charge `bytes` bytes against the byte budget.
    #[inline]
    pub fn charge(&mut self, bytes: usize) -> Result<(), BudgetError> {
        let next = self.bytes.saturating_add(bytes);
        if next > self.limits.max_bytes {
            Err(BudgetError::Bytes)
        } else {
            self.bytes = next;
            Ok(())
        }
    }

    /// Run `f` at one level of additional depth.
    ///
    /// Increments the depth counter, invokes `f`, then decrements. Returns
    /// [`BudgetError::Depth`] without invoking `f` if the new depth
    /// would exceed `max_depth`.
    ///
    /// The closure form is necessary because `Budget` must remain
    /// mutably accessible inside the descent (so the body can call
    /// `tick`, `charge`, or another `descend`); an RAII guard holding
    /// `&mut Budget` would block all of those.
    ///
    /// `Budget` is a per-traversal object; if `f` panics the budget is
    /// discarded by the caller, so leaving the depth counter
    /// incremented on the unwinding path is harmless and avoids the
    /// borrow-checker gymnastics a panic-safe guard would require.
    ///
    /// ```
    /// use dol_core::policy::{Budget, Limits};
    ///
    /// let mut b = Budget::new(Limits::iot());
    /// b.descend(|b| {
    ///     b.tick(1).unwrap();
    ///     b.descend(|b| {
    ///         assert_eq!(b.depth(), 2);
    ///     })
    ///     .unwrap();
    /// })
    /// .unwrap();
    /// assert_eq!(b.depth(), 0);
    /// ```
    #[inline]
    pub fn descend<F, R>(&mut self, f: F) -> Result<R, BudgetError>
    where
        F: FnOnce(&mut Self) -> R,
    {
        let next = self.depth.saturating_add(1);
        if next > self.limits.max_depth {
            return Err(BudgetError::Depth);
        }
        self.depth = next;
        let r = f(self);
        self.depth = self.depth.saturating_sub(1);
        Ok(r)
    }

    /// Validate that a string of `len` bytes fits the per-string cap.
    ///
    /// Does not mutate the byte counter; pair with [`Budget::charge`] if
    /// the string is also being held.
    #[inline]
    pub fn check_str(&self, len: usize) -> Result<(), BudgetError> {
        if len > self.limits.max_str_bytes {
            Err(BudgetError::StrBytes)
        } else {
            Ok(())
        }
    }

    /// Number of nodes charged so far.
    #[inline]
    #[must_use]
    pub const fn nodes(&self) -> usize {
        self.nodes
    }

    /// Number of bytes charged so far.
    #[inline]
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// Current depth (0 at the root, before any [`Budget::descend`]).
    #[inline]
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth
    }
}

/// Cumulative resource quota that spans many independent traversals.
///
/// [`Limits`] caps a *single* traversal; [`Quota`] caps the total work a
/// caller (tenant, session, request batch) is allowed across the lifetime
/// of the quota object. Common pattern:
///
/// ```text
/// // Once per tenant.
/// let mut quota = Quota::new(QuotaCaps {
///     max_total_nodes: 1_000_000,
///     max_total_bytes: 100 * 1024 * 1024,
/// });
///
/// // Per request:
/// let mut budget = Budget::new(Limits::host());
/// run_traversal(&mut budget)?;     // mutates `budget`
/// quota.consume(&budget)?;          // rolls budget into quota
/// ```
///
/// `Quota::consume` performs saturating addition of the per-traversal
/// counters into per-quota totals and returns [`BudgetError`] if any
/// total would exceed its cap. The quota is otherwise free of policy
/// — it does not enforce per-traversal `Limits`; that remains the
/// `Budget`'s job.
///
/// `Quota` is `Copy + 'static`-clean (`Clone + Debug`), `no_std`, and
/// never allocates. It is **not** `Sync`-protected: callers wanting
/// shared-mutable access wrap it in `Mutex` / `parking_lot::Mutex`.
#[derive(Clone, Debug)]
pub struct Quota {
    caps: QuotaCaps,
    total_nodes: usize,
    total_bytes: usize,
}

/// Static caps for a [`Quota`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct QuotaCaps {
    /// Total nodes allowed across the lifetime of the quota.
    pub max_total_nodes: usize,
    /// Total bytes allowed across the lifetime of the quota.
    pub max_total_bytes: usize,
}

impl QuotaCaps {
    /// Effectively-unbounded caps; both fields set to `usize::MAX`.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            max_total_nodes: usize::MAX,
            max_total_bytes: usize::MAX,
        }
    }
}

impl Default for QuotaCaps {
    fn default() -> Self {
        Self::unbounded()
    }
}

impl Quota {
    /// Create a fresh quota with the supplied caps.
    #[must_use]
    pub const fn new(caps: QuotaCaps) -> Self {
        Self {
            caps,
            total_nodes: 0,
            total_bytes: 0,
        }
    }

    /// Borrow the underlying [`QuotaCaps`].
    #[inline]
    #[must_use]
    pub const fn caps(&self) -> &QuotaCaps {
        &self.caps
    }

    /// Roll a finished traversal's [`Budget`] into the quota.
    ///
    /// On success the quota's totals are advanced. On error the totals
    /// are unchanged and the offending [`BudgetError`] is returned —
    /// the caller should treat the traversal as accounted-for elsewhere
    /// (e.g. surface a 429 / quota-exceeded diagnostic).
    pub fn consume(&mut self, budget: &Budget) -> Result<(), BudgetError> {
        let nodes = self.total_nodes.saturating_add(budget.nodes());
        let bytes = self.total_bytes.saturating_add(budget.bytes());
        if nodes > self.caps.max_total_nodes {
            return Err(BudgetError::Nodes);
        }
        if bytes > self.caps.max_total_bytes {
            return Err(BudgetError::Bytes);
        }
        self.total_nodes = nodes;
        self.total_bytes = bytes;
        Ok(())
    }

    /// Cumulative nodes consumed.
    #[inline]
    #[must_use]
    pub const fn total_nodes(&self) -> usize {
        self.total_nodes
    }

    /// Cumulative bytes consumed.
    #[inline]
    #[must_use]
    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_distinct_and_consistent() {
        // iot < fuzz check is intentionally NOT made: fuzz is even
        // tighter than iot. Just verify host >= iot >= fuzz on every
        // axis.
        let host = Limits::host();
        let iot = Limits::iot();
        let fuzz = Limits::fuzz();

        assert!(host.max_nodes > iot.max_nodes);
        assert!(host.max_depth > iot.max_depth);
        assert!(host.max_bytes > iot.max_bytes);
        assert!(host.max_str_bytes > iot.max_str_bytes);

        assert!(iot.max_nodes >= fuzz.max_nodes);
        assert!(iot.max_depth >= fuzz.max_depth);
        assert!(iot.max_bytes >= fuzz.max_bytes);
        assert!(iot.max_str_bytes >= fuzz.max_str_bytes);
    }

    #[test]
    fn tick_charges_nodes_and_refuses_overflow() {
        let mut b = Budget::new(Limits {
            max_nodes: 3,
            max_depth: 4,
            max_bytes: 100,
            max_str_bytes: 16,
        });
        assert!(b.tick(1).is_ok());
        assert!(b.tick(2).is_ok());
        assert_eq!(b.nodes(), 3);
        assert_eq!(b.tick(1), Err(BudgetError::Nodes));
        // Failed tick must not mutate the counter.
        assert_eq!(b.nodes(), 3);
    }

    #[test]
    fn charge_tracks_bytes() {
        let mut b = Budget::new(Limits {
            max_nodes: 999,
            max_depth: 999,
            max_bytes: 10,
            max_str_bytes: 8,
        });
        assert!(b.charge(7).is_ok());
        assert_eq!(b.bytes(), 7);
        assert_eq!(b.charge(4), Err(BudgetError::Bytes));
        assert_eq!(b.bytes(), 7);
    }

    #[test]
    fn descend_increments_and_decrements() {
        let mut b = Budget::new(Limits::iot());
        b.descend(|b| {
            assert_eq!(b.depth(), 1);
            b.descend(|b| {
                assert_eq!(b.depth(), 2);
            })
            .unwrap();
            assert_eq!(b.depth(), 1);
        })
        .unwrap();
        assert_eq!(b.depth(), 0);
    }

    #[test]
    fn descend_refuses_over_max_depth() {
        let mut b = Budget::new(Limits {
            max_nodes: 999,
            max_depth: 2,
            max_bytes: 999,
            max_str_bytes: 999,
        });
        b.descend(|b| {
            b.descend(|b| {
                assert!(matches!(b.descend(|_| ()), Err(BudgetError::Depth)));
                assert_eq!(b.depth(), 2);
            })
            .unwrap();
        })
        .unwrap();
        assert_eq!(b.depth(), 0);
    }

    #[test]
    fn descend_propagates_inner_value() {
        let mut b = Budget::new(Limits::iot());
        let r = b.descend(|_| 1234_u32).unwrap();
        assert_eq!(r, 1234);
    }

    #[test]
    fn check_str_enforces_per_string_cap() {
        let b = Budget::new(Limits::iot());
        assert!(b.check_str(0).is_ok());
        assert!(b.check_str(Limits::iot().max_str_bytes).is_ok());
        assert_eq!(
            b.check_str(Limits::iot().max_str_bytes + 1),
            Err(BudgetError::StrBytes)
        );
    }

    #[test]
    fn unbounded_preset_admits_extreme_inputs() {
        let mut b = Budget::new(Limits::unbounded());
        // Anything short of `usize::MAX` must succeed; saturating_add
        // means even the max value is rejected (would overflow), which
        // is fine because no realistic traversal hits that.
        assert!(b.tick(1_000_000).is_ok());
        assert!(b.charge(1_000_000).is_ok());
        b.descend(|b| {
            assert_eq!(b.depth(), 1);
        })
        .unwrap();
    }

    #[test]
    fn saturating_add_does_not_panic_on_extreme_inputs() {
        let mut b = Budget::new(Limits::host());
        // Even with `usize::MAX`, the call must return a clean error
        // rather than panic on overflow.
        assert_eq!(b.tick(usize::MAX), Err(BudgetError::Nodes));
        assert_eq!(b.charge(usize::MAX), Err(BudgetError::Bytes));
    }

    #[test]
    fn quota_consumes_finished_budgets_until_capped() {
        let mut quota = Quota::new(QuotaCaps {
            max_total_nodes: 10,
            max_total_bytes: 100,
        });
        let mut b1 = Budget::new(Limits::host());
        b1.tick(4).unwrap();
        b1.charge(40).unwrap();
        quota.consume(&b1).unwrap();
        assert_eq!(quota.total_nodes(), 4);
        assert_eq!(quota.total_bytes(), 40);

        let mut b2 = Budget::new(Limits::host());
        b2.tick(5).unwrap();
        b2.charge(50).unwrap();
        quota.consume(&b2).unwrap();
        assert_eq!(quota.total_nodes(), 9);
        assert_eq!(quota.total_bytes(), 90);

        // Third traversal pushes us over the node cap; quota state
        // must be unchanged on rejection.
        let mut b3 = Budget::new(Limits::host());
        b3.tick(2).unwrap();
        b3.charge(5).unwrap();
        assert_eq!(quota.consume(&b3), Err(BudgetError::Nodes));
        assert_eq!(quota.total_nodes(), 9);
        assert_eq!(quota.total_bytes(), 90);
    }

    #[test]
    fn quota_caps_unbounded_never_refuses() {
        let mut quota = Quota::new(QuotaCaps::unbounded());
        let mut b = Budget::new(Limits::host());
        b.tick(1_000_000).unwrap();
        b.charge(1_000_000).unwrap();
        quota.consume(&b).unwrap();
        assert_eq!(quota.total_nodes(), 1_000_000);
        assert_eq!(quota.total_bytes(), 1_000_000);
    }
}

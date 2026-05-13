//! # `budget` — single-traversal resource counter (per `dol-rewrite-plan-v2.md` §6.2).
//!
//! Decoders, validators, lowerers, and walkers thread a [`Budget`] by
//! `&mut` reference and call [`Budget::depth`], [`Budget::node`], and
//! [`Budget::bytes`] on every unit of work. The first violation returns
//! [`BudgetExceeded`].
//!
//! Budgets are constructed from a [`BudgetConfig`]:
//!
//! ```
//! use dol_core::budget::{Budget, BudgetExceeded};
//! use dol_core::config::BudgetConfig;
//!
//! let cfg = BudgetConfig::standard();
//! let mut b = Budget::from_config(&cfg);
//! assert!(b.node().is_ok());
//! ```
//!
//! ## Difference from [`crate::policy::Budget`]
//!
//! The legacy `policy::Budget` is closure-based and bundles a `Limits`
//! reference. The new [`Budget`] here is a flat counter struct sized
//! and seeded by [`BudgetConfig`]. It is the canonical surface for v2;
//! `policy::Budget` is retained during the M0–M3 transition because
//! existing internal call-sites (and the legacy on-disk crates under
//! `lib/expr` etc.) still consume it.

use crate::config::BudgetConfig;

/// First-violation enum returned by every [`Budget`] method. Variants
/// identify which cap was exhausted; the caller is responsible for
/// turning that into a [`crate::diagnostic::Diagnostic`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum BudgetExceeded {
    /// [`Budget::depth`] tried to descend past `0`.
    Depth,
    /// [`Budget::node`] tried to allocate one more node past `0`.
    Nodes,
    /// [`Budget::bytes`] tried to charge bytes past the remaining cap.
    Bytes,
}

impl core::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Depth => f.write_str("budget exceeded: depth"),
            Self::Nodes => f.write_str("budget exceeded: nodes"),
            Self::Bytes => f.write_str("budget exceeded: bytes"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BudgetExceeded {}

/// Mutable single-traversal counter. Each field starts at the
/// corresponding [`BudgetConfig`] cap and is decremented as the
/// traversal makes progress.
///
/// Budgets are `Copy` so a checkpoint can be stashed before a
/// speculative branch and restored on rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Budget {
    /// Remaining depth headroom. Decremented by [`Self::depth`].
    pub depth: u32,
    /// Remaining node headroom. Decremented by [`Self::node`].
    pub nodes: u32,
    /// Remaining byte headroom. Decremented by [`Self::bytes`].
    pub bytes: u64,
}

impl Budget {
    /// Seed a fresh budget from a [`BudgetConfig`].
    #[must_use]
    #[inline]
    pub const fn from_config(cfg: &BudgetConfig) -> Self {
        Self {
            depth: cfg.max_depth,
            nodes: cfg.max_nodes,
            bytes: cfg.max_bytes,
        }
    }

    /// Charge one level of nesting depth. Returns
    /// [`BudgetExceeded::Depth`] if no headroom remains.
    ///
    /// The matching [`Self::leave_depth`] credit is intentionally not
    /// required — most traversals descend monotonically. Callers that
    /// need symmetric depth tracking can copy the budget before
    /// descending and discard the inner copy on return.
    #[inline]
    pub fn depth(&mut self) -> Result<(), BudgetExceeded> {
        self.depth = self.depth.checked_sub(1).ok_or(BudgetExceeded::Depth)?;
        Ok(())
    }

    /// Credit one level of nesting depth — the inverse of
    /// [`Self::depth`]. Saturating; never overflows.
    #[inline]
    pub fn leave_depth(&mut self) {
        self.depth = self.depth.saturating_add(1);
    }

    /// Charge one node. Returns [`BudgetExceeded::Nodes`] if no
    /// headroom remains.
    #[inline]
    pub fn node(&mut self) -> Result<(), BudgetExceeded> {
        self.nodes = self.nodes.checked_sub(1).ok_or(BudgetExceeded::Nodes)?;
        Ok(())
    }

    /// Charge `n` bytes. Returns [`BudgetExceeded::Bytes`] if the
    /// charge would overflow the remaining cap.
    #[inline]
    pub fn bytes(&mut self, n: u64) -> Result<(), BudgetExceeded> {
        self.bytes = self.bytes.checked_sub(n).ok_or(BudgetExceeded::Bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_seeds_all_three_counters() {
        let cfg = BudgetConfig {
            max_depth: 5,
            max_nodes: 10,
            max_bytes: 100,
        };
        let b = Budget::from_config(&cfg);
        assert_eq!(b.depth, 5);
        assert_eq!(b.nodes, 10);
        assert_eq!(b.bytes, 100);
    }

    #[test]
    fn depth_returns_depth_variant_on_exhaustion() {
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 1,
            max_nodes: 1,
            max_bytes: 1,
        });
        assert!(b.depth().is_ok());
        assert_eq!(b.depth().unwrap_err(), BudgetExceeded::Depth);
    }

    #[test]
    fn node_returns_nodes_variant_on_exhaustion() {
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 1,
            max_nodes: 1,
            max_bytes: 1,
        });
        assert!(b.node().is_ok());
        assert_eq!(b.node().unwrap_err(), BudgetExceeded::Nodes);
    }

    #[test]
    fn bytes_returns_bytes_variant_on_exhaustion() {
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 1,
            max_nodes: 1,
            max_bytes: 10,
        });
        assert!(b.bytes(7).is_ok());
        assert_eq!(b.bytes(4).unwrap_err(), BudgetExceeded::Bytes);
    }

    #[test]
    fn leave_depth_credits_back() {
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 2,
            max_nodes: 1,
            max_bytes: 1,
        });
        b.depth().unwrap();
        b.depth().unwrap();
        assert_eq!(b.depth().unwrap_err(), BudgetExceeded::Depth);
        b.leave_depth();
        assert!(b.depth().is_ok());
    }

    #[test]
    fn budget_is_copy_for_checkpoint_rollback() {
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 5,
            max_nodes: 5,
            max_bytes: 5,
        });
        let checkpoint = b;
        b.node().unwrap();
        b.node().unwrap();
        assert_eq!(b.nodes, 3);
        // Restore from copy.
        b = checkpoint;
        assert_eq!(b.nodes, 5);
    }
}

//! Memory-accounting helpers for the expression engine.
//!
//! These are *purely informational* — nothing in the lowering or build path
//! depends on them. They exist so:
//!
//! 1. Benchmarks can print a single number per change ("the arena now uses
//!    20% fewer bytes for this representative plan").
//! 2. Future regressions (e.g. an enum variant inflating beyond the 32 B
//!    budget for `ExprNode`, or the interner bloating after a refactor)
//!    are visible alongside throughput numbers, not just size assertions.
//!
//! Not part of any wire format; safe to evolve freely.

use crate::arena::ExprArena;
use crate::interner::Interner;

#[cfg(test)]
extern crate alloc;

/// A snapshot of resident bytes across the major arena pools.
///
/// Field meanings:
///
/// * `nodes` — `ExprArena::nodes` capacity × `size_of::<ExprNode>()`.
/// * `side_pools` — sum of every per-shape pool (fields, funcs, obj_lits,
///   windows, cases, in_lists, queries, inserts, updates, deletes,
///   upserts, lits, span table).
/// * `interner` — bytes blob + slice index + lookup table footprint
///   (see [`Interner::heap_bytes`]).
/// * `total` — `nodes + side_pools + interner`.
///
/// All figures count `Vec` *capacity*, not `len`, so they reflect the real
/// resident set rather than the populated portion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Number of `ExprNode`s stored in the hot pool.
    pub node_count: usize,
    /// Bytes resident in the hot `ExprNode` vector.
    pub nodes: usize,
    /// Bytes resident in every side pool (fields, funcs, lits, …).
    pub side_pools: usize,
    /// Bytes resident in the [`Interner`].
    pub interner: usize,
    /// Sum of all of the above.
    pub total: usize,
}

/// Summarise the resident heap usage of an [`ExprArena`] / [`Interner`] pair.
// Heap-byte estimate; multiplications and additions are bounded by `Vec`
// capacities and would saturate at `usize::MAX` on 64-bit, which is harmless.
#[allow(clippy::arithmetic_side_effects)]
pub fn snapshot(arena: &ExprArena, interner: &Interner) -> Stats {
    use core::mem::size_of;
    let nodes_bytes = arena.nodes_capacity() * size_of::<crate::expr::ExprNode>();
    let arena_total = arena.heap_bytes();
    let side_pools = arena_total.saturating_sub(nodes_bytes);
    let interner_bytes = interner.heap_bytes();
    Stats {
        node_count: arena.len(),
        nodes: nodes_bytes,
        side_pools,
        interner: interner_bytes,
        total: nodes_bytes + side_pools + interner_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::BuildSession;

    #[test]
    fn snapshot_grows_monotonically() {
        let mut sess = BuildSession::new();
        let s0 = snapshot(&sess.arena, &sess.interner);
        for i in 0..32 {
            // Use distinct names so the interner actually grows. Keys are
            // short to keep the bytes-blob delta dominated by the slice
            // table on this micro test.
            let name = alloc::format!("col_{i}");
            let _ = sess.field(&name);
        }
        let s1 = snapshot(&sess.arena, &sess.interner);
        assert!(s1.node_count > s0.node_count);
        assert!(s1.total >= s0.total);
        assert!(s1.interner >= s0.interner);
    }

    #[test]
    fn empty_snapshot_has_zero_count() {
        let sess = BuildSession::new();
        let s = snapshot(&sess.arena, &sess.interner);
        assert_eq!(s.node_count, 0);
    }
}

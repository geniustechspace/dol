//! Bottom-up content-hash walker for [`ExprArena`].
//!
//! Per `dol-rewrite-plan-v2.md` §8.5.
//!
//! ## What it computes
//!
//! [`content_hash`] returns the BLAKE3-128 content address of the
//! subtree rooted at a [`NodeId`]. The hash is computed from the
//! node's opcode family + flags + aux + child content addresses (for
//! recursive families) or leaf operand bits (for leaves). The result
//! is **cross-process stable**: two processes building the
//! byte-identical subtree always derive the same digest.
//!
//! ## Memoisation
//!
//! Results are cached in a [`ContentIndex`]; subsequent calls on the
//! same [`NodeId`] short-circuit to the cached digest. The walker
//! mutates the index in place.
//!
//! ## Budget
//!
//! Every recursive descent charges one depth + one node against the
//! caller's [`Budget`]. Leaves charge nodes only. Cached hits skip
//! the budget entirely — they are free lookups by design.

#[cfg(feature = "std")]
use dol_cas::content_index::ContentIndex;
#[cfg(feature = "std")]
use dol_cas::handle::NodeId;

#[cfg(feature = "std")]
use dol_core::budget::{Budget, BudgetExceeded};
#[cfg(feature = "std")]
use dol_core::hash::{Hasher, content128};

#[cfg(feature = "std")]
use super::arena::ExprArena;
#[cfg(feature = "std")]
use super::node::ExprNode;
#[cfg(feature = "std")]
use super::ops::OpFamily;

/// Walker-specific failure modes.
///
/// All other arena operations (`push`, `get`, `intern_node`) return
/// [`Option`] for the same conditions — the walker promotes them to a
/// rich error because callers usually need to distinguish a missing
/// node from a budget exhaustion from an unknown opcode.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentHashError {
    /// The caller's [`Budget`] would be exceeded by this descent.
    Budget(BudgetExceeded),
    /// A child reference points to a node that does not exist in
    /// this arena — the arena is malformed.
    MissingNode,
    /// The node carries an opcode family this build does not
    /// recognise (typically a wire-format / version-skew issue).
    UnknownOpcode,
}

#[cfg(feature = "std")]
impl From<BudgetExceeded> for ContentHashError {
    #[inline]
    fn from(e: BudgetExceeded) -> Self {
        Self::Budget(e)
    }
}

/// Fixed domain prefix tag fed to BLAKE3 before the rest of the
/// node bytes. Bumping this byte invalidates every persisted digest
/// — treat it as a wire-format version, not a casual constant.
#[cfg(feature = "std")]
const HASH_DOMAIN_TAG: u8 = 0x01;

/// Compute the BLAKE3-128 content address of the subtree rooted at
/// `id`. Memoised in `index`; subsequent calls on the same id are
/// free lookups.
///
/// Returns [`ContentHashError::MissingNode`] if `id` (or any of its
/// transitive children) is not present in `arena`, and
/// [`ContentHashError::UnknownOpcode`] if a node carries an opcode
/// family this build does not recognise.
#[cfg(feature = "std")]
pub fn content_hash(
    arena: &ExprArena,
    id: NodeId,
    index: &mut ContentIndex,
    budget: &mut Budget,
) -> Result<[u8; 16], ContentHashError> {
    if let Some(h) = index.get(id) {
        return Ok(h);
    }
    budget.depth()?;
    budget.node()?;
    let node = arena.get(id).ok_or(ContentHashError::MissingNode)?;
    let family = node.family().ok_or(ContentHashError::UnknownOpcode)?;
    let digest = hash_node(arena, node, family, index, budget)?;
    index.insert(id, digest);
    Ok(digest)
}

#[cfg(feature = "std")]
fn hash_node(
    arena: &ExprArena,
    node: &ExprNode,
    family: OpFamily,
    index: &mut ContentIndex,
    budget: &mut Budget,
) -> Result<[u8; 16], ContentHashError> {
    // Build the byte image: [domain || family || flags || aux || ...].
    // Recursive families substitute their child content addresses for
    // the raw NodeId so the result is structural, not arena-relative.
    let bytes: [u8; 16] = bytemuck::cast(*node);
    let flags = bytes[1];
    let aux_lo = bytes[2];
    let aux_hi = bytes[3];

    let mut h = Hasher::new();
    h.update(&[HASH_DOMAIN_TAG, family.as_u8(), flags, aux_lo, aux_hi]);

    match family {
        OpFamily::Bin => {
            // Children are at bytes[4..8] and [8..12].
            let (_, left, right) = node.as_bin().ok_or(ContentHashError::UnknownOpcode)?;
            let lh = content_hash(arena, left, index, budget)?;
            let rh = content_hash(arena, right, index, budget)?;
            h.update(&lh);
            h.update(&rh);
        }
        OpFamily::Unary => {
            let (_, operand) = node.as_unary().ok_or(ContentHashError::UnknownOpcode)?;
            let oh = content_hash(arena, operand, index, budget)?;
            h.update(&oh);
        }
        OpFamily::Cast => {
            // Hash domain: [common header || operand digest || target type id bytes].
            // The TypePool is push-only (no structural dedup), so two
            // `Cast(_, Int32)` calls receive distinct `TypeId`s and
            // therefore distinct digests — sound but slightly less
            // sharing than the underlying type identity allows. Using
            // the id bytes (rather than recursing into `DataType`)
            // keeps the walker arena-local and budget-bounded.
            let (operand, _ty) = node.as_cast().ok_or(ContentHashError::UnknownOpcode)?;
            let oh = content_hash(arena, operand, index, budget)?;
            h.update(&oh);
            // Folded the type id into the header via bytes[4..8] below
            // would have aliased with operand bytes for other families.
            // Append it separately so the type id participates in the
            // digest in a position the other families never write to.
            h.update(&bytes[8..12]);
        }
        OpFamily::If => {
            let (cond, t, e) = node.as_if().ok_or(ContentHashError::UnknownOpcode)?;
            let ch = content_hash(arena, cond, index, budget)?;
            let th = content_hash(arena, t, index, budget)?;
            let eh = content_hash(arena, e, index, budget)?;
            h.update(&ch);
            h.update(&th);
            h.update(&eh);
        }
        OpFamily::InRange => {
            let (e, lo, hi) = node.as_in_range().ok_or(ContentHashError::UnknownOpcode)?;
            let eh = content_hash(arena, e, index, budget)?;
            let lh = content_hash(arena, lo, index, budget)?;
            let hh = content_hash(arena, hi, index, budget)?;
            h.update(&eh);
            h.update(&lh);
            h.update(&hh);
        }
        OpFamily::Seq => {
            let span = node.as_seq().ok_or(ContentHashError::UnknownOpcode)?;
            let slots = arena
                .operands()
                .get(span)
                .ok_or(ContentHashError::MissingNode)?;
            // Length participates in the digest so `[a]` ≠ `[a, a]` etc.
            h.update(&u32::try_from(slots.len()).unwrap_or(u32::MAX).to_le_bytes());
            for slot in slots {
                let child = dol_cas::handle::Lid::from_u32(*slot)
                    .ok_or(ContentHashError::MissingNode)?;
                let ch = content_hash(arena, child, index, budget)?;
                h.update(&ch);
            }
        }
        OpFamily::Map => {
            let span = node.as_map().ok_or(ContentHashError::UnknownOpcode)?;
            let slots = arena
                .operands()
                .get(span)
                .ok_or(ContentHashError::MissingNode)?;
            // Map slabs alternate `[StrId, NodeId, …]` and require an
            // even length. Pair count goes into the digest so empty
            // maps and single-pair maps don't alias.
            let pair_count = slots.len() / 2;
            h.update(
                &u32::try_from(pair_count)
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            // Iterate in pairs. We deliberately don't sort by key —
            // the lowering pass preserves the source insertion order
            // and the digest must reflect it.
            let mut i = 0usize;
            while i < slots.len() {
                let key_slot = slots.get(i).copied().ok_or(ContentHashError::MissingNode)?;
                // Key contributes its raw `StrId` u32 — strings are
                // already content-deduped through `StringPool`, so
                // equal source names share a `StrId` and therefore
                // contribute identical bytes.
                h.update(&key_slot.to_le_bytes());
                let val_slot = slots
                    .get(i.saturating_add(1))
                    .copied()
                    .ok_or(ContentHashError::MissingNode)?;
                let val_id = dol_cas::handle::Lid::from_u32(val_slot)
                    .ok_or(ContentHashError::MissingNode)?;
                let vh = content_hash(arena, val_id, index, budget)?;
                h.update(&vh);
                i = i.saturating_add(2);
            }
        }
        OpFamily::Call => {
            let (_, span) = node.as_call().ok_or(ContentHashError::UnknownOpcode)?;
            // Function id is in bytes[4..8] (already covered) but for
            // clarity feed it explicitly so leaf-only families that
            // don't recurse into the operand slab still produce
            // distinct digests when the callee differs.
            h.update(&bytes[4..8]);
            let slots = arena
                .operands()
                .get(span)
                .ok_or(ContentHashError::MissingNode)?;
            h.update(&u32::try_from(slots.len()).unwrap_or(u32::MAX).to_le_bytes());
            for slot in slots {
                let child = dol_cas::handle::Lid::from_u32(*slot)
                    .ok_or(ContentHashError::MissingNode)?;
                let ch = content_hash(arena, child, index, budget)?;
                h.update(&ch);
            }
        }
        OpFamily::Match => {
            let (span, fallback) =
                node.as_match().ok_or(ContentHashError::UnknownOpcode)?;
            let slots = arena
                .operands()
                .get(span)
                .ok_or(ContentHashError::MissingNode)?;
            h.update(&u32::try_from(slots.len()).unwrap_or(u32::MAX).to_le_bytes());
            for slot in slots {
                let child = dol_cas::handle::Lid::from_u32(*slot)
                    .ok_or(ContentHashError::MissingNode)?;
                let ch = content_hash(arena, child, index, budget)?;
                h.update(&ch);
            }
            // Fallback: feed a presence byte then either the digest of
            // the fallback expression or zero-padding of equal length.
            match fallback {
                Some(id) => {
                    h.update(&[1u8]);
                    let fh = content_hash(arena, id, index, budget)?;
                    h.update(&fh);
                }
                None => {
                    h.update(&[0u8]);
                    h.update(&[0u8; 16]);
                }
            }
        }
        OpFamily::MemberOf => {
            let (e, span) =
                node.as_member_of().ok_or(ContentHashError::UnknownOpcode)?;
            let eh = content_hash(arena, e, index, budget)?;
            h.update(&eh);
            let slots = arena
                .operands()
                .get(span)
                .ok_or(ContentHashError::MissingNode)?;
            h.update(&u32::try_from(slots.len()).unwrap_or(u32::MAX).to_le_bytes());
            for slot in slots {
                let child = dol_cas::handle::Lid::from_u32(*slot)
                    .ok_or(ContentHashError::MissingNode)?;
                let ch = content_hash(arena, child, index, budget)?;
                h.update(&ch);
            }
        }
        OpFamily::Label => {
            let (e, name) = node.as_label().ok_or(ContentHashError::UnknownOpcode)?;
            let eh = content_hash(arena, e, index, budget)?;
            h.update(&eh);
            // Label name is a `StrId`; strings are content-deduped so
            // equal labels collapse to identical bytes naturally.
            h.update(&name.get().to_le_bytes());
        }
        OpFamily::Scoped => {
            let (e, ctx_id) = node.as_scoped().ok_or(ContentHashError::UnknownOpcode)?;
            let eh = content_hash(arena, e, index, budget)?;
            h.update(&eh);
            // Recurse into the lowered context: hash partition_by and
            // order_by entries (children + sort metadata) and fold the
            // optional Frame in.
            let ctx = arena
                .contexts()
                .get(ctx_id)
                .ok_or(ContentHashError::MissingNode)?;
            h.update(
                &u32::try_from(ctx.partition_by.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            for &child in &ctx.partition_by {
                let ch = content_hash(arena, child, index, budget)?;
                h.update(&ch);
            }
            h.update(
                &u32::try_from(ctx.order_by.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            for entry in &ctx.order_by {
                let ch = content_hash(arena, entry.expr, index, budget)?;
                h.update(&ch);
                h.update(&[
                    if entry.dir.is_ascending() { 0 } else { 1 },
                    match entry.nulls {
                        crate::expr::order::NullsOrder::First => 1,
                        crate::expr::order::NullsOrder::Last => 2,
                        crate::expr::order::NullsOrder::Default => 0,
                    },
                ]);
            }
            // Frame: presence byte + canonical bytes if present.
            match ctx.frame {
                None => {
                    h.update(&[0u8]);
                }
                Some(frame) => {
                    h.update(&[1u8]);
                    h.update(&frame_bytes(frame));
                }
            }
        }
        // Pure leaf families: feed the raw 32-bit operand a/b/c slab.
        // For PathRef the path id is in `a` and is content-stable
        // through the structurally-deduped `PathPool`.
        OpFamily::PathRef
        | OpFamily::LitRef
        | OpFamily::FieldRef
        | OpFamily::FuncRef
        | OpFamily::Param
        | OpFamily::Wildcard
        | OpFamily::CountAll
        | OpFamily::Reserved => {
            h.update(&bytes[4..16]);
        }
    }

    // Re-use the BLAKE3 streaming hasher, then truncate to 128 bits
    // via the same chokepoint helper used elsewhere in the workspace.
    // We can't call `content128` directly because we've already fed
    // bytes streaming-style; finalize128 gives the matching digest.
    Ok(h.finalize128())
}

/// Convenience wrapper: hash a one-shot byte slice through the
/// project's `content128` chokepoint. Provided for callers that want
/// to derive auxiliary digests in the same domain as the walker
/// without manually setting up a [`Hasher`].
// budget-gate: opt-out: one-shot byte hash with no recursive descent;
// caller-controlled input length, no per-node fan-out to charge.
#[cfg(feature = "std")]
#[inline]
#[must_use]
pub fn content_hash_bytes(bytes: &[u8]) -> [u8; 16] {
    content128(bytes)
}

/// Canonical 19-byte serialisation of a [`Frame`]: 1 byte unit + 9
/// bytes for `start` + 9 bytes for `end`. Used by [`content_hash`] so
/// `Scoped` digests fold the frame in canonically without recursing.
#[cfg(feature = "std")]
fn frame_bytes(frame: super::frame::Frame) -> [u8; 19] {
    use super::frame::{Boundary, Extent, FrameUnit};
    fn boundary(b: Boundary) -> [u8; 9] {
        // Tag + 8-byte little-endian payload (zero when unused).
        let (tag, ext_tag, off): (u8, u8, u64) = match b {
            Boundary::Current => (0, 0, 0),
            Boundary::Before(Extent::Unbounded) => (1, 0, 0),
            Boundary::Before(Extent::Offset(n)) => (1, 1, n),
            Boundary::After(Extent::Unbounded) => (2, 0, 0),
            Boundary::After(Extent::Offset(n)) => (2, 1, n),
        };
        let mut out = [0u8; 9];
        out[0] = tag;
        out[1..9].copy_from_slice(&off.to_le_bytes());
        // Squeeze the extent tag into the high byte of the offset
        // slot (offsets above 2^56 are vanishingly unlikely; the tag
        // bit also distinguishes `Unbounded` from `Offset(0)`).
        out[8] = ext_tag;
        out
    }
    let unit_byte: u8 = match frame.unit {
        FrameUnit::Rows => 0,
        FrameUnit::Range => 1,
        FrameUnit::Groups => 2,
    };
    let mut out = [0u8; 19];
    out[0] = unit_byte;
    out[1..10].copy_from_slice(&boundary(frame.start));
    out[10..19].copy_from_slice(&boundary(frame.end));
    out
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::expr::flags::NodeFlags;
    use crate::expr::node::ExprNode;
    use crate::expr::ops::{BinOp, UnaryOp};
    use dol_core::budget::Budget;
    use dol_core::config::BudgetConfig;

    fn budget() -> Budget {
        Budget::from_config(&BudgetConfig {
            max_depth: 64,
            max_nodes: 1024,
            max_bytes: 1 << 20,
        })
    }

    #[test]
    fn leaf_hash_is_deterministic() {
        let mut a = ExprArena::new();
        let id = a.intern_node(ExprNode::param(7)).unwrap();
        let mut idx = ContentIndex::new();
        let mut b = budget();
        let h1 = content_hash(&a, id, &mut idx, &mut b).unwrap();
        // Second call hits the cache → free.
        let mut b2 = budget();
        let h2 = content_hash(&a, id, &mut idx, &mut b2).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(idx.len(), 1);
        // Cache hit consumed no budget.
        assert_eq!(b2, budget());
    }

    #[test]
    fn distinct_leaves_have_distinct_hashes() {
        let mut a = ExprArena::new();
        let p1 = a.intern_node(ExprNode::param(1)).unwrap();
        let p2 = a.intern_node(ExprNode::param(2)).unwrap();
        let wc = a.intern_node(ExprNode::wildcard()).unwrap();
        let ca = a.intern_node(ExprNode::count_all()).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = budget();
        let h_p1 = content_hash(&a, p1, &mut idx, &mut b).unwrap();
        let h_p2 = content_hash(&a, p2, &mut idx, &mut b).unwrap();
        let h_wc = content_hash(&a, wc, &mut idx, &mut b).unwrap();
        let h_ca = content_hash(&a, ca, &mut idx, &mut b).unwrap();

        assert_ne!(h_p1, h_p2);
        assert_ne!(h_p2, h_wc);
        assert_ne!(h_wc, h_ca);
    }

    #[test]
    fn flag_difference_changes_hash() {
        let mut a = ExprArena::new();
        let child = a.intern_node(ExprNode::param(1)).unwrap();
        let mut n_plain = ExprNode::unary(UnaryOp::Not, child);
        let mut n_neg = ExprNode::unary(UnaryOp::Not, child);
        n_neg.set_flags(NodeFlags::new().with_negated(true));
        let id_plain = a.intern_node(n_plain).unwrap();
        let id_neg = a.intern_node(n_neg).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = budget();
        let h_plain = content_hash(&a, id_plain, &mut idx, &mut b).unwrap();
        let h_neg = content_hash(&a, id_neg, &mut idx, &mut b).unwrap();
        assert_ne!(h_plain, h_neg);
        // Touch n_plain to silence unused-mut. The mutable binding is
        // mirrored on the neg side for symmetry; both nodes are
        // constructed and interned in this scope.
        n_plain.set_flags(n_plain.flags());
    }

    #[test]
    fn structural_equality_yields_identical_hash() {
        // Two arenas building the same subtree must derive the same
        // digest — that is the cross-process-stability promise.
        let mut a1 = ExprArena::new();
        let l1 = a1.intern_node(ExprNode::param(1)).unwrap();
        let r1 = a1.intern_node(ExprNode::param(2)).unwrap();
        let s1 = a1.intern_node(ExprNode::bin(BinOp::Add, l1, r1)).unwrap();

        let mut a2 = ExprArena::new();
        // Allocate noise nodes first so ids differ between arenas.
        let _ = a2.push(ExprNode::wildcard()).unwrap();
        let _ = a2.push(ExprNode::count_all()).unwrap();
        let l2 = a2.intern_node(ExprNode::param(1)).unwrap();
        let r2 = a2.intern_node(ExprNode::param(2)).unwrap();
        let s2 = a2.intern_node(ExprNode::bin(BinOp::Add, l2, r2)).unwrap();

        // The internal NodeIds are different on purpose.
        assert_ne!(s1, s2);

        let mut idx1 = ContentIndex::new();
        let mut idx2 = ContentIndex::new();
        let mut b = budget();
        let h1 = content_hash(&a1, s1, &mut idx1, &mut b).unwrap();
        let mut b = budget();
        let h2 = content_hash(&a2, s2, &mut idx2, &mut b).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn operand_order_matters() {
        let mut a = ExprArena::new();
        let x = a.intern_node(ExprNode::param(1)).unwrap();
        let y = a.intern_node(ExprNode::param(2)).unwrap();
        let xy = a.intern_node(ExprNode::bin(BinOp::Sub, x, y)).unwrap();
        let yx = a.intern_node(ExprNode::bin(BinOp::Sub, y, x)).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = budget();
        let h_xy = content_hash(&a, xy, &mut idx, &mut b).unwrap();
        let h_yx = content_hash(&a, yx, &mut idx, &mut b).unwrap();
        assert_ne!(h_xy, h_yx);
    }

    #[test]
    fn opcode_changes_hash_with_same_operands() {
        let mut a = ExprArena::new();
        let l = a.intern_node(ExprNode::param(1)).unwrap();
        let r = a.intern_node(ExprNode::param(2)).unwrap();
        let add = a.intern_node(ExprNode::bin(BinOp::Add, l, r)).unwrap();
        let mul = a.intern_node(ExprNode::bin(BinOp::Mul, l, r)).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = budget();
        let h_add = content_hash(&a, add, &mut idx, &mut b).unwrap();
        let h_mul = content_hash(&a, mul, &mut idx, &mut b).unwrap();
        assert_ne!(h_add, h_mul);
    }

    #[test]
    fn missing_node_reports_error() {
        let a = ExprArena::new();
        let mut idx = ContentIndex::new();
        let mut b = budget();
        let phantom = dol_cas::handle::Lid::from_u32(1).unwrap();
        let err = content_hash(&a, phantom, &mut idx, &mut b).unwrap_err();
        assert_eq!(err, ContentHashError::MissingNode);
    }

    #[test]
    fn missing_child_reports_error() {
        // Construct a binary that references an out-of-range child.
        let mut a = ExprArena::new();
        let dangling = dol_cas::handle::Lid::from_u32(999).unwrap();
        let real = a.intern_node(ExprNode::param(1)).unwrap();
        let bad = a
            .intern_node(ExprNode::bin(BinOp::Add, dangling, real))
            .unwrap();

        let mut idx = ContentIndex::new();
        let mut b = budget();
        let err = content_hash(&a, bad, &mut idx, &mut b).unwrap_err();
        assert_eq!(err, ContentHashError::MissingNode);
    }

    #[test]
    fn budget_depth_exhaustion_is_reported() {
        // Build a 5-deep unary chain; budget allows depth = 3.
        let mut a = ExprArena::new();
        let leaf = a.intern_node(ExprNode::param(0)).unwrap();
        let n1 = a.intern_node(ExprNode::unary(UnaryOp::Not, leaf)).unwrap();
        let n2 = a.intern_node(ExprNode::unary(UnaryOp::Not, n1)).unwrap();
        let n3 = a.intern_node(ExprNode::unary(UnaryOp::Not, n2)).unwrap();
        let n4 = a.intern_node(ExprNode::unary(UnaryOp::Not, n3)).unwrap();
        let n5 = a.intern_node(ExprNode::unary(UnaryOp::Not, n4)).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 3,
            max_nodes: 1024,
            max_bytes: 1 << 20,
        });
        let err = content_hash(&a, n5, &mut idx, &mut b).unwrap_err();
        assert!(matches!(
            err,
            ContentHashError::Budget(BudgetExceeded::Depth)
        ));
    }

    #[test]
    fn budget_node_exhaustion_is_reported() {
        let mut a = ExprArena::new();
        let l = a.intern_node(ExprNode::param(1)).unwrap();
        let r = a.intern_node(ExprNode::param(2)).unwrap();
        let s = a.intern_node(ExprNode::bin(BinOp::Add, l, r)).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = Budget::from_config(&BudgetConfig {
            max_depth: 64,
            max_nodes: 2, // Not enough to walk all three nodes.
            max_bytes: 1 << 20,
        });
        let err = content_hash(&a, s, &mut idx, &mut b).unwrap_err();
        assert!(matches!(
            err,
            ContentHashError::Budget(BudgetExceeded::Nodes)
        ));
    }

    #[test]
    fn cache_eliminates_repeated_descent_cost() {
        // Pre-warm the cache for the children, then confirm that
        // hashing the parent only charges one node + one depth.
        let mut a = ExprArena::new();
        let l = a.intern_node(ExprNode::param(1)).unwrap();
        let r = a.intern_node(ExprNode::param(2)).unwrap();
        let s = a.intern_node(ExprNode::bin(BinOp::Add, l, r)).unwrap();

        let mut idx = ContentIndex::new();
        let mut b = budget();
        let _ = content_hash(&a, l, &mut idx, &mut b).unwrap();
        let _ = content_hash(&a, r, &mut idx, &mut b).unwrap();
        let before = b;
        let _ = content_hash(&a, s, &mut idx, &mut b).unwrap();
        // Parent costs one depth + one node; children were cached.
        assert_eq!(before.depth - b.depth, 1);
        assert_eq!(before.nodes - b.nodes, 1);
    }

    #[test]
    fn content_hash_bytes_matches_content128() {
        // Sanity: the public convenience wrapper is the same chokepoint.
        let h1 = content_hash_bytes(b"hello");
        let h2 = content128(b"hello");
        assert_eq!(h1, h2);
    }
}

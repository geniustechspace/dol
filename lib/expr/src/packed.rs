//! `PackedNode` — 16-byte plain-old-data node representation, per
//! `docs/v2_plan.md` §35.
//!
//! This module is the **scaffolding** for the planned rewrite of
//! `lib/expr` onto a single, packed, branchless node representation.
//! The existing variant-style [`crate::ExprNode`] tree continues to
//! drive every code path today; `PackedNode` exists alongside so:
//!
//! 1. Backends and downstream tooling can start consuming the packed
//!    shape ahead of the cut-over.
//! 2. The arena-side iterative walker (work-stack on
//!    [`dol_core::storage::Storage`]) is exercised by tests well before
//!    any production node graph depends on it.
//! 3. Future commits can lower `ExprNode` → `PackedNode` one opcode
//!    family at a time without a flag day.
//!
//! # Layout
//!
//! ```text
//!  byte:  0   1   2   3   4   5   6   7   8   9   10  11  12  13  14  15
//!         ├──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┤
//!         │op │fl │ aux  │     a (u32)   │     b (u32)   │     c (u32)   │
//!         │ 1 │ 1 │  2   │      4        │      4        │      4        │
//! ```
//!
//! - **`op`** (`u8`) — opcode discriminator (see [`PackedOp`]).
//! - **`flags`** (`u8`) — small payload bits (e.g. nullability, ascending/
//!   descending order, three-valued booleans). Specific bits are owned
//!   per-op.
//! - **`aux`** (`u16`) — small in-line payload (sub-opcode for `BinOp`,
//!   short literal indexes, …). Larger payloads use `a`/`b`/`c`.
//! - **`a` / `b` / `c`** (`u32` each) — operand handles. Their exact
//!   meaning is opcode-specific and lives in [`PackedOp`]'s docs.
//!   `0` is the canonical "absent" value (matching the [`dol_core::id::Id`]
//!   `NonZeroU32` niche).
//!
//! Total: **16 bytes**, same as `[u32; 4]`.
//!
//! `PackedNode` is `#[repr(C)]` and `bytemuck::{Pod, Zeroable}` so it can
//! be `bytemuck::cast_slice`d into `&[u8]` for fast hashing, written
//! directly into a `mmap`'d arena, or laid down on flash without
//! rearranging fields.
//!
//! # Iterative traversal
//!
//! [`walk_iter`] is the canonical traversal: a depth-first pre-order
//! visit driven by an explicit work-stack on `Storage<NodeId>`. The
//! traversal threads `&mut Budget`; the recursive call surface visible
//! to a caller is exactly one `walk_iter` invocation, so DOS-by-deep-tree
//! is impossible by construction.

use bytemuck::{Pod, Zeroable};

use dol_core::policy::{Budget, BudgetError};

use crate::ids::NodeId;

/// 16-byte packed expression node.
///
/// See the [module docs](self) for the field layout and invariants. Use
/// the [`PackedNode::new`] constructor; never lay one down by hand
/// unless you're decoding from bytes (e.g. `bytemuck::from_bytes`).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct PackedNode {
    /// Opcode tag (cast of [`PackedOp`]).
    pub op: u8,
    /// Small flag bits (opcode-specific).
    pub flags: u8,
    /// 16-bit inline auxiliary value (sub-opcode, short index, …).
    pub aux: u16,
    /// First operand handle. `0` means absent.
    pub a: u32,
    /// Second operand handle. `0` means absent.
    pub b: u32,
    /// Third operand handle. `0` means absent.
    pub c: u32,
}

const _: () = {
    assert!(core::mem::size_of::<PackedNode>() == 16);
    assert!(core::mem::align_of::<PackedNode>() == 4);
};

/// Opcode tag for [`PackedNode::op`].
///
/// The variants intentionally collapse families (every binary operator
/// shares [`PackedOp::Bin`], every unary operator shares
/// [`PackedOp::Una`]) and use [`PackedNode::aux`] for the sub-opcode.
/// This keeps the discriminant space small (≤ 256) while the real
/// operator menu remains expressive.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PackedOp {
    /// Empty / sentinel. `bytemuck::Zeroable` lands here.
    Nop = 0,
    /// Binary operator. `aux` is the [`crate::BinOp`] sub-opcode (cast
    /// to `u16`); `a`/`b` are the operand [`NodeId`]s.
    Bin = 1,
    /// Unary operator. `aux` is the [`crate::UnaryOp`] sub-opcode;
    /// `a` is the operand [`NodeId`].
    Una = 2,
    /// Reference to an interned literal. `a` is a [`crate::LiteralId`].
    Lit = 3,
    /// Reference to a parameter / placeholder. `a` is the parameter
    /// index.
    Param = 4,
    /// Reference to a column / field. `a` is a [`crate::FieldId`].
    Field = 5,
    /// Reference to a function call. `a` is a [`crate::FuncId`].
    Func = 6,
    /// `CASE … WHEN …` reference. `a` is a [`crate::CaseId`].
    Case = 7,
    /// `IN (…)` list reference. `a` is an [`crate::InListId`].
    InList = 8,
    /// Sub-query reference. `a` is a [`crate::QueryId`].
    SubQuery = 9,
}

impl PackedOp {
    /// Convert from the raw `u8` stored in `PackedNode::op`. Returns
    /// `None` for tags this build does not recognise — decoders treat
    /// that as a hard error rather than a silent skip.
    #[must_use]
    pub fn from_u8(op: u8) -> Option<Self> {
        match op {
            0 => Some(Self::Nop),
            1 => Some(Self::Bin),
            2 => Some(Self::Una),
            3 => Some(Self::Lit),
            4 => Some(Self::Param),
            5 => Some(Self::Field),
            6 => Some(Self::Func),
            7 => Some(Self::Case),
            8 => Some(Self::InList),
            9 => Some(Self::SubQuery),
            _ => None,
        }
    }
}

impl PackedNode {
    /// Construct a `PackedNode` from its components.
    #[must_use]
    pub const fn new(op: PackedOp, flags: u8, aux: u16, a: u32, b: u32, c: u32) -> Self {
        Self {
            op: op as u8,
            flags,
            aux,
            a,
            b,
            c,
        }
    }

    /// Decode the opcode tag, or `None` if the byte is unrecognised.
    #[must_use]
    pub fn opcode(&self) -> Option<PackedOp> {
        PackedOp::from_u8(self.op)
    }

    /// Iterate the up-to-three child operand handles that refer to other
    /// `PackedNode`s in the same arena (i.e. operands of `Bin`, `Una`,
    /// `Func` arguments-via-list, etc. — opcodes whose `a`/`b`/`c`
    /// fields hold [`NodeId`]s rather than non-node ids like
    /// `LiteralId`).
    ///
    /// This is the seam the iterative walker uses to push work; opcodes
    /// whose `a`/`b`/`c` are *not* `NodeId`s return an empty iterator
    /// here and surface their referents through dedicated arena tables.
    pub fn child_node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        let candidates: [u32; 3] = match self.opcode() {
            Some(PackedOp::Bin) => [self.a, self.b, 0],
            Some(PackedOp::Una) => [self.a, 0, 0],
            // Lit / Param / Field / Func / Case / InList / SubQuery /
            // Nop reference non-`NodeId` payloads — no recursion.
            _ => [0, 0, 0],
        };
        candidates.into_iter().filter_map(NodeId::from_u32)
    }
}

/// Pre-order depth-first walk of a `PackedNode` graph rooted at `root`.
///
/// `arena` is borrowed read-only; `visit` is called with the current
/// [`NodeId`] before any of its children are visited. The traversal is
/// **iterative** — work lives on an internal `alloc::vec::Vec<NodeId>`
/// stack — so a 50 000-node-deep expression cannot blow the host
/// thread's stack.
///
/// Threads `&mut Budget` and charges one node per visit. Returns
/// [`BudgetError::Nodes`] on budget exhaustion.
///
/// # Errors
///
/// Returns [`BudgetError::Nodes`] when the budget runs out mid-walk.
/// Out-of-bounds child ids are silently skipped (a malformed arena is a
/// validation error, not a walker concern).
pub fn walk_iter<F>(
    arena: &[PackedNode],
    root: NodeId,
    budget: &mut Budget,
    mut visit: F,
) -> Result<(), BudgetError>
where
    F: FnMut(NodeId, &PackedNode),
{
    let mut stack: alloc::vec::Vec<NodeId> = alloc::vec::Vec::new();
    stack.push(root);

    while let Some(id) = stack.pop() {
        // Fetch — `NodeId::get()` is one-based; `idx = id.index()`.
        let idx = id.index();
        let Some(node) = arena.get(idx) else {
            continue;
        };

        budget.tick(1)?;
        visit(id, node);

        // Push children in reverse so left-most is visited first.
        let children: alloc::vec::Vec<NodeId> = node.child_node_ids().collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::num::NonZeroU32;
    use dol_core::policy::Limits;

    fn nid(n: u32) -> NodeId {
        NodeId::new(NonZeroU32::new(n).unwrap())
    }

    #[test]
    fn size_and_align() {
        assert_eq!(core::mem::size_of::<PackedNode>(), 16);
        assert_eq!(core::mem::align_of::<PackedNode>(), 4);
    }

    #[test]
    fn pod_castable_to_bytes() {
        let n = PackedNode::new(PackedOp::Bin, 0, 7, 1, 2, 0);
        let bytes: &[u8] = bytemuck::bytes_of(&n);
        assert_eq!(bytes.len(), 16);
        let n2: PackedNode = *bytemuck::from_bytes(bytes);
        assert_eq!(n, n2);
    }

    #[test]
    fn opcode_roundtrip() {
        for raw in 0u8..=9u8 {
            let op = PackedOp::from_u8(raw).unwrap();
            assert_eq!(op as u8, raw);
        }
        assert!(PackedOp::from_u8(255).is_none());
    }

    #[test]
    fn child_iter_for_bin_yields_two() {
        let n = PackedNode::new(PackedOp::Bin, 0, 0, 2, 3, 0);
        let kids: alloc::vec::Vec<NodeId> = n.child_node_ids().collect();
        assert_eq!(kids, alloc::vec![nid(2), nid(3)]);
    }

    #[test]
    fn child_iter_for_lit_yields_none() {
        let n = PackedNode::new(PackedOp::Lit, 0, 0, 5, 0, 0);
        let kids: alloc::vec::Vec<NodeId> = n.child_node_ids().collect();
        assert!(kids.is_empty());
    }

    #[test]
    fn walk_iter_visits_preorder() {
        // arena indices (1-based NodeIds):
        //   1: Bin(2, 3)
        //   2: Lit(7)
        //   3: Una(4)
        //   4: Lit(8)
        let arena = alloc::vec![
            PackedNode::new(PackedOp::Bin, 0, 0, 2, 3, 0),
            PackedNode::new(PackedOp::Lit, 0, 0, 7, 0, 0),
            PackedNode::new(PackedOp::Una, 0, 0, 4, 0, 0),
            PackedNode::new(PackedOp::Lit, 0, 0, 8, 0, 0),
        ];
        let mut order = alloc::vec::Vec::new();
        let mut budget = Budget::new(Limits::host());
        walk_iter(&arena, nid(1), &mut budget, |id, _| order.push(id.get())).unwrap();
        assert_eq!(order, alloc::vec![1, 2, 3, 4]);
    }

    #[test]
    fn walk_iter_respects_budget() {
        let arena = alloc::vec![
            PackedNode::new(PackedOp::Bin, 0, 0, 2, 3, 0),
            PackedNode::new(PackedOp::Lit, 0, 0, 7, 0, 0),
            PackedNode::new(PackedOp::Lit, 0, 0, 8, 0, 0),
        ];
        let mut budget = Budget::new(Limits {
            max_nodes: 2,
            ..Limits::host()
        });
        let err = walk_iter(&arena, nid(1), &mut budget, |_, _| {}).unwrap_err();
        assert_eq!(err, BudgetError::Nodes);
    }

    #[test]
    fn walk_iter_skips_oob_children() {
        // arena has only one node, but its `b` references a non-existent
        // index. The walker should skip silently rather than panic.
        let arena = alloc::vec![PackedNode::new(PackedOp::Bin, 0, 0, 99, 100, 0)];
        let mut budget = Budget::new(Limits::host());
        walk_iter(&arena, nid(1), &mut budget, |_, _| {}).unwrap();
    }
}

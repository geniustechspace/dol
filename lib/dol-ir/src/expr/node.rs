//! [`ExprNode`] — the v2 16-byte expression record.
//!
//! Per `dol-rewrite-plan-v2.md` §8.1.
//!
//! ## Layout (`#[repr(C)]`, exactly 16 bytes)
//!
//! | offset | size | field   | meaning                                              |
//! |-------:|-----:|---------|------------------------------------------------------|
//! |      0 |    1 | `op`    | opcode family ([`OpFamily`])                         |
//! |      1 |    1 | `flags` | [`NodeFlags`] bitset                                 |
//! |      2 |    2 | `aux`   | sub-opcode (`BinOp`/`UnaryOp`) or u16 inline payload |
//! |      4 |    4 | `a`     | operand A — `NodeId` / `LiteralId` / `FieldId` / …   |
//! |      8 |    4 | `b`     | operand B                                            |
//! |     12 |    4 | `c`     | operand C                                            |
//!
//! The 16-byte budget is **non-negotiable** — it is asserted both by
//! `const _: () = assert!(...)` below and by `xtask size-check`.
//!
//! ## Encapsulation
//!
//! Raw `a` / `b` / `c` field access is **not** exposed outside this
//! module. All construction goes through the typed `ExprNode::*`
//! constructors and all reading through `as_*` accessors. This is
//! what keeps the opcode↔operand contract enforced by the type
//! system — a `BinOp` node always has two `NodeId` operands; you
//! cannot accidentally read its `c` field as anything else.

use bytemuck::{Pod, Zeroable};

use dol_cas::handle::{FieldId, FuncId, Lid, LiteralId, NodeId};

use super::flags::NodeFlags;
use super::ops::{BinOp, OpFamily, UnaryOp};

/// 16-byte expression record. Members are private so that the
/// constructor / accessor pairs in this module are the only way to
/// build or inspect a node.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Pod, Zeroable)]
pub struct ExprNode {
    op: u8,
    flags: u8,
    aux: u16,
    a: u32,
    b: u32,
    c: u32,
}

// Compile-time guarantee. Mirrors the runtime test in `mod tests` and
// the xtask `size-check` assertion. Any change that grows `ExprNode`
// beyond 16 bytes fails to build.
const _: () = assert!(core::mem::size_of::<ExprNode>() == 16);
const _: () = assert!(core::mem::align_of::<ExprNode>() == 4);

impl ExprNode {
    // ─── Family / flag accessors ─────────────────────────────────────

    /// Opcode family. Returns `None` for unknown / future variants.
    #[must_use]
    #[inline]
    pub fn family(&self) -> Option<OpFamily> {
        OpFamily::try_from_u8(self.op)
    }

    /// Per-node bitset.
    #[must_use]
    #[inline]
    pub const fn flags(&self) -> NodeFlags {
        NodeFlags::from_bits(self.flags)
    }

    /// Replace the bitset.
    #[inline]
    pub fn set_flags(&mut self, flags: NodeFlags) {
        self.flags = flags.to_bits();
    }

    // ─── Constructors ───────────────────────────────────────────────
    //
    // Each constructor maps a typed shape onto the 16-byte slab.
    // The match between family + aux + operand types is the contract
    // enforced by these methods. Tests round-trip every constructor
    // through its matching accessor.

    /// `left op right` — binary operation.
    #[must_use]
    #[inline]
    pub fn bin(op: BinOp, left: NodeId, right: NodeId) -> Self {
        Self {
            op: OpFamily::Bin.as_u8(),
            flags: 0,
            aux: op.as_u16(),
            a: left.get(),
            b: right.get(),
            c: 0,
        }
    }

    /// `op operand` — unary operation.
    #[must_use]
    #[inline]
    pub fn unary(op: UnaryOp, operand: NodeId) -> Self {
        Self {
            op: OpFamily::Unary.as_u8(),
            flags: 0,
            aux: op.as_u16(),
            a: operand.get(),
            b: 0,
            c: 0,
        }
    }

    /// Reference an interned literal.
    #[must_use]
    #[inline]
    pub fn lit_ref(id: LiteralId) -> Self {
        Self {
            op: OpFamily::LitRef.as_u8(),
            flags: 0,
            aux: 0,
            a: id.get(),
            b: 0,
            c: 0,
        }
    }

    /// Reference a field.
    #[must_use]
    #[inline]
    pub fn field_ref(id: FieldId) -> Self {
        Self {
            op: OpFamily::FieldRef.as_u8(),
            flags: 0,
            aux: 0,
            a: id.get(),
            b: 0,
            c: 0,
        }
    }

    /// Reference a function / aggregate.
    #[must_use]
    #[inline]
    pub fn func_ref(id: FuncId) -> Self {
        Self {
            op: OpFamily::FuncRef.as_u8(),
            flags: 0,
            aux: 0,
            a: id.get(),
            b: 0,
            c: 0,
        }
    }

    /// Positional bind parameter `$index`.
    #[must_use]
    #[inline]
    pub fn param(index: u16) -> Self {
        Self {
            op: OpFamily::Param.as_u8(),
            flags: 0,
            aux: index,
            a: 0,
            b: 0,
            c: 0,
        }
    }

    /// Wildcard projection (`*`).
    #[must_use]
    #[inline]
    pub fn wildcard() -> Self {
        Self {
            op: OpFamily::Wildcard.as_u8(),
            flags: 0,
            aux: 0,
            a: 0,
            b: 0,
            c: 0,
        }
    }

    /// `COUNT(*)` aggregate.
    #[must_use]
    #[inline]
    pub fn count_all() -> Self {
        Self {
            op: OpFamily::CountAll.as_u8(),
            flags: 0,
            aux: 0,
            a: 0,
            b: 0,
            c: 0,
        }
    }

    // ─── Accessors ──────────────────────────────────────────────────
    //
    // Each accessor first matches the family, then decodes `aux` /
    // operand fields into typed handles. Returns `None` if the family
    // does not match — this is the only correct way for callers to
    // inspect operands.

    /// Decode as a binary operation.
    #[must_use]
    pub fn as_bin(&self) -> Option<(BinOp, NodeId, NodeId)> {
        if self.family()? != OpFamily::Bin {
            return None;
        }
        let op = BinOp::try_from_u16(self.aux)?;
        let left = Lid::from_u32(self.a)?;
        let right = Lid::from_u32(self.b)?;
        Some((op, left, right))
    }

    /// Decode as a unary operation.
    #[must_use]
    pub fn as_unary(&self) -> Option<(UnaryOp, NodeId)> {
        if self.family()? != OpFamily::Unary {
            return None;
        }
        let op = UnaryOp::try_from_u16(self.aux)?;
        let operand = Lid::from_u32(self.a)?;
        Some((op, operand))
    }

    /// Decode as a literal reference.
    #[must_use]
    pub fn as_lit_ref(&self) -> Option<LiteralId> {
        if self.family()? != OpFamily::LitRef {
            return None;
        }
        Lid::from_u32(self.a)
    }

    /// Decode as a field reference.
    #[must_use]
    pub fn as_field_ref(&self) -> Option<FieldId> {
        if self.family()? != OpFamily::FieldRef {
            return None;
        }
        Lid::from_u32(self.a)
    }

    /// Decode as a function reference.
    #[must_use]
    pub fn as_func_ref(&self) -> Option<FuncId> {
        if self.family()? != OpFamily::FuncRef {
            return None;
        }
        Lid::from_u32(self.a)
    }

    /// Decode as a positional parameter; returns the parameter index.
    #[must_use]
    pub fn as_param(&self) -> Option<u16> {
        if self.family()? != OpFamily::Param {
            return None;
        }
        Some(self.aux)
    }

    /// `true` iff this node is a wildcard projection.
    #[must_use]
    pub fn is_wildcard(&self) -> bool {
        self.family() == Some(OpFamily::Wildcard)
    }

    /// `true` iff this node is `COUNT(*)`.
    #[must_use]
    pub fn is_count_all(&self) -> bool {
        self.family() == Some(OpFamily::CountAll)
    }
}

impl core::fmt::Debug for ExprNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("ExprNode");
        s.field("family", &self.family());
        s.field("flags", &self.flags());
        s.field("aux", &self.aux);
        s.field("a", &self.a);
        s.field("b", &self.b);
        s.field("c", &self.c);
        s.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    fn nid(i: u32) -> NodeId {
        Lid::from_u32(i).unwrap()
    }

    #[test]
    fn expr_node_is_exactly_16_bytes() {
        assert_eq!(size_of::<ExprNode>(), 16);
        assert_eq!(align_of::<ExprNode>(), 4);
    }

    #[test]
    fn bin_round_trip() {
        let n = ExprNode::bin(BinOp::Add, nid(7), nid(11));
        assert_eq!(n.family(), Some(OpFamily::Bin));
        assert_eq!(n.as_bin(), Some((BinOp::Add, nid(7), nid(11))));
        assert_eq!(n.as_unary(), None);
        assert_eq!(n.as_lit_ref(), None);
    }

    #[test]
    fn unary_round_trip() {
        let n = ExprNode::unary(UnaryOp::Not, nid(3));
        assert_eq!(n.as_unary(), Some((UnaryOp::Not, nid(3))));
        assert_eq!(n.as_bin(), None);
    }

    #[test]
    fn lit_ref_round_trip() {
        let id: LiteralId = Lid::from_u32(42).unwrap();
        let n = ExprNode::lit_ref(id);
        assert_eq!(n.as_lit_ref(), Some(id));
        assert_eq!(n.as_field_ref(), None);
    }

    #[test]
    fn field_ref_round_trip() {
        let id: FieldId = Lid::from_u32(99).unwrap();
        let n = ExprNode::field_ref(id);
        assert_eq!(n.as_field_ref(), Some(id));
        assert_eq!(n.as_lit_ref(), None);
    }

    #[test]
    fn func_ref_round_trip() {
        let id: FuncId = Lid::from_u32(13).unwrap();
        let n = ExprNode::func_ref(id);
        assert_eq!(n.as_func_ref(), Some(id));
    }

    #[test]
    fn param_round_trip() {
        let n = ExprNode::param(5);
        assert_eq!(n.as_param(), Some(5));
        assert_eq!(n.as_bin(), None);
    }

    #[test]
    fn wildcard_round_trip() {
        let n = ExprNode::wildcard();
        assert!(n.is_wildcard());
        assert!(!n.is_count_all());
        assert_eq!(n.family(), Some(OpFamily::Wildcard));
    }

    #[test]
    fn count_all_round_trip() {
        let n = ExprNode::count_all();
        assert!(n.is_count_all());
        assert!(!n.is_wildcard());
    }

    #[test]
    fn flags_round_trip_through_node() {
        let mut n = ExprNode::bin(BinOp::Eq, nid(1), nid(2));
        assert_eq!(n.flags(), NodeFlags::new());
        n.set_flags(NodeFlags::new().with_nullable(true).with_negated(true));
        assert!(n.flags().is_nullable());
        assert!(n.flags().is_negated());
        // Operands unchanged.
        assert_eq!(n.as_bin(), Some((BinOp::Eq, nid(1), nid(2))));
    }

    #[test]
    fn distinct_families_dont_alias() {
        let a = ExprNode::wildcard();
        let b = ExprNode::count_all();
        assert_ne!(a, b);
        assert_ne!(a.family(), b.family());
    }

    #[test]
    fn pod_round_trip_via_bytes() {
        // `Pod` lets us memcpy the node back and forth, which is the
        // whole point of the 16-byte fixed layout.
        let n = ExprNode::bin(BinOp::Mul, nid(100), nid(200));
        let bytes: [u8; 16] = bytemuck::cast(n);
        let back: ExprNode = bytemuck::cast(bytes);
        assert_eq!(n, back);
        assert_eq!(back.as_bin(), Some((BinOp::Mul, nid(100), nid(200))));
    }
}

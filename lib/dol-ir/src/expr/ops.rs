//! Opcode tables for [`ExprNode`](super::node::ExprNode).
//!
//! Per `dol-rewrite-plan-v2.md` §8.1.
//!
//! ## Append-only contract
//!
//! Once a numeric value is shipped in a v2 release it is **frozen
//! forever**:
//!
//! - New variants are appended at the **end** of each enum.
//! - Existing numeric values are **never renumbered**.
//! - Removed variants leave a gap; the number is **never reused**.
//!
//! This contract is what lets `Program` blobs round-trip across
//! compiler versions and what makes `BinOp::try_from_u16` a stable
//! decode point. The numeric-stability snapshot test in `mod tests`
//! at the bottom of this file fails CI if any existing value moves.

/// Top-level family of an [`ExprNode`](super::node::ExprNode).
///
/// Stored in `ExprNode::op` as a `u8`. `aux` further refines families
/// that have a sub-opcode (e.g. `Bin` → [`BinOp`], `Unary` →
/// [`UnaryOp`]).
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpFamily {
    /// Reserved sentinel. Never assigned to a real node.
    Reserved = 0,
    /// Binary operation. `aux` is a [`BinOp`]; `a`/`b` are
    /// [`NodeId`](dol_cas::handle::NodeId) operands.
    Bin = 1,
    /// Unary operation. `aux` is a [`UnaryOp`]; `a` is a `NodeId`.
    Unary = 2,
    /// Reference to an interned [`Literal`](dol_core) via
    /// [`LiteralId`](dol_cas::handle::LiteralId).
    LitRef = 3,
    /// Reference to a [`FieldId`](dol_cas::handle::FieldId).
    FieldRef = 4,
    /// Reference to a [`FuncId`](dol_cas::handle::FuncId).
    FuncRef = 5,
    /// Positional bind parameter. `aux` carries the parameter index.
    Param = 6,
    /// Wildcard projection (`*`). No operands.
    Wildcard = 7,
    /// `COUNT(*)` aggregate. No operands.
    CountAll = 8,
    /// Reference to an interned [`Path<StrId>`](dol_core::path::Path)
    /// via [`PathId`](dol_cas::handle::PathId). Carries the lowered
    /// form of `Expr::Ref(Path<Name>)`. `a` is the [`PathId`].
    PathRef = 9,
    /// Variadic ordered sequence (`Expr::Seq` lowering). `b`/`c`
    /// encode an [`OperandSpan`](super::slab::OperandSpan) into the
    /// arena's [`OperandSlab`](super::slab::OperandSlab) — each `u32`
    /// slot is a [`NodeId`](dol_cas::handle::NodeId).
    Seq = 10,
    /// Variadic key-value mapping (`Expr::Map` lowering). `b`/`c`
    /// encode an [`OperandSpan`](super::slab::OperandSpan); slots
    /// alternate `[StrId, NodeId, StrId, NodeId, …]` with even total
    /// length.
    Map = 11,
    /// Function / aggregate call (`Expr::Call` lowering). `a` is a
    /// [`FuncId`](dol_cas::handle::FuncId); `b`/`c` encode an
    /// [`OperandSpan`](super::slab::OperandSpan) of [`NodeId`] argument slots.
    Call = 12,
    /// Type coercion (`Expr::Cast` lowering). `a` is the operand
    /// [`NodeId`](dol_cas::handle::NodeId); `b` is a
    /// [`TypeId`](dol_cas::handle::TypeId) into the
    /// [`TypePool`](super::types::TypePool).
    Cast = 13,
    /// Multi-arm conditional (`Expr::Match` lowering). `a` is the
    /// fallback [`NodeId`](dol_cas::handle::NodeId) when present
    /// (zero when absent); `b`/`c` encode an
    /// [`OperandSpan`](super::slab::OperandSpan) over alternating
    /// `[cond, result, cond, result, …]` arms with even length.
    Match = 14,
    /// Two-branch conditional (`Expr::If` lowering). `a` is the
    /// condition, `b` the then-branch, `c` the else-branch.
    If = 15,
    /// Inclusive range containment (`Expr::InRange` lowering).
    /// `a` is the expression, `b` the lower bound, `c` the upper bound.
    InRange = 16,
    /// Set membership (`Expr::MemberOf` lowering). `a` is the
    /// expression; `b`/`c` encode an
    /// [`OperandSpan`](super::slab::OperandSpan) of candidate-set
    /// [`NodeId`] slots.
    MemberOf = 17,
    /// Output-label decorator (`Expr::Label` lowering). `a` is the
    /// expression [`NodeId`](dol_cas::handle::NodeId), `b` is the
    /// label [`StrId`](dol_cas::handle::StrId).
    Label = 18,
    /// Scoped evaluation (`Expr::Scoped` lowering). `a` is the
    /// expression [`NodeId`](dol_cas::handle::NodeId), `b` is a
    /// [`ContextId`](dol_cas::handle::ContextId) into the
    /// [`ContextPool`](super::contexts::ContextPool).
    Scoped = 19,
    // Append new families here. Never renumber. Never reuse.
}

impl OpFamily {
    /// Decode a `u8` back into a family. Returns `None` for unknown
    /// values — the caller must decide whether to error or treat the
    /// node as opaque.
    #[must_use]
    pub fn try_from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Reserved,
            1 => Self::Bin,
            2 => Self::Unary,
            3 => Self::LitRef,
            4 => Self::FieldRef,
            5 => Self::FuncRef,
            6 => Self::Param,
            7 => Self::Wildcard,
            8 => Self::CountAll,
            9 => Self::PathRef,
            10 => Self::Seq,
            11 => Self::Map,
            12 => Self::Call,
            13 => Self::Cast,
            14 => Self::Match,
            15 => Self::If,
            16 => Self::InRange,
            17 => Self::MemberOf,
            18 => Self::Label,
            19 => Self::Scoped,
            _ => return None,
        })
    }

    /// Cast back to its `u8` representation.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Binary operator. Stored in `ExprNode::aux` when
/// `ExprNode::op == OpFamily::Bin`.
///
/// Numeric values follow the plan's grouping (arithmetic 0–4,
/// comparison 10–15, logical 20–21, bitwise 30–34, concat 40,
/// pattern-match 50–51) so future opcodes can be appended within
/// each band without disturbing existing numbers.
#[repr(u16)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // ── Arithmetic ─────────────────────────────────────────────────
    /// `a + b`
    Add = 0,
    /// `a - b`
    Sub = 1,
    /// `a * b`
    Mul = 2,
    /// `a / b`
    Div = 3,
    /// `a % b` (modulo / remainder)
    Rem = 4,

    // ── Comparison ─────────────────────────────────────────────────
    /// `a = b`
    Eq = 10,
    /// `a <> b`
    Ne = 11,
    /// `a < b`
    Lt = 12,
    /// `a <= b`
    Le = 13,
    /// `a > b`
    Gt = 14,
    /// `a >= b`
    Ge = 15,

    // ── Logical ────────────────────────────────────────────────────
    /// `a AND b`
    And = 20,
    /// `a OR b`
    Or = 21,

    // ── Bitwise ────────────────────────────────────────────────────
    /// `a & b`
    BitAnd = 30,
    /// `a | b`
    BitOr = 31,
    /// `a ^ b`
    BitXor = 32,
    /// `a << b`
    Shl = 33,
    /// `a >> b`
    Shr = 34,

    // ── String ─────────────────────────────────────────────────────
    /// `a || b` (string concatenation)
    Concat = 40,

    // ── Pattern match ──────────────────────────────────────────────
    /// `a LIKE b` (case-sensitive pattern match)
    Like = 50,
    /// `a ILIKE b` (case-insensitive pattern match)
    ILike = 51,
    // Append new variants at the end only. Never renumber. Never reuse.
}

impl BinOp {
    /// Decode a `u16` back into a binary opcode. Returns `None` for
    /// unknown values; callers should treat this as a wire-format or
    /// version-skew error.
    #[must_use]
    pub fn try_from_u16(v: u16) -> Option<Self> {
        Some(match v {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::Mul,
            3 => Self::Div,
            4 => Self::Rem,
            10 => Self::Eq,
            11 => Self::Ne,
            12 => Self::Lt,
            13 => Self::Le,
            14 => Self::Gt,
            15 => Self::Ge,
            20 => Self::And,
            21 => Self::Or,
            30 => Self::BitAnd,
            31 => Self::BitOr,
            32 => Self::BitXor,
            33 => Self::Shl,
            34 => Self::Shr,
            40 => Self::Concat,
            50 => Self::Like,
            51 => Self::ILike,
            _ => return None,
        })
    }

    /// Cast to the `u16` representation stored in `ExprNode::aux`.
    #[must_use]
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Unary operator. Stored in `ExprNode::aux` when
/// `ExprNode::op == OpFamily::Unary`.
#[repr(u16)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `NOT a` (logical negation; also used to negate `LIKE`,
    /// `BETWEEN`, `IN` per the plan's normalisation rule).
    Not = 0,
    /// `-a` (arithmetic negation).
    Neg = 1,
    /// `~a` (bitwise complement).
    BitNot = 2,
    /// `a IS NULL`.
    IsNull = 10,
    /// `a IS NOT NULL`.
    IsNotNull = 11,
    // Append new variants at the end only. Never renumber. Never reuse.
}

impl UnaryOp {
    /// Decode a `u16` back into a unary opcode.
    #[must_use]
    pub fn try_from_u16(v: u16) -> Option<Self> {
        Some(match v {
            0 => Self::Not,
            1 => Self::Neg,
            2 => Self::BitNot,
            10 => Self::IsNull,
            11 => Self::IsNotNull,
            _ => return None,
        })
    }

    /// Cast to the `u16` representation stored in `ExprNode::aux`.
    #[must_use]
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Numeric-stability snapshot. **DO NOT** change these numbers
    /// without bumping the wire-format version. Any modification here
    /// breaks every persisted `Program` blob.
    #[test]
    fn binop_numeric_stability() {
        assert_eq!(BinOp::Add as u16, 0);
        assert_eq!(BinOp::Sub as u16, 1);
        assert_eq!(BinOp::Mul as u16, 2);
        assert_eq!(BinOp::Div as u16, 3);
        assert_eq!(BinOp::Rem as u16, 4);
        assert_eq!(BinOp::Eq as u16, 10);
        assert_eq!(BinOp::Ne as u16, 11);
        assert_eq!(BinOp::Lt as u16, 12);
        assert_eq!(BinOp::Le as u16, 13);
        assert_eq!(BinOp::Gt as u16, 14);
        assert_eq!(BinOp::Ge as u16, 15);
        assert_eq!(BinOp::And as u16, 20);
        assert_eq!(BinOp::Or as u16, 21);
        assert_eq!(BinOp::BitAnd as u16, 30);
        assert_eq!(BinOp::BitOr as u16, 31);
        assert_eq!(BinOp::BitXor as u16, 32);
        assert_eq!(BinOp::Shl as u16, 33);
        assert_eq!(BinOp::Shr as u16, 34);
        assert_eq!(BinOp::Concat as u16, 40);
        assert_eq!(BinOp::Like as u16, 50);
        assert_eq!(BinOp::ILike as u16, 51);
    }

    #[test]
    fn unaryop_numeric_stability() {
        assert_eq!(UnaryOp::Not as u16, 0);
        assert_eq!(UnaryOp::Neg as u16, 1);
        assert_eq!(UnaryOp::BitNot as u16, 2);
        assert_eq!(UnaryOp::IsNull as u16, 10);
        assert_eq!(UnaryOp::IsNotNull as u16, 11);
    }

    #[test]
    fn opfamily_numeric_stability() {
        assert_eq!(OpFamily::Reserved as u8, 0);
        assert_eq!(OpFamily::Bin as u8, 1);
        assert_eq!(OpFamily::Unary as u8, 2);
        assert_eq!(OpFamily::LitRef as u8, 3);
        assert_eq!(OpFamily::FieldRef as u8, 4);
        assert_eq!(OpFamily::FuncRef as u8, 5);
        assert_eq!(OpFamily::Param as u8, 6);
        assert_eq!(OpFamily::Wildcard as u8, 7);
        assert_eq!(OpFamily::CountAll as u8, 8);
        assert_eq!(OpFamily::PathRef as u8, 9);
        assert_eq!(OpFamily::Seq as u8, 10);
        assert_eq!(OpFamily::Map as u8, 11);
        assert_eq!(OpFamily::Call as u8, 12);
        assert_eq!(OpFamily::Cast as u8, 13);
        assert_eq!(OpFamily::Match as u8, 14);
        assert_eq!(OpFamily::If as u8, 15);
        assert_eq!(OpFamily::InRange as u8, 16);
        assert_eq!(OpFamily::MemberOf as u8, 17);
        assert_eq!(OpFamily::Label as u8, 18);
        assert_eq!(OpFamily::Scoped as u8, 19);
    }

    #[test]
    fn binop_round_trip_for_every_known_value() {
        let known = [
            0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 20, 21, 30, 31, 32, 33, 34, 40, 50, 51,
        ];
        for v in known {
            let op = BinOp::try_from_u16(v).expect("known");
            assert_eq!(op.as_u16(), v);
        }
    }

    #[test]
    fn binop_rejects_unknown() {
        assert!(BinOp::try_from_u16(5).is_none());
        assert!(BinOp::try_from_u16(9).is_none());
        assert!(BinOp::try_from_u16(99).is_none());
        assert!(BinOp::try_from_u16(u16::MAX).is_none());
    }

    #[test]
    fn unaryop_round_trip_for_every_known_value() {
        for v in [0, 1, 2, 10, 11] {
            let op = UnaryOp::try_from_u16(v).expect("known");
            assert_eq!(op.as_u16(), v);
        }
    }

    #[test]
    fn unaryop_rejects_unknown() {
        assert!(UnaryOp::try_from_u16(3).is_none());
        assert!(UnaryOp::try_from_u16(12).is_none());
    }

    #[test]
    fn opfamily_round_trip() {
        for v in 0..=19u8 {
            let f = OpFamily::try_from_u8(v).expect("known");
            assert_eq!(f.as_u8(), v);
        }
        assert!(OpFamily::try_from_u8(20).is_none());
        assert!(OpFamily::try_from_u8(255).is_none());
    }
}

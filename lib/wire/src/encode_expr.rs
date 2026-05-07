//! [`Encode`] impls for `dol-expr` types.

extern crate alloc;

use dol_core::policy::Budget;
use dol_expr::arena::{
    ArrayLitNode, CaseNode, ExprArena, FieldNode, FieldStep, FuncNode, InListNode, ObjLitNode,
    Span, SpanTable, WindowNode,
};
use dol_expr::expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, ExprOp, InsertNode, JoinNode, JoinType, LockHint,
    Order, QueryNode, UnaryOp, UpdateNode, UpsertNode,
};
use dol_expr::interner::Interner;
use dol_expr::tree::window::{FrameBound, FrameKind, WindowFrame};

use crate::encoder::{Encode, EncodeError, Writer, encode_slice};

// ─── StrId ───────────────────────────────────────────────────────────────────

// ─── FieldStep ───────────────────────────────────────────────────────────────

impl Encode for FieldStep {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            FieldStep::Key(id) => {
                w.write_varint_u32(0)?;
                b.descend(|b| id.encode(w, b))??;
            }
            FieldStep::Index(i) => {
                w.write_varint_u32(1)?;
                w.write_varint_u32(*i)?;
            }
            _ => return Err(EncodeError::Custom("FieldStep: unknown variant")),
        }
        Ok(())
    }
}

// ─── FieldNode ───────────────────────────────────────────────────────────────

impl Encode for FieldNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.namespace.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.steps.encode(w, b))??;
        Ok(())
    }
}

// ─── FuncNode ────────────────────────────────────────────────────────────────

impl Encode for FuncNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.args.encode(w, b))??;
        Ok(())
    }
}

// ─── ObjLitNode ──────────────────────────────────────────────────────────────

impl Encode for ObjLitNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.0.encode(w, b))??;
        Ok(())
    }
}

// ─── Order ───────────────────────────────────────────────────────────────────

impl Encode for Order {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            Order::Asc => w.write_varint_u32(0),
            Order::Desc => w.write_varint_u32(1),
            _ => Err(EncodeError::Custom("unknown Order variant")),
        }
    }
}

// ─── LockHint ────────────────────────────────────────────────────────────────

impl Encode for LockHint {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            LockHint::ForUpdate => w.write_varint_u32(0),
            LockHint::ForShare => w.write_varint_u32(1),
            LockHint::SkipLocked => w.write_varint_u32(2),
            LockHint::NoWait => w.write_varint_u32(3),
            _ => Err(EncodeError::Custom("unknown LockHint variant")),
        }
    }
}

// ─── FrameBound ──────────────────────────────────────────────────────────────

impl Encode for FrameBound {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            FrameBound::UnboundedPreceding => w.write_varint_u32(0),
            FrameBound::Preceding(n) => {
                w.write_varint_u32(1)?;
                w.write_varint_u32(*n)
            }
            FrameBound::CurrentRow => w.write_varint_u32(2),
            FrameBound::Following(n) => {
                w.write_varint_u32(3)?;
                w.write_varint_u32(*n)
            }
            FrameBound::UnboundedFollowing => w.write_varint_u32(4),
        }
    }
}

// ─── FrameKind ───────────────────────────────────────────────────────────────

impl Encode for FrameKind {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            FrameKind::Rows => w.write_varint_u32(0),
            FrameKind::Range => w.write_varint_u32(1),
        }
    }
}

// ─── WindowFrame ─────────────────────────────────────────────────────────────

impl Encode for WindowFrame {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.kind.encode(w, b))??;
        b.descend(|b| self.start.encode(w, b))??;
        b.descend(|b| self.end.encode(w, b))??;
        Ok(())
    }
}

// ─── WindowNode ──────────────────────────────────────────────────────────────

impl Encode for WindowNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.func.encode(w, b))??;
        b.descend(|b| self.partition.encode(w, b))??;
        b.descend(|b| self.order.encode(w, b))??;
        b.descend(|b| self.frame.encode(w, b))??;
        Ok(())
    }
}

// ─── CaseNode ────────────────────────────────────────────────────────────────

impl Encode for CaseNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.branches.encode(w, b))??;
        b.descend(|b| self.else_.encode(w, b))??;
        Ok(())
    }
}

// ─── InListNode ──────────────────────────────────────────────────────────────

impl Encode for InListNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.expr.encode(w, b))??;
        b.descend(|b| self.list.encode(w, b))??;
        Ok(())
    }
}

// ─── ArrayLitNode ────────────────────────────────────────────────────────────

impl Encode for ArrayLitNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.items.encode(w, b))??;
        Ok(())
    }
}

// ─── Span / SpanTable ────────────────────────────────────────────────────────

impl Encode for Span {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_varint_u32(self.start)?;
        w.write_varint_u32(self.end)
    }
}

impl Encode for SpanTable {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| encode_slice(self.spans_slice(), w, b))??;
        b.descend(|b| encode_slice(self.owners_slice(), w, b))??;
        Ok(())
    }
}

// ─── JoinType ────────────────────────────────────────────────────────────────

impl Encode for JoinType {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            JoinType::Inner => w.write_varint_u32(0),
            JoinType::Left => w.write_varint_u32(1),
            JoinType::Right => w.write_varint_u32(2),
            JoinType::Full => w.write_varint_u32(3),
            JoinType::Cross => w.write_varint_u32(4),
            _ => Err(EncodeError::Custom("JoinType: unknown variant")),
        }
    }
}

// ─── JoinNode ────────────────────────────────────────────────────────────────

impl Encode for JoinNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.source.encode(w, b))??;
        b.descend(|b| self.alias.encode(w, b))??;
        b.descend(|b| self.join_type.encode(w, b))??;
        b.descend(|b| self.on.encode(w, b))??;
        Ok(())
    }
}

// ─── QueryNode ───────────────────────────────────────────────────────────────

impl Encode for QueryNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.from.encode(w, b))??;
        b.descend(|b| self.alias.encode(w, b))??;
        b.descend(|b| self.joins.encode(w, b))??;
        b.descend(|b| self.filter.encode(w, b))??;
        b.descend(|b| self.columns.encode(w, b))??;
        b.descend(|b| self.group_by.encode(w, b))??;
        b.descend(|b| self.having.encode(w, b))??;
        b.descend(|b| self.order_by.encode(w, b))??;
        b.descend(|b| self.limit.encode(w, b))??;
        b.descend(|b| self.offset.encode(w, b))??;
        b.descend(|b| self.lock.encode(w, b))??;
        Ok(())
    }
}

// ─── ConflictClause ──────────────────────────────────────────────────────────

impl Encode for ConflictClause {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            ConflictClause::DoNothing => w.write_varint_u32(0)?,
            ConflictClause::DoUpdate { assignments } => {
                w.write_varint_u32(1)?;
                b.descend(|b| assignments.encode(w, b))??;
            }
            _ => return Err(EncodeError::Custom("ConflictClause: unknown variant")),
        }
        Ok(())
    }
}

// ─── InsertNode ──────────────────────────────────────────────────────────────

impl Encode for InsertNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.columns.encode(w, b))??;
        b.descend(|b| self.values.encode(w, b))??;
        b.descend(|b| self.returning.encode(w, b))??;
        b.descend(|b| self.conflict.encode(w, b))??;
        Ok(())
    }
}

// ─── UpdateNode ──────────────────────────────────────────────────────────────

impl Encode for UpdateNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.columns.encode(w, b))??;
        b.descend(|b| self.values.encode(w, b))??;
        b.descend(|b| self.filter.encode(w, b))??;
        b.descend(|b| self.returning.encode(w, b))??;
        Ok(())
    }
}

// ─── DeleteNode ──────────────────────────────────────────────────────────────

impl Encode for DeleteNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.filter.encode(w, b))??;
        b.descend(|b| self.returning.encode(w, b))??;
        Ok(())
    }
}

// ─── UpsertNode ──────────────────────────────────────────────────────────────

impl Encode for UpsertNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.columns.encode(w, b))??;
        b.descend(|b| self.values.encode(w, b))??;
        b.descend(|b| self.returning.encode(w, b))??;
        b.descend(|b| self.conflict.encode(w, b))??;
        Ok(())
    }
}

// ─── BinOp ───────────────────────────────────────────────────────────────────

impl Encode for BinOp {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        let tag = match self {
            BinOp::Eq => 0,
            BinOp::Ne => 1,
            BinOp::Lt => 2,
            BinOp::Le => 3,
            BinOp::Gt => 4,
            BinOp::Ge => 5,
            BinOp::And => 6,
            BinOp::Or => 7,
            BinOp::Add => 8,
            BinOp::Sub => 9,
            BinOp::Mul => 10,
            BinOp::Div => 11,
            BinOp::Rem => 12,
            BinOp::Like => 13,
            BinOp::ILike => 14,
            BinOp::Similar => 15,
            BinOp::BitAnd => 16,
            BinOp::BitOr => 17,
            BinOp::BitXor => 18,
            BinOp::Shl => 19,
            BinOp::Shr => 20,
            BinOp::Concat => 21,
            BinOp::IsDistinctFrom => 22,
            BinOp::IsNotDistinctFrom => 23,
            _ => return Err(EncodeError::Custom("BinOp: unknown variant")),
        };
        w.write_varint_u32(tag)
    }
}

// ─── UnaryOp ─────────────────────────────────────────────────────────────────

impl Encode for UnaryOp {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        let tag = match self {
            UnaryOp::Neg => 0,
            UnaryOp::Not => 1,
            UnaryOp::BitNot => 2,
            UnaryOp::IsNull => 3,
            UnaryOp::IsNotNull => 4,
            UnaryOp::IsTrue => 5,
            UnaryOp::IsFalse => 6,
            _ => return Err(EncodeError::Custom("UnaryOp: unknown variant")),
        };
        w.write_varint_u32(tag)
    }
}

// ─── ExprNode (16 B packed POD) ──────────────────────────────────────────────

/// Encode the packed [`ExprNode`] field-by-field, opcode-aware.
///
/// We do NOT just dump the 16 raw bytes: that would commit the wire
/// format to a host-endian POD layout and waste bytes for opcodes that
/// don't use all of `a`/`b`/`c`. Instead, write the opcode first and
/// then only the fields that opcode actually uses (matching the
/// per-opcode "Field layout" table in `dol_expr::expr`).
///
/// The wire format is byte-stable across hosts and small (≈ 1–4 bytes
/// per leaf, ≈ 7 bytes per Bin) — comparable to the previous
/// variant-tag form.
///
/// **Budget accounting:** the whole node is one logical "field-of-its-
/// parent", so we charge one `descend` for the encode call as a whole
/// (in the caller's tree-walker), then write each scalar inline.
/// Per-scalar `descend` was the variant-form pattern and is unhelpful
/// here — these are flat reads off the POD, not recursive sub-encodes.
impl Encode for ExprNode {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        let op = ExprOp::from_u8(self.op).ok_or(EncodeError::Custom("ExprNode: unknown opcode"))?;
        // Tag byte first.
        w.write_varint_u32(self.op as u32)?;
        match op {
            // No payload: every field is zero.
            ExprOp::Nop => {}
            // Single u32 in `a` — pooled-id payload.
            ExprOp::Namespace
            | ExprOp::Field
            | ExprOp::Param
            | ExprOp::Lit
            | ExprOp::ObjectLit
            | ExprOp::ArrayLit
            | ExprOp::Func
            | ExprOp::Window
            | ExprOp::Case
            | ExprOp::InList
            | ExprOp::Query
            | ExprOp::Insert
            | ExprOp::Update
            | ExprOp::Delete
            | ExprOp::Upsert
            | ExprOp::Exists => {
                w.write_varint_u32(self.a)?;
            }
            // aux + a + b — Bin(BinOp, lhs, rhs).
            ExprOp::Bin => {
                w.write_varint_u32(self.aux as u32)?;
                w.write_varint_u32(self.a)?;
                w.write_varint_u32(self.b)?;
            }
            // aux + a — Una(UnaryOp, operand).
            ExprOp::Una => {
                w.write_varint_u32(self.aux as u32)?;
                w.write_varint_u32(self.a)?;
            }
            // flags + a + b — Agg(distinct, func StrId, expr NodeId).
            ExprOp::Agg => {
                w.write_varint_u32(self.flags as u32)?;
                w.write_varint_u32(self.a)?;
                w.write_varint_u32(self.b)?;
            }
            // a + b — Cast(expr, to), Alias(expr, name), InSub(expr, sub).
            ExprOp::Cast | ExprOp::Alias | ExprOp::InSub => {
                w.write_varint_u32(self.a)?;
                w.write_varint_u32(self.b)?;
            }
            // a + b + c — Between(expr, lo, hi).
            ExprOp::Between => {
                w.write_varint_u32(self.a)?;
                w.write_varint_u32(self.b)?;
                w.write_varint_u32(self.c)?;
            }
            // ExprOp is `#[non_exhaustive]`; future opcodes added in
            // `dol-expr` must extend this match before they roundtrip.
            _ => return Err(EncodeError::Custom("ExprNode: unsupported opcode")),
        }
        Ok(())
    }
}

// ─── Interner ────────────────────────────────────────────────────────────────

impl Encode for Interner {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        let strings = self.sorted_strings();
        let len: u32 = strings
            .len()
            .try_into()
            .map_err(|_| EncodeError::LengthOverflow)?;
        w.write_varint_u32(len)?;
        for s in &strings {
            b.descend(|b| s.encode(w, b))??;
        }
        Ok(())
    }
}

// ─── ExprArena ───────────────────────────────────────────────────────────────

impl Encode for ExprArena {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        // Write all pools in the same order as decode.
        b.descend(|b| encode_slice(self.nodes_slice(), w, b))??;
        b.descend(|b| self.span_table_ref().encode(w, b))??;
        b.descend(|b| encode_slice(self.lits_slice(), w, b))??;
        b.descend(|b| encode_slice(self.funcs_slice(), w, b))??;
        b.descend(|b| encode_slice(self.obj_lits_slice(), w, b))??;
        b.descend(|b| encode_slice(self.array_lits_slice(), w, b))??;
        b.descend(|b| encode_slice(self.windows_slice(), w, b))??;
        b.descend(|b| encode_slice(self.cases_slice(), w, b))??;
        b.descend(|b| encode_slice(self.in_lists_slice(), w, b))??;
        b.descend(|b| encode_slice(self.queries_slice(), w, b))??;
        b.descend(|b| encode_slice(self.inserts_slice(), w, b))??;
        b.descend(|b| encode_slice(self.updates_slice(), w, b))??;
        b.descend(|b| encode_slice(self.deletes_slice(), w, b))??;
        b.descend(|b| encode_slice(self.upserts_slice(), w, b))??;
        b.descend(|b| encode_slice(self.fields_slice(), w, b))??;
        Ok(())
    }
}

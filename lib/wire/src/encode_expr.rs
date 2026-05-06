//! [`Encode`] impls for `dol-expr` types.

extern crate alloc;

use dol_core::policy::Budget;
use dol_expr::arena::{CaseNode, ExprArena, FieldNode, FieldStep, FuncNode, InListNode, ObjLitNode, Span, SpanTable, WindowNode};
use dol_expr::expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, InsertNode, JoinNode, JoinType, LockHint, Order,
    QueryNode, UnaryOp, UpdateNode, UpsertNode,
};
use dol_expr::ids::{
    CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NodeId, ObjLitId, QueryId,
    UpdateId, UpsertId, WindowId,
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
            BinOp::Arrow => 22,
            BinOp::LongArrow => 23,
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

// ─── ExprNode ────────────────────────────────────────────────────────────────

impl Encode for ExprNode {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            ExprNode::Namespace(id) => {
                w.write_varint_u32(0)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Field(id) => {
                w.write_varint_u32(1)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Param => {
                w.write_varint_u32(2)?;
            }
            ExprNode::Lit(id) => {
                w.write_varint_u32(3)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::ObjectLit(id) => {
                w.write_varint_u32(4)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::ArrayLit(v) => {
                w.write_varint_u32(5)?;
                b.descend(|b| v.encode(w, b))??;
            }
            ExprNode::BinOp { op, lhs, rhs } => {
                w.write_varint_u32(6)?;
                b.descend(|b| op.encode(w, b))??;
                b.descend(|b| lhs.encode(w, b))??;
                b.descend(|b| rhs.encode(w, b))??;
            }
            ExprNode::UnaryOp { op, operand } => {
                w.write_varint_u32(7)?;
                b.descend(|b| op.encode(w, b))??;
                b.descend(|b| operand.encode(w, b))??;
            }
            ExprNode::Func(id) => {
                w.write_varint_u32(8)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Agg { func, expr, distinct } => {
                w.write_varint_u32(9)?;
                b.descend(|b| func.encode(w, b))??;
                b.descend(|b| expr.encode(w, b))??;
                b.descend(|b| distinct.encode(w, b))??;
            }
            ExprNode::Window(id) => {
                w.write_varint_u32(10)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Cast { expr, to } => {
                w.write_varint_u32(11)?;
                b.descend(|b| expr.encode(w, b))??;
                b.descend(|b| to.encode(w, b))??;
            }
            ExprNode::Case(id) => {
                w.write_varint_u32(12)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Alias { expr, name } => {
                w.write_varint_u32(13)?;
                b.descend(|b| expr.encode(w, b))??;
                b.descend(|b| name.encode(w, b))??;
            }
            ExprNode::InList(id) => {
                w.write_varint_u32(14)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::InSub { expr, sub } => {
                w.write_varint_u32(15)?;
                b.descend(|b| expr.encode(w, b))??;
                b.descend(|b| sub.encode(w, b))??;
            }
            ExprNode::Exists { sub } => {
                w.write_varint_u32(16)?;
                b.descend(|b| sub.encode(w, b))??;
            }
            ExprNode::IsNull { expr } => {
                w.write_varint_u32(17)?;
                b.descend(|b| expr.encode(w, b))??;
            }
            ExprNode::Between { expr, lo, hi } => {
                w.write_varint_u32(18)?;
                b.descend(|b| expr.encode(w, b))??;
                b.descend(|b| lo.encode(w, b))??;
                b.descend(|b| hi.encode(w, b))??;
            }
            ExprNode::Query(id) => {
                w.write_varint_u32(19)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Insert(id) => {
                w.write_varint_u32(20)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Update(id) => {
                w.write_varint_u32(21)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Delete(id) => {
                w.write_varint_u32(22)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ExprNode::Upsert(id) => {
                w.write_varint_u32(23)?;
                b.descend(|b| id.encode(w, b))??;
            }
            _ => return Err(EncodeError::Custom("ExprNode: unknown variant")),
        }
        Ok(())
    }
}

// ─── Interner ────────────────────────────────────────────────────────────────

impl Encode for Interner {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        let strings = self.sorted_strings();
        let len: u32 = strings.len().try_into().map_err(|_| EncodeError::LengthOverflow)?;
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

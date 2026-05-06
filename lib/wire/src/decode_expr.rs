//! [`Decode`] impls for `dol-expr` types.

extern crate alloc;

use alloc::vec::Vec;

use dol_core::policy::Budget;
use dol_expr::arena::{
    CaseNode, ExprArena, FieldNode, FieldStep, FuncNode, InListNode, ObjLitNode, Span, SpanTable,
    WindowNode,
};
use dol_expr::expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, InsertNode, JoinNode, JoinType, LockHint, Order,
    QueryNode, UnaryOp, UpdateNode, UpsertNode,
};
use dol_expr::ids::{
    CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NodeId, ObjLitId, QueryId,
    StrId, UpdateId, UpsertId, WindowId,
};
use dol_expr::interner::Interner;
use dol_expr::tree::window::{FrameBound, FrameKind, WindowFrame};

use crate::decoder::{Decode, DecodeError, Reader};

// ─── StrId ───────────────────────────────────────────────────────────────────

// ─── FieldStep ───────────────────────────────────────────────────────────────

impl Decode for FieldStep {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(FieldStep::Key(
                budget.descend(|b| StrId::decode(reader, b))??,
            )),
            1 => Ok(FieldStep::Index(reader.read_varint_u32()?)),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "FieldStep",
                seen,
            }),
        }
    }
}

// ─── FieldNode ───────────────────────────────────────────────────────────────

impl Decode for FieldNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let namespace = budget.descend(|b| Option::<StrId>::decode(reader, b))??;
        let name = budget.descend(|b| StrId::decode(reader, b))??;
        let steps =
            budget.descend(|b| smallvec::SmallVec::<[FieldStep; 4]>::decode(reader, b))??;
        Ok(FieldNode {
            namespace,
            name,
            steps,
        })
    }
}

// ─── FuncNode ────────────────────────────────────────────────────────────────

impl Decode for FuncNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let name = budget.descend(|b| StrId::decode(reader, b))??;
        let args = budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        Ok(FuncNode { name, args })
    }
}

// ─── ObjLitNode ──────────────────────────────────────────────────────────────

impl Decode for ObjLitNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let inner =
            budget.descend(|b| smallvec::SmallVec::<[(StrId, NodeId); 4]>::decode(reader, b))??;
        Ok(ObjLitNode(inner))
    }
}

// ─── Order ───────────────────────────────────────────────────────────────────

impl Decode for Order {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(Order::Asc),
            1 => Ok(Order::Desc),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "Order",
                seen,
            }),
        }
    }
}

// ─── LockHint ────────────────────────────────────────────────────────────────

impl Decode for LockHint {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(LockHint::ForUpdate),
            1 => Ok(LockHint::ForShare),
            2 => Ok(LockHint::SkipLocked),
            3 => Ok(LockHint::NoWait),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "LockHint",
                seen,
            }),
        }
    }
}

// ─── FrameBound ──────────────────────────────────────────────────────────────

impl Decode for FrameBound {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(FrameBound::UnboundedPreceding),
            1 => Ok(FrameBound::Preceding(reader.read_varint_u32()?)),
            2 => Ok(FrameBound::CurrentRow),
            3 => Ok(FrameBound::Following(reader.read_varint_u32()?)),
            4 => Ok(FrameBound::UnboundedFollowing),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "FrameBound",
                seen,
            }),
        }
    }
}

// ─── FrameKind ───────────────────────────────────────────────────────────────

impl Decode for FrameKind {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(FrameKind::Rows),
            1 => Ok(FrameKind::Range),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "FrameKind",
                seen,
            }),
        }
    }
}

// ─── WindowFrame ─────────────────────────────────────────────────────────────

impl Decode for WindowFrame {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let kind = budget.descend(|b| FrameKind::decode(reader, b))??;
        let start = budget.descend(|b| FrameBound::decode(reader, b))??;
        let end = budget.descend(|b| Option::<FrameBound>::decode(reader, b))??;
        Ok(WindowFrame { kind, start, end })
    }
}

// ─── WindowNode ──────────────────────────────────────────────────────────────

impl Decode for WindowNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let func = budget.descend(|b| StrId::decode(reader, b))??;
        let partition =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        let order =
            budget.descend(|b| smallvec::SmallVec::<[(NodeId, Order); 2]>::decode(reader, b))??;
        let frame = budget.descend(|b| Option::<WindowFrame>::decode(reader, b))??;
        Ok(WindowNode {
            func,
            partition,
            order,
            frame,
        })
    }
}

// ─── CaseNode ────────────────────────────────────────────────────────────────

impl Decode for CaseNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let branches = budget
            .descend(|b| smallvec::SmallVec::<[(NodeId, NodeId); 4]>::decode(reader, b))??;
        let else_ = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(CaseNode { branches, else_ })
    }
}

// ─── InListNode ──────────────────────────────────────────────────────────────

impl Decode for InListNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let expr = budget.descend(|b| NodeId::decode(reader, b))??;
        let list = budget.descend(|b| smallvec::SmallVec::<[NodeId; 8]>::decode(reader, b))??;
        Ok(InListNode { expr, list })
    }
}

// ─── Span / SpanTable ────────────────────────────────────────────────────────

impl Decode for Span {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        let start = reader.read_varint_u32()?;
        let end = reader.read_varint_u32()?;
        Ok(Span { start, end })
    }
}

impl Decode for SpanTable {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let spans = budget.descend(|b| Vec::<Span>::decode(reader, b))??;
        let owners = budget.descend(|b| Vec::<NodeId>::decode(reader, b))??;
        let mut table = SpanTable::default();
        for (owner, span) in owners.into_iter().zip(spans.into_iter()) {
            table.push(owner, span);
        }
        Ok(table)
    }
}

// ─── JoinType ────────────────────────────────────────────────────────────────

impl Decode for JoinType {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(JoinType::Inner),
            1 => Ok(JoinType::Left),
            2 => Ok(JoinType::Right),
            3 => Ok(JoinType::Full),
            4 => Ok(JoinType::Cross),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "JoinType",
                seen,
            }),
        }
    }
}

// ─── JoinNode ────────────────────────────────────────────────────────────────

impl Decode for JoinNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let source = budget.descend(|b| StrId::decode(reader, b))??;
        let alias = budget.descend(|b| Option::<StrId>::decode(reader, b))??;
        let join_type = budget.descend(|b| JoinType::decode(reader, b))??;
        let on = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(JoinNode {
            source,
            alias,
            join_type,
            on,
        })
    }
}

// ─── QueryNode ───────────────────────────────────────────────────────────────

impl Decode for QueryNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let from = budget.descend(|b| StrId::decode(reader, b))??;
        let alias = budget.descend(|b| Option::<StrId>::decode(reader, b))??;
        let joins =
            budget.descend(|b| smallvec::SmallVec::<[JoinNode; 2]>::decode(reader, b))??;
        let filter = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let columns =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 8]>::decode(reader, b))??;
        let group_by =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        let having = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let order_by =
            budget.descend(|b| smallvec::SmallVec::<[(NodeId, Order); 4]>::decode(reader, b))??;
        let limit = budget.descend(|b| Option::<u64>::decode(reader, b))??;
        let offset = budget.descend(|b| Option::<u64>::decode(reader, b))??;
        let lock = budget.descend(|b| Option::<LockHint>::decode(reader, b))??;
        Ok(QueryNode {
            from,
            alias,
            joins,
            filter,
            columns,
            group_by,
            having,
            order_by,
            limit,
            offset,
            lock,
        })
    }
}

// ─── ConflictClause ──────────────────────────────────────────────────────────

impl Decode for ConflictClause {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(ConflictClause::DoNothing),
            1 => {
                let assignments = budget
                    .descend(|b| smallvec::SmallVec::<[(StrId, NodeId); 4]>::decode(reader, b))??;
                Ok(ConflictClause::DoUpdate { assignments })
            }
            seen => Err(DecodeError::InvalidVariant {
                type_name: "ConflictClause",
                seen,
            }),
        }
    }
}

// ─── InsertNode ──────────────────────────────────────────────────────────────

impl Decode for InsertNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| StrId::decode(reader, b))??;
        let columns = budget.descend(|b| smallvec::SmallVec::<[StrId; 8]>::decode(reader, b))??;
        let values = budget.descend(|b| smallvec::SmallVec::<[NodeId; 8]>::decode(reader, b))??;
        let returning =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        let conflict = budget.descend(|b| Option::<ConflictClause>::decode(reader, b))??;
        Ok(InsertNode {
            target,
            columns,
            values,
            returning,
            conflict,
        })
    }
}

// ─── UpdateNode ──────────────────────────────────────────────────────────────

impl Decode for UpdateNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| StrId::decode(reader, b))??;
        let columns = budget.descend(|b| smallvec::SmallVec::<[StrId; 8]>::decode(reader, b))??;
        let values = budget.descend(|b| smallvec::SmallVec::<[NodeId; 8]>::decode(reader, b))??;
        let filter = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let returning =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        Ok(UpdateNode {
            target,
            columns,
            values,
            filter,
            returning,
        })
    }
}

// ─── DeleteNode ──────────────────────────────────────────────────────────────

impl Decode for DeleteNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| StrId::decode(reader, b))??;
        let filter = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let returning =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        Ok(DeleteNode {
            target,
            filter,
            returning,
        })
    }
}

// ─── UpsertNode ──────────────────────────────────────────────────────────────

impl Decode for UpsertNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| StrId::decode(reader, b))??;
        let columns = budget.descend(|b| smallvec::SmallVec::<[StrId; 8]>::decode(reader, b))??;
        let values = budget.descend(|b| smallvec::SmallVec::<[NodeId; 8]>::decode(reader, b))??;
        let returning =
            budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        let conflict = budget.descend(|b| Option::<ConflictClause>::decode(reader, b))??;
        Ok(UpsertNode {
            target,
            columns,
            values,
            returning,
            conflict,
        })
    }
}

// ─── BinOp ───────────────────────────────────────────────────────────────────

impl Decode for BinOp {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(BinOp::Eq),
            1 => Ok(BinOp::Ne),
            2 => Ok(BinOp::Lt),
            3 => Ok(BinOp::Le),
            4 => Ok(BinOp::Gt),
            5 => Ok(BinOp::Ge),
            6 => Ok(BinOp::And),
            7 => Ok(BinOp::Or),
            8 => Ok(BinOp::Add),
            9 => Ok(BinOp::Sub),
            10 => Ok(BinOp::Mul),
            11 => Ok(BinOp::Div),
            12 => Ok(BinOp::Rem),
            13 => Ok(BinOp::Like),
            14 => Ok(BinOp::ILike),
            15 => Ok(BinOp::Similar),
            16 => Ok(BinOp::BitAnd),
            17 => Ok(BinOp::BitOr),
            18 => Ok(BinOp::BitXor),
            19 => Ok(BinOp::Shl),
            20 => Ok(BinOp::Shr),
            21 => Ok(BinOp::Concat),
            22 => Ok(BinOp::Arrow),
            23 => Ok(BinOp::LongArrow),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "BinOp",
                seen,
            }),
        }
    }
}

// ─── UnaryOp ─────────────────────────────────────────────────────────────────

impl Decode for UnaryOp {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(UnaryOp::Neg),
            1 => Ok(UnaryOp::Not),
            2 => Ok(UnaryOp::BitNot),
            3 => Ok(UnaryOp::IsNull),
            4 => Ok(UnaryOp::IsNotNull),
            5 => Ok(UnaryOp::IsTrue),
            6 => Ok(UnaryOp::IsFalse),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "UnaryOp",
                seen,
            }),
        }
    }
}

// ─── ExprNode ────────────────────────────────────────────────────────────────

impl Decode for ExprNode {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let id = budget.descend(|b| StrId::decode(reader, b))??;
                Ok(ExprNode::Namespace(id))
            }
            1 => {
                let id = budget.descend(|b| FieldId::decode(reader, b))??;
                Ok(ExprNode::Field(id))
            }
            2 => Ok(ExprNode::Param),
            3 => {
                let id = budget.descend(|b| LiteralId::decode(reader, b))??;
                Ok(ExprNode::Lit(id))
            }
            4 => {
                let id = budget.descend(|b| ObjLitId::decode(reader, b))??;
                Ok(ExprNode::ObjectLit(id))
            }
            5 => {
                let v =
                    budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
                Ok(ExprNode::ArrayLit(v))
            }
            6 => {
                let op = budget.descend(|b| BinOp::decode(reader, b))??;
                let lhs = budget.descend(|b| NodeId::decode(reader, b))??;
                let rhs = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ExprNode::BinOp { op, lhs, rhs })
            }
            7 => {
                let op = budget.descend(|b| UnaryOp::decode(reader, b))??;
                let operand = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ExprNode::UnaryOp { op, operand })
            }
            8 => {
                let id = budget.descend(|b| FuncId::decode(reader, b))??;
                Ok(ExprNode::Func(id))
            }
            9 => {
                let func = budget.descend(|b| StrId::decode(reader, b))??;
                let expr = budget.descend(|b| NodeId::decode(reader, b))??;
                let distinct = budget.descend(|b| bool::decode(reader, b))??;
                Ok(ExprNode::Agg {
                    func,
                    expr,
                    distinct,
                })
            }
            10 => {
                let id = budget.descend(|b| WindowId::decode(reader, b))??;
                Ok(ExprNode::Window(id))
            }
            11 => {
                let expr = budget.descend(|b| NodeId::decode(reader, b))??;
                let to = budget.descend(|b| StrId::decode(reader, b))??;
                Ok(ExprNode::Cast { expr, to })
            }
            12 => {
                let id = budget.descend(|b| CaseId::decode(reader, b))??;
                Ok(ExprNode::Case(id))
            }
            13 => {
                let expr = budget.descend(|b| NodeId::decode(reader, b))??;
                let name = budget.descend(|b| StrId::decode(reader, b))??;
                Ok(ExprNode::Alias { expr, name })
            }
            14 => {
                let id = budget.descend(|b| InListId::decode(reader, b))??;
                Ok(ExprNode::InList(id))
            }
            15 => {
                let expr = budget.descend(|b| NodeId::decode(reader, b))??;
                let sub = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ExprNode::InSub { expr, sub })
            }
            16 => {
                let sub = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ExprNode::Exists { sub })
            }
            17 => {
                let expr = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ExprNode::IsNull { expr })
            }
            18 => {
                let expr = budget.descend(|b| NodeId::decode(reader, b))??;
                let lo = budget.descend(|b| NodeId::decode(reader, b))??;
                let hi = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ExprNode::Between { expr, lo, hi })
            }
            19 => {
                let id = budget.descend(|b| QueryId::decode(reader, b))??;
                Ok(ExprNode::Query(id))
            }
            20 => {
                let id = budget.descend(|b| InsertId::decode(reader, b))??;
                Ok(ExprNode::Insert(id))
            }
            21 => {
                let id = budget.descend(|b| UpdateId::decode(reader, b))??;
                Ok(ExprNode::Update(id))
            }
            22 => {
                let id = budget.descend(|b| DeleteId::decode(reader, b))??;
                Ok(ExprNode::Delete(id))
            }
            23 => {
                let id = budget.descend(|b| UpsertId::decode(reader, b))??;
                Ok(ExprNode::Upsert(id))
            }
            seen => Err(DecodeError::InvalidVariant {
                type_name: "ExprNode",
                seen,
            }),
        }
    }
}

// ─── Interner ────────────────────────────────────────────────────────────────

impl Decode for Interner {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let strings = budget.descend(|b| Vec::<alloc::string::String>::decode(reader, b))??;
        let mut interner = Interner::default();
        for s in &strings {
            interner
                .try_intern(s)
                .map_err(|_| DecodeError::Custom("interner collision"))?;
        }
        Ok(interner)
    }
}

// ─── ExprArena ───────────────────────────────────────────────────────────────

impl Decode for ExprArena {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let mut arena = ExprArena::new();

        // 1. nodes
        let nodes = budget.descend(|b| Vec::<ExprNode>::decode(reader, b))??;
        for node in nodes {
            arena.alloc(node);
        }

        // 2. span_table
        let span_table = budget.descend(|b| SpanTable::decode(reader, b))??;
        // We need to push spans into the arena's span_table. Since ExprArena
        // exposes attach_span (which calls span_table.push), we iterate.
        // But we decoded a complete SpanTable — we need to push each entry.
        // Access via attach_span which calls push(owner, span).
        // We need to iterate over the decoded table's entries.
        // Since we have spans_slice and owners_slice, do it manually:
        for (owner, span) in span_table
            .owners_slice()
            .iter()
            .zip(span_table.spans_slice().iter())
        {
            arena.attach_span(*owner, span.clone());
        }

        // 3. lits
        let lits = budget.descend(|b| Vec::<dol_core::Literal<'static>>::decode(reader, b))??;
        for lit in lits {
            arena.alloc_lit(lit);
        }

        // 4. funcs
        let funcs = budget.descend(|b| Vec::<FuncNode>::decode(reader, b))??;
        for func in funcs {
            arena.alloc_func(func);
        }

        // 5. obj_lits
        let obj_lits = budget.descend(|b| Vec::<ObjLitNode>::decode(reader, b))??;
        for obj in obj_lits {
            arena.alloc_obj_lit(obj);
        }

        // 6. windows
        let windows = budget.descend(|b| Vec::<WindowNode>::decode(reader, b))??;
        for win in windows {
            arena.alloc_window(win);
        }

        // 7. cases
        let cases = budget.descend(|b| Vec::<CaseNode>::decode(reader, b))??;
        for case in cases {
            arena.alloc_case(case);
        }

        // 8. in_lists
        let in_lists = budget.descend(|b| Vec::<InListNode>::decode(reader, b))??;
        for il in in_lists {
            arena.alloc_in_list(il);
        }

        // 9. queries
        let queries = budget.descend(|b| Vec::<QueryNode>::decode(reader, b))??;
        for q in queries {
            arena.alloc_query(q);
        }

        // 10. inserts
        let inserts = budget.descend(|b| Vec::<InsertNode>::decode(reader, b))??;
        for ins in inserts {
            arena.alloc_insert(ins);
        }

        // 11. updates
        let updates = budget.descend(|b| Vec::<UpdateNode>::decode(reader, b))??;
        for upd in updates {
            arena.alloc_update(upd);
        }

        // 12. deletes
        let deletes = budget.descend(|b| Vec::<DeleteNode>::decode(reader, b))??;
        for del in deletes {
            arena.alloc_delete(del);
        }

        // 13. upserts
        let upserts = budget.descend(|b| Vec::<UpsertNode>::decode(reader, b))??;
        for ups in upserts {
            arena.alloc_upsert(ups);
        }

        // 14. fields
        let fields = budget.descend(|b| Vec::<FieldNode>::decode(reader, b))??;
        for field in fields {
            arena.alloc_field(field);
        }

        Ok(arena)
    }
}

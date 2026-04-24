use smallvec::SmallVec;

use crate::ids::{CaseId, FuncId, InListId, LiteralId, MutateId, NodeId, ObjLitId, SelectId, StrId, WindowId};

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Add, Sub, Mul, Div, Rem,
    Like, ILike, Similar,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Concat,
    Arrow, LongArrow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg, Not, IsNull, IsNotNull, IsTrue, IsFalse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LockHint {
    ForUpdate,
    ForShare,
    SkipLocked,
    NoWait,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictClause {
    DoNothing,
    DoUpdate { assignments: SmallVec<[(StrId, NodeId); 4]> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinNode {
    pub source:    StrId,
    pub alias:     Option<StrId>,
    pub join_type: JoinType,
    pub on:        NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner, Left, Right, Full, Cross,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectNode {
    pub from:     StrId,
    pub alias:    Option<StrId>,
    pub joins:    SmallVec<[JoinNode; 2]>,
    pub filter:   NodeId,
    pub columns:  SmallVec<[NodeId; 8]>,
    pub group_by: SmallVec<[NodeId; 4]>,
    pub having:   NodeId,
    pub order_by: SmallVec<[(NodeId, Order); 4]>,
    pub limit:    Option<u64>,
    pub offset:   Option<u64>,
    pub lock:     Option<LockHint>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MutateKind {
    Insert,
    Update,
    Delete,
    Upsert,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutateNode {
    pub kind:      MutateKind,
    pub target:    StrId,
    pub columns:   SmallVec<[StrId; 8]>,
    pub values:    SmallVec<[NodeId; 8]>,
    pub filter:    NodeId,
    pub returning: SmallVec<[NodeId; 4]>,
    pub conflict:  Option<ConflictClause>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprNode {
    Field(StrId),
    QualifiedField { source: StrId, field: StrId },
    Param,
    /// A literal constant. The actual [`Literal`] is stored in `ExprArena::lits`;
    /// this variant holds only the pool index.
    ///
    /// [`Literal`]: crate::types::value::Literal
    Lit(LiteralId),
    /// An object literal. The field-pair list is stored in `ExprArena::obj_lits`;
    /// this variant holds only the pool index.
    ObjectLit(ObjLitId),
    ArrayLit(SmallVec<[NodeId; 4]>),
    BinOp   { op: BinOp,   lhs: NodeId, rhs: NodeId },
    UnaryOp { op: UnaryOp, operand: NodeId },
    /// A function call. The name and argument list are stored in `ExprArena::funcs`;
    /// this variant holds only the pool index.
    Func(FuncId),
    Agg     { func: StrId, expr: NodeId, distinct: bool },
    /// A window function. The payload is stored in `ExprArena::windows`;
    /// this variant holds only the pool index.
    Window(WindowId),
    Cast    { expr: NodeId, to: StrId },
    /// A `CASE WHEN … THEN … ELSE … END` expression. The payload is stored in
    /// `ExprArena::cases`; this variant holds only the pool index.
    Case(CaseId),
    Alias   { expr: NodeId, name: StrId },
    /// An `expr IN (list)` expression. The payload is stored in
    /// `ExprArena::in_lists`; this variant holds only the pool index.
    InList(InListId),
    InSub   { expr: NodeId, sub: NodeId },
    Exists  { sub: NodeId },
    IsNull  { expr: NodeId },
    Between { expr: NodeId, lo: NodeId, hi: NodeId },
    /// A SELECT sub-query. The payload is stored in `ExprArena::selects`;
    /// this variant holds only the pool index.
    Select(SelectId),
    /// A mutating statement (INSERT/UPDATE/DELETE/UPSERT). The payload is
    /// stored in `ExprArena::mutates`; this variant holds only the pool index.
    Mutate(MutateId),
}

#[cfg(test)]
mod size_tests {
    use std::mem::size_of;

    use super::ExprNode;
    use crate::types::value::{Literal, Value};

    #[test]
    fn expr_node_fits_32_bytes() {
        let sz = size_of::<ExprNode>();
        assert!(
            sz <= 32,
            "ExprNode is {sz} bytes on this target — must be ≤ 32; \
             pool a large variant via ExprArena",
        );
    }

    #[test]
    fn value_fits_24_bytes() {
        let sz = size_of::<Value>();
        assert!(
            sz <= 24,
            "Value is {sz} bytes — must be ≤ 24",
        );
    }

    #[test]
    fn literal_static_fits_32_bytes() {
        let sz = size_of::<Literal<'static>>();
        assert!(
            sz <= 32,
            "Literal<'static> is {sz} bytes — must be ≤ 32",
        );
    }
}

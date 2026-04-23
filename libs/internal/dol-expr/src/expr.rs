use smallvec::SmallVec;

use crate::ids::{NodeId, StrId};
use crate::types::value::Literal;

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
    Lit(Literal<'static>),
    ObjectLit(SmallVec<[(StrId, NodeId); 4]>),
    ArrayLit(SmallVec<[NodeId; 4]>),
    BinOp   { op: BinOp,   lhs: NodeId, rhs: NodeId },
    UnaryOp { op: UnaryOp, operand: NodeId },
    Func    { name: StrId, args: SmallVec<[NodeId; 4]> },
    Agg     { func: StrId, expr: NodeId, distinct: bool },
    Window  { func: StrId, partition: SmallVec<[NodeId; 4]>, order: SmallVec<[(NodeId, Order); 2]> },
    Cast    { expr: NodeId, to: StrId },
    Case    { branches: SmallVec<[(NodeId, NodeId); 4]>, else_: NodeId },
    Alias   { expr: NodeId, name: StrId },
    InList  { expr: NodeId, list: SmallVec<[NodeId; 8]> },
    InSub   { expr: NodeId, sub: NodeId },
    Exists  { sub: NodeId },
    IsNull  { expr: NodeId },
    Between { expr: NodeId, lo: NodeId, hi: NodeId },
    Select(SelectNode),
    Mutate(MutateNode),
}

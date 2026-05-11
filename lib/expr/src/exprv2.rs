use bytemuck::{Pod, Zeroable};
use smallvec::SmallVec;

use dol_core::strings::StrId;

use crate::ids::{
    CaseId, CompositeId, DeleteId, FieldId, FuncId, InsertId, LiteralId, NodeId, QueryId, UpdateId,
    UpsertId, WindowId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Like,
    ILike,
    Similar,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Concat,
    /// `a IS DISTINCT FROM b` — null-safe inequality. Promoted to
    /// [`BinOp`] from the tree-side `OpDef::IS_DISTINCT_FROM` so it
    /// lowers through the fast path rather than a string-named lookup.
    IsDistinctFrom,
    /// `a IS NOT DISTINCT FROM b` — null-safe equality.
    IsNotDistinctFrom,
}

impl BinOp {
    /// Stable u16 tag used in the wire format and in
    /// [`ExprNode::aux`] for [`ExprOp::Bin`] nodes.
    ///
    /// Tags are append-only; never renumber.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
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
        }
    }

    /// Inverse of [`Self::as_u16`]. Returns `None` for unknown tags so
    /// decoders can surface `DecodeError::InvalidVariant` cleanly.
    #[must_use]
    pub const fn try_from_u16(tag: u16) -> Option<Self> {
        Some(match tag {
            0 => BinOp::Eq,
            1 => BinOp::Ne,
            2 => BinOp::Lt,
            3 => BinOp::Le,
            4 => BinOp::Gt,
            5 => BinOp::Ge,
            6 => BinOp::And,
            7 => BinOp::Or,
            8 => BinOp::Add,
            9 => BinOp::Sub,
            10 => BinOp::Mul,
            11 => BinOp::Div,
            12 => BinOp::Rem,
            13 => BinOp::Like,
            14 => BinOp::ILike,
            15 => BinOp::Similar,
            16 => BinOp::BitAnd,
            17 => BinOp::BitOr,
            18 => BinOp::BitXor,
            19 => BinOp::Shl,
            20 => BinOp::Shr,
            21 => BinOp::Concat,
            22 => BinOp::IsDistinctFrom,
            23 => BinOp::IsNotDistinctFrom,
            _ => return None,
        })
    }
}

/// Unary operator menu. Stored in [`ExprNode::aux`] for [`ExprOp::Una`]
/// nodes; same `as_u16` / `try_from_u16` round-trip discipline as
/// [`BinOp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum UnaryOp {
    Neg,
    Not,
    /// Bitwise NOT (one's complement) on integer-typed operands. Distinct
    /// from `Not` (logical) so backends and lowerers cannot conflate the
    /// two on integer expressions.
    BitNot,
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse,
}

impl UnaryOp {
    /// Stable u16 tag used in the wire format and in
    /// [`ExprNode::aux`] for [`ExprOp::Una`] nodes.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
            UnaryOp::Neg => 0,
            UnaryOp::Not => 1,
            UnaryOp::BitNot => 2,
            UnaryOp::IsNull => 3,
            UnaryOp::IsNotNull => 4,
            UnaryOp::IsTrue => 5,
            UnaryOp::IsFalse => 6,
        }
    }

    /// Inverse of [`Self::as_u16`].
    #[must_use]
    pub const fn try_from_u16(tag: u16) -> Option<Self> {
        Some(match tag {
            0 => UnaryOp::Neg,
            1 => UnaryOp::Not,
            2 => UnaryOp::BitNot,
            3 => UnaryOp::IsNull,
            4 => UnaryOp::IsNotNull,
            5 => UnaryOp::IsTrue,
            6 => UnaryOp::IsFalse,
            _ => return None,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ExprOp + ExprNode (the packed POD core)
// ═══════════════════════════════════════════════════════════════════════════

/// Opcode discriminator for [`ExprNode`].
///
/// The variants intentionally collapse families (every binary operator
/// shares [`ExprOp::Bin`], every unary operator shares [`ExprOp::Una`])
/// and use [`ExprNode::aux`] for the sub-opcode. This keeps the
/// discriminant space small (≤ 256) while the operator menu stays
/// expressive.
///
/// Tag values are **stable** and append-only — wire format compatibility
/// depends on it. Always extend by adding a new tag at the end.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum ExprOp {
    /// Empty / sentinel. `bytemuck::Zeroable` lands here.
    Nop = 0,
    /// Static container address. `a` is a [`StrId`] for the dotted
    /// container path (e.g. `"public.users"`).
    Namespace = 1,
    /// Field reference. `a` is a [`FieldId`] into `ExprArena::fields`.
    Field = 2,
    /// Parameter / placeholder. `a` is the (optional) parameter index;
    /// `0` means "unindexed" (matching today's positional semantics).
    Param = 3,
    /// Pooled literal. `a` is a [`LiteralId`] into `ExprArena::lits`.
    Lit = 4,
    /// Pooled composite (array / object / tuple) literal. `a` is a
    /// [`CompositeId`] into `ExprArena::composites`. The `kind`
    /// (Array / Object / Tuple) lives on the side-pool record.
    ///
    /// Collapses the v2-prototype `ObjectLit` and `ArrayLit` opcodes
    /// onto a single structural slot, with `Tuple` added to back the
    /// row form of `IN (a, b, c)`.
    Composite = 5,
    /// Binary operator. `aux` carries [`BinOp::as_u16`]; `a`/`b` are
    /// the operand [`NodeId`]s.
    Bin = 6,
    /// Unary operator. `aux` carries [`UnaryOp::as_u16`]; `a` is the
    /// operand [`NodeId`].
    Una = 7,
    /// Function call. `a` is a [`FuncId`] into `ExprArena::funcs`.
    Func = 8,
    /// Aggregate. `flags` bit 0 is `distinct`; `a` is the aggregate
    /// function name [`StrId`]; `b` is the operand [`NodeId`].
    Agg = 9,
    /// Window function. `a` is a [`WindowId`] into `ExprArena::windows`.
    Window = 10,
    /// `CAST(expr AS to)`. `a` is the operand [`NodeId`]; `b` is the
    /// target-type-name [`StrId`].
    Cast = 11,
    /// `CASE WHEN …`. `a` is a [`CaseId`] into `ExprArena::cases`.
    Case = 12,
    /// `expr AS name`. `a` is the inner [`NodeId`]; `b` is the alias
    /// [`StrId`].
    Alias = 13,
    /// `probe IN collection`. `a` is the probe [`NodeId`]; `b` is the
    /// collection [`NodeId`]. The collection's own opcode discriminates
    /// the form: a [`Self::Composite`] (array / tuple), a
    /// [`Self::Query`] (subquery), a [`Self::Param`] (parameter), or
    /// a [`Self::Field`] (field-of-array). Replaces the v2-prototype
    /// `InList` and `InSub` opcodes.
    In = 14,
    /// `EXISTS(subquery)`. `a` is the subquery [`NodeId`].
    Exists = 15,
    /// `expr BETWEEN lo AND hi`. `a`/`b`/`c` are the three operand
    /// [`NodeId`]s.
    Between = 16,
    /// SELECT subquery. `a` is a [`QueryId`] into `ExprArena::queries`.
    Query = 17,
    /// INSERT statement. `a` is an [`InsertId`] into `ExprArena::inserts`.
    Insert = 18,
    /// UPDATE statement. `a` is an [`UpdateId`] into `ExprArena::updates`.
    Update = 19,
    /// DELETE statement. `a` is a [`DeleteId`] into `ExprArena::deletes`.
    Delete = 20,
    /// UPSERT statement. `a` is an [`UpsertId`] into `ExprArena::upserts`.
    Upsert = 21,
}

impl ExprOp {
    /// Convert from a raw `u8` (e.g. [`ExprNode::op`]). Returns `None`
    /// for tags this build doesn't recognise — decoders surface that as
    /// a hard error rather than a silent skip.
    #[must_use]
    pub const fn from_u8(op: u8) -> Option<Self> {
        Some(match op {
            0 => Self::Nop,
            1 => Self::Namespace,
            2 => Self::Field,
            3 => Self::Param,
            4 => Self::Lit,
            5 => Self::Composite,
            6 => Self::Bin,
            7 => Self::Una,
            8 => Self::Func,
            9 => Self::Agg,
            10 => Self::Window,
            11 => Self::Cast,
            12 => Self::Case,
            13 => Self::Alias,
            14 => Self::In,
            15 => Self::Exists,
            16 => Self::Between,
            17 => Self::Query,
            18 => Self::Insert,
            19 => Self::Update,
            20 => Self::Delete,
            21 => Self::Upsert,
            _ => return None,
        })
    }
}

/// 16-byte plain-old-data expression node.
///
/// See the [module docs](self) for the field layout. Construct via the
/// typed constructors ([`ExprNode::bin`], [`ExprNode::field`], …) or
/// inspect via the typed accessors ([`ExprNode::as_bin`],
/// [`ExprNode::as_field`], …); never reach into raw `a`/`b`/`c` from
/// downstream code.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ExprNode {
    /// Opcode tag (cast of [`ExprOp`]). Validate via [`ExprOp::from_u8`].
    pub op: u8,
    /// Small flag bits (opcode-specific).
    pub flags: u8,
    /// 16-bit auxiliary value (sub-opcode for `Bin`/`Una`, short
    /// inline value, …).
    pub aux: u16,
    /// First operand handle. `0` means absent.
    pub a: u32,
    /// Second operand handle. `0` means absent.
    pub b: u32,
    /// Third operand handle. `0` means absent.
    pub c: u32,
}

// Compile-time guarantee: `ExprNode` is exactly 16 bytes. A 4-byte
// alignment matches `[u32; 4]` and is the minimum that lets us
// `bytemuck::cast_slice` into `&[u8]` without surprise padding.
const _: () = {
    assert!(core::mem::size_of::<ExprNode>() == 16);
    assert!(core::mem::align_of::<ExprNode>() == 4);
};

// ═══════════════════════════════════════════════════════════════════════════
// Typed constructors and accessors
// ═══════════════════════════════════════════════════════════════════════════

impl ExprNode {
    /// Raw constructor — prefer the typed builders ([`Self::bin`], …)
    /// in normal code; this exists for decoders.
    #[must_use]
    pub const fn raw(op: ExprOp, flags: u8, aux: u16, a: u32, b: u32, c: u32) -> Self {
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
    pub const fn opcode(&self) -> Option<ExprOp> {
        ExprOp::from_u8(self.op)
    }

    // ── Constructors (one per opcode) ───────────────────────────────────────

    #[must_use]
    pub const fn nop() -> Self {
        Self::raw(ExprOp::Nop, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub const fn namespace(id: StrId) -> Self {
        Self::raw(ExprOp::Namespace, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn field(id: FieldId) -> Self {
        Self::raw(ExprOp::Field, 0, 0, id.get(), 0, 0)
    }

    /// Build a `Param` node. `index = 0` means "unindexed positional"
    /// — the legacy default that matches the previous unit-variant
    /// `ExprNode::Param`.
    #[must_use]
    pub const fn param(index: u32) -> Self {
        Self::raw(ExprOp::Param, 0, 0, index, 0, 0)
    }

    #[must_use]
    pub const fn lit(id: LiteralId) -> Self {
        Self::raw(ExprOp::Lit, 0, 0, id.get(), 0, 0)
    }

    /// Build a `Composite` (array / object / tuple) reference node.
    /// `id` points into `ExprArena::composites`.
    #[must_use]
    pub const fn composite(id: CompositeId) -> Self {
        Self::raw(ExprOp::Composite, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn bin(op: BinOp, lhs: NodeId, rhs: NodeId) -> Self {
        Self::raw(ExprOp::Bin, 0, op.as_u16(), lhs.get(), rhs.get(), 0)
    }

    #[must_use]
    pub const fn una(op: UnaryOp, operand: NodeId) -> Self {
        Self::raw(ExprOp::Una, 0, op.as_u16(), operand.get(), 0, 0)
    }

    #[must_use]
    pub const fn func(id: FuncId) -> Self {
        Self::raw(ExprOp::Func, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn agg(func: StrId, expr: NodeId, distinct: bool) -> Self {
        Self::raw(
            ExprOp::Agg,
            if distinct { 1 } else { 0 },
            0,
            func.get(),
            expr.get(),
            0,
        )
    }

    #[must_use]
    pub const fn window(id: WindowId) -> Self {
        Self::raw(ExprOp::Window, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn cast(expr: NodeId, to: StrId) -> Self {
        Self::raw(ExprOp::Cast, 0, 0, expr.get(), to.get(), 0)
    }

    #[must_use]
    pub const fn case(id: CaseId) -> Self {
        Self::raw(ExprOp::Case, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn alias(expr: NodeId, name: StrId) -> Self {
        Self::raw(ExprOp::Alias, 0, 0, expr.get(), name.get(), 0)
    }

    /// Build an `In` (probe-IN-collection) node. `collection` is a
    /// [`NodeId`] referring to another arena node whose opcode encodes
    /// the form of the right-hand side: a [`ExprOp::Composite`]
    /// (`(a, b, c)` row form), a [`ExprOp::Query`] (`(SELECT …)`
    /// subquery form), a [`ExprOp::Param`] (`= ANY($1)` parameter
    /// form), or a [`ExprOp::Field`] (field-of-array form).
    ///
    /// Replaces the v2-prototype `in_list` (pooled list) and `in_sub`
    /// (subquery NodeId) constructors.
    #[must_use]
    pub const fn in_(probe: NodeId, collection: NodeId) -> Self {
        Self::raw(ExprOp::In, 0, 0, probe.get(), collection.get(), 0)
    }

    #[must_use]
    pub const fn exists(sub: NodeId) -> Self {
        Self::raw(ExprOp::Exists, 0, 0, sub.get(), 0, 0)
    }

    #[must_use]
    pub const fn between(expr: NodeId, lo: NodeId, hi: NodeId) -> Self {
        Self::raw(ExprOp::Between, 0, 0, expr.get(), lo.get(), hi.get())
    }

    #[must_use]
    pub const fn query(id: QueryId) -> Self {
        Self::raw(ExprOp::Query, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn insert(id: InsertId) -> Self {
        Self::raw(ExprOp::Insert, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn update(id: UpdateId) -> Self {
        Self::raw(ExprOp::Update, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn delete(id: DeleteId) -> Self {
        Self::raw(ExprOp::Delete, 0, 0, id.get(), 0, 0)
    }

    #[must_use]
    pub const fn upsert(id: UpsertId) -> Self {
        Self::raw(ExprOp::Upsert, 0, 0, id.get(), 0, 0)
    }

    // ── Accessors (one per opcode) ──────────────────────────────────────────

    /// Extract the [`StrId`] payload of a [`ExprOp::Namespace`] node.
    #[must_use]
    pub fn as_namespace(&self) -> Option<StrId> {
        if self.opcode()? == ExprOp::Namespace {
            StrId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract the [`FieldId`] payload of a [`ExprOp::Field`] node.
    #[must_use]
    pub fn as_field(&self) -> Option<FieldId> {
        if self.opcode()? == ExprOp::Field {
            FieldId::from_u32(self.a)
        } else {
            None
        }
    }

    /// True iff this is a [`ExprOp::Param`] node. Returns the param
    /// index (`0` = unindexed positional) wrapped in `Some`.
    #[must_use]
    pub fn as_param(&self) -> Option<u32> {
        if self.opcode()? == ExprOp::Param {
            Some(self.a)
        } else {
            None
        }
    }

    /// Extract the [`LiteralId`] payload of a [`ExprOp::Lit`] node.
    #[must_use]
    pub fn as_lit(&self) -> Option<LiteralId> {
        if self.opcode()? == ExprOp::Lit {
            LiteralId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract the [`CompositeId`] payload of an [`ExprOp::Composite`]
    /// node (array / object / tuple literal).
    #[must_use]
    pub fn as_composite(&self) -> Option<CompositeId> {
        if self.opcode()? == ExprOp::Composite {
            CompositeId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract `(BinOp, lhs, rhs)` of a [`ExprOp::Bin`] node. Returns
    /// `None` if the opcode is wrong, the `aux` tag is unknown, or
    /// either operand id is `0`.
    #[must_use]
    pub fn as_bin(&self) -> Option<(BinOp, NodeId, NodeId)> {
        if self.opcode()? != ExprOp::Bin {
            return None;
        }
        let op = BinOp::try_from_u16(self.aux)?;
        let lhs = NodeId::from_u32(self.a)?;
        let rhs = NodeId::from_u32(self.b)?;
        Some((op, lhs, rhs))
    }

    /// Extract `(UnaryOp, operand)` of a [`ExprOp::Una`] node.
    #[must_use]
    pub fn as_una(&self) -> Option<(UnaryOp, NodeId)> {
        if self.opcode()? != ExprOp::Una {
            return None;
        }
        let op = UnaryOp::try_from_u16(self.aux)?;
        let operand = NodeId::from_u32(self.a)?;
        Some((op, operand))
    }

    /// Extract the [`FuncId`] payload of a [`ExprOp::Func`] node.
    #[must_use]
    pub fn as_func(&self) -> Option<FuncId> {
        if self.opcode()? == ExprOp::Func {
            FuncId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract `(func, expr, distinct)` of an [`ExprOp::Agg`] node.
    #[must_use]
    pub fn as_agg(&self) -> Option<(StrId, NodeId, bool)> {
        if self.opcode()? != ExprOp::Agg {
            return None;
        }
        let func = StrId::from_u32(self.a)?;
        let expr = NodeId::from_u32(self.b)?;
        Some((func, expr, (self.flags & 1) != 0))
    }

    /// Extract the [`WindowId`] payload of a [`ExprOp::Window`] node.
    #[must_use]
    pub fn as_window(&self) -> Option<WindowId> {
        if self.opcode()? == ExprOp::Window {
            WindowId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract `(expr, target type)` of an [`ExprOp::Cast`] node.
    #[must_use]
    pub fn as_cast(&self) -> Option<(NodeId, StrId)> {
        if self.opcode()? != ExprOp::Cast {
            return None;
        }
        let expr = NodeId::from_u32(self.a)?;
        let to = StrId::from_u32(self.b)?;
        Some((expr, to))
    }

    /// Extract the [`CaseId`] payload of a [`ExprOp::Case`] node.
    #[must_use]
    pub fn as_case(&self) -> Option<CaseId> {
        if self.opcode()? == ExprOp::Case {
            CaseId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract `(expr, alias name)` of an [`ExprOp::Alias`] node.
    #[must_use]
    pub fn as_alias(&self) -> Option<(NodeId, StrId)> {
        if self.opcode()? != ExprOp::Alias {
            return None;
        }
        let expr = NodeId::from_u32(self.a)?;
        let name = StrId::from_u32(self.b)?;
        Some((expr, name))
    }

    /// Extract `(probe, collection)` of an [`ExprOp::In`] node.
    /// The collection's opcode discriminates list / subquery / param /
    /// field forms (see [`Self::in_`]).
    #[must_use]
    pub fn as_in(&self) -> Option<(NodeId, NodeId)> {
        if self.opcode()? != ExprOp::In {
            return None;
        }
        let probe = NodeId::from_u32(self.a)?;
        let collection = NodeId::from_u32(self.b)?;
        Some((probe, collection))
    }

    /// Extract the subquery [`NodeId`] of a [`ExprOp::Exists`] node.
    #[must_use]
    pub fn as_exists(&self) -> Option<NodeId> {
        if self.opcode()? == ExprOp::Exists {
            NodeId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract `(expr, lo, hi)` of a [`ExprOp::Between`] node.
    #[must_use]
    pub fn as_between(&self) -> Option<(NodeId, NodeId, NodeId)> {
        if self.opcode()? != ExprOp::Between {
            return None;
        }
        let expr = NodeId::from_u32(self.a)?;
        let lo = NodeId::from_u32(self.b)?;
        let hi = NodeId::from_u32(self.c)?;
        Some((expr, lo, hi))
    }

    /// Extract the [`QueryId`] payload of a [`ExprOp::Query`] node.
    #[must_use]
    pub fn as_query(&self) -> Option<QueryId> {
        if self.opcode()? == ExprOp::Query {
            QueryId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract the [`InsertId`] payload of a [`ExprOp::Insert`] node.
    #[must_use]
    pub fn as_insert(&self) -> Option<InsertId> {
        if self.opcode()? == ExprOp::Insert {
            InsertId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract the [`UpdateId`] payload of a [`ExprOp::Update`] node.
    #[must_use]
    pub fn as_update(&self) -> Option<UpdateId> {
        if self.opcode()? == ExprOp::Update {
            UpdateId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract the [`DeleteId`] payload of a [`ExprOp::Delete`] node.
    #[must_use]
    pub fn as_delete(&self) -> Option<DeleteId> {
        if self.opcode()? == ExprOp::Delete {
            DeleteId::from_u32(self.a)
        } else {
            None
        }
    }

    /// Extract the [`UpsertId`] payload of a [`ExprOp::Upsert`] node.
    #[must_use]
    pub fn as_upsert(&self) -> Option<UpsertId> {
        if self.opcode()? == ExprOp::Upsert {
            UpsertId::from_u32(self.a)
        } else {
            None
        }
    }

    // ── Children ────────────────────────────────────────────────────────────

    /// Iterate the up-to-three child operand handles that refer to other
    /// `ExprNode`s in the same arena. Opcodes whose `a`/`b`/`c` are
    /// not [`NodeId`]s (i.e. they reference side pools or scalar
    /// payloads) yield an empty iterator here; their referent traversal
    /// goes through the side-pool tables.
    pub fn child_node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        let candidates: [u32; 3] = match self.opcode() {
            Some(ExprOp::Bin) => [self.a, self.b, 0],
            Some(ExprOp::Una) => [self.a, 0, 0],
            Some(ExprOp::Agg) => [self.b, 0, 0], // a = func StrId, b = expr NodeId
            Some(ExprOp::Cast) => [self.a, 0, 0], // a = expr NodeId, b = StrId
            Some(ExprOp::Alias) => [self.a, 0, 0], // a = expr NodeId, b = StrId
            Some(ExprOp::In) => [self.a, self.b, 0], // a = probe, b = collection (both NodeId)
            Some(ExprOp::Exists) => [self.a, 0, 0],
            Some(ExprOp::Between) => [self.a, self.b, self.c],
            // Side-pool ops (Field, Func, Case, Window, Composite,
            // Query, Insert, Update, Delete, Upsert) and leaf ops
            // (Namespace, Param, Lit, Nop) reference non-`NodeId`
            // payloads — no recursion through this edge.
            _ => [0, 0, 0],
        };
        candidates.into_iter().filter_map(NodeId::from_u32)
    }
}

#[cfg(test)]
mod size_tests {
    use core::mem::size_of;

    use super::ExprNode;
    use crate::types::value::{Literal, Value};

    #[test]
    fn expr_node_is_16_bytes() {
        let sz = size_of::<ExprNode>();
        assert_eq!(sz, 16, "ExprNode is {sz} bytes — must be exactly 16");
    }

    #[test]
    fn value_fits_24_bytes() {
        let sz = size_of::<Value>();
        assert!(sz <= 24, "Value is {sz} bytes — must be ≤ 24",);
    }

    #[test]
    fn literal_static_fits_32_bytes() {
        let sz = size_of::<Literal<'static>>();
        assert!(sz <= 32, "Literal<'static> is {sz} bytes — must be ≤ 32",);
    }
}

#[cfg(test)]
mod codec_tests {
    use super::*;

    #[test]
    fn binop_u16_round_trips() {
        for tag in 0u16..=23 {
            let op = BinOp::try_from_u16(tag).expect("known tag");
            assert_eq!(op.as_u16(), tag);
        }
        assert!(BinOp::try_from_u16(24).is_none());
        assert!(BinOp::try_from_u16(u16::MAX).is_none());
    }

    #[test]
    fn unaryop_u16_round_trips() {
        for tag in 0u16..=6 {
            let op = UnaryOp::try_from_u16(tag).expect("known tag");
            assert_eq!(op.as_u16(), tag);
        }
        assert!(UnaryOp::try_from_u16(7).is_none());
    }

    #[test]
    fn expr_op_u8_round_trips() {
        for tag in 0u8..=21 {
            let op = ExprOp::from_u8(tag).expect("known tag");
            assert_eq!(op as u8, tag);
        }
        assert!(ExprOp::from_u8(22).is_none());
        assert!(ExprOp::from_u8(255).is_none());
    }

    #[test]
    fn typed_constructors_and_accessors_round_trip() {
        let nid = NodeId::from_u32(7).unwrap();
        let nid2 = NodeId::from_u32(11).unwrap();
        let nid3 = NodeId::from_u32(13).unwrap();
        let sid = StrId::from_u32(3).unwrap();

        // Bin
        let n = ExprNode::bin(BinOp::Eq, nid, nid2);
        assert_eq!(n.as_bin(), Some((BinOp::Eq, nid, nid2)));
        // Una
        let n = ExprNode::una(UnaryOp::Not, nid);
        assert_eq!(n.as_una(), Some((UnaryOp::Not, nid)));
        // Agg with distinct
        let n = ExprNode::agg(sid, nid, true);
        assert_eq!(n.as_agg(), Some((sid, nid, true)));
        let n = ExprNode::agg(sid, nid, false);
        assert_eq!(n.as_agg(), Some((sid, nid, false)));
        // Cast
        let n = ExprNode::cast(nid, sid);
        assert_eq!(n.as_cast(), Some((nid, sid)));
        // Alias
        let n = ExprNode::alias(nid, sid);
        assert_eq!(n.as_alias(), Some((nid, sid)));
        // In (collapsed InList + InSub)
        let n = ExprNode::in_(nid, nid2);
        assert_eq!(n.as_in(), Some((nid, nid2)));
        // Composite (collapsed ObjectLit + ArrayLit)
        let cid = crate::ids::CompositeId::from_u32(5).unwrap();
        let n = ExprNode::composite(cid);
        assert_eq!(n.as_composite(), Some(cid));
        // Between
        let n = ExprNode::between(nid, nid2, nid3);
        assert_eq!(n.as_between(), Some((nid, nid2, nid3)));
        // Param
        let n = ExprNode::param(0);
        assert_eq!(n.as_param(), Some(0));
        let n = ExprNode::param(42);
        assert_eq!(n.as_param(), Some(42));
        // Wrong-opcode accessors return None.
        assert!(n.as_bin().is_none());
        assert!(n.as_field().is_none());
    }

    #[test]
    fn child_iter_for_bin_yields_two() {
        let n = ExprNode::bin(
            BinOp::Eq,
            NodeId::from_u32(2).unwrap(),
            NodeId::from_u32(3).unwrap(),
        );
        let kids: alloc::vec::Vec<NodeId> = n.child_node_ids().collect();
        assert_eq!(
            kids,
            alloc::vec![NodeId::from_u32(2).unwrap(), NodeId::from_u32(3).unwrap()]
        );
    }

    #[test]
    fn child_iter_for_lit_yields_none() {
        let n = ExprNode::lit(LiteralId::from_u32(5).unwrap());
        let kids: alloc::vec::Vec<NodeId> = n.child_node_ids().collect();
        assert!(kids.is_empty());
    }

    #[test]
    fn child_iter_for_between_yields_three() {
        let n = ExprNode::between(
            NodeId::from_u32(2).unwrap(),
            NodeId::from_u32(3).unwrap(),
            NodeId::from_u32(4).unwrap(),
        );
        let kids: alloc::vec::Vec<NodeId> = n.child_node_ids().collect();
        assert_eq!(kids.len(), 3);
    }

    #[test]
    fn child_iter_for_agg_yields_one_node_id() {
        // Agg's `a` is a StrId (function name) — not a NodeId — so only
        // `b` (the operand) participates in node-graph traversal.
        let func = StrId::from_u32(1).unwrap();
        let expr = NodeId::from_u32(99).unwrap();
        let n = ExprNode::agg(func, expr, false);
        let kids: alloc::vec::Vec<NodeId> = n.child_node_ids().collect();
        assert_eq!(kids, alloc::vec![expr]);
    }

    #[test]
    fn pod_castable_to_bytes() {
        let n = ExprNode::bin(
            BinOp::Add,
            NodeId::from_u32(1).unwrap(),
            NodeId::from_u32(2).unwrap(),
        );
        let bytes: &[u8] = bytemuck::bytes_of(&n);
        assert_eq!(bytes.len(), 16);
        let n2: ExprNode = *bytemuck::from_bytes(bytes);
        assert_eq!(n, n2);
    }
}

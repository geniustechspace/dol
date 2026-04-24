use smallvec::SmallVec;

use crate::expr::{ExprNode, Order, MutateNode, SelectNode};
use crate::ids::{
    CaseId, FieldId, FuncId, InListId, LiteralId, MutateId, NodeId, ObjLitId, SelectId, SpanId,
    StrId, WindowId,
};
use crate::types::value::Literal;

// ─── FieldStep / FieldNode ───────────────────────────────────────────────────

/// A single traversal step inside a [`FieldNode`] path.
///
/// Steps allow a [`ExprNode::Field`] to express arbitrarily deep navigation
/// through JSON/object structures or arrays, independent of the backing store:
///
/// | Step | SQL (Postgres JSONB) | REST / document |
/// |---|---|---|
/// | `Key("name")` | `col->>'name'` | `.name` |
/// | `Index(0)`    | `col->>0`      | `[0]`   |
#[derive(Debug, Clone, PartialEq)]
pub enum FieldStep {
    /// Named key access: `.key`, `->>'key'`, `["key"]`.
    Key(StrId),
    /// Positional array index: `[n]`, `->>n`.
    Index(u32),
}

/// Payload for [`ExprNode::Field`], stored in [`ExprArena::fields`].
///
/// `namespace` is an optional table/schema qualifier (e.g. the alias `u` in
/// `u.profile_json`, or a fully-qualified `public.users`). `column` is the
/// base column or attribute name. `steps` is the optional traversal chain
/// that follows the column — empty means a bare column reference.
///
/// # Backend rendering examples
///
/// | `namespace` | `column`       | `steps`          | SQL (Postgres)                    |
/// |-------------|----------------|------------------|-----------------------------------|
/// | `None`      | `"id"`         | `[]`             | `id`                              |
/// | `Some("u")` | `"id"`         | `[]`             | `u.id`                            |
/// | `None`      | `"profile_json"` | `[Key("name")]` | `profile_json->>'name'`           |
/// | `Some("u")` | `"data"`       | `[Key("x"), Index(0)]` | `u.data->'x'->>0`         |
#[derive(Debug, Clone, PartialEq)]
pub struct FieldNode {
    /// Optional table/alias qualifier (`None` = unqualified).
    pub namespace: Option<StrId>,
    /// Base column or attribute name (first step from the container).
    pub column:    StrId,
    /// Traversal steps that follow the column (empty = bare column reference).
    pub steps:     SmallVec<[FieldStep; 4]>,
}

// ─── Pooled payload structs ───────────────────────────────────────────────────

/// Payload for [`ExprNode::Func`], stored in [`ExprArena::funcs`].
///
/// Moved out of the enum variant to keep `ExprNode` ≤ 32 bytes: the
/// two fields (`name: u32` + 4-byte alignment gap + 24-byte `SmallVec`)
/// would otherwise push the variant to 32 bytes of *payload*, which
/// combined with the discriminant word exceeds the target.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncNode {
    pub name: StrId,
    pub args: SmallVec<[NodeId; 4]>,
}

/// Payload for [`ExprNode::ObjectLit`], stored in [`ExprArena::obj_lits`].
///
/// The inline buffer of `SmallVec<[(StrId, NodeId); 4]>` is 4 × 8 = 32 bytes
/// on its own — already over budget before the discriminant word is counted.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjLitNode(pub SmallVec<[(StrId, NodeId); 4]>);

/// Payload for [`ExprNode::Window`], stored in [`ExprArena::windows`].
///
/// Two `SmallVec` fields (each 24 bytes) plus `func: StrId` total 52+ bytes
/// of payload — pooled to keep `ExprNode` ≤ 32 bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowNode {
    pub func:      StrId,
    pub partition: SmallVec<[NodeId; 4]>,
    pub order:     SmallVec<[(NodeId, Order); 2]>,
}

/// Payload for [`ExprNode::Case`], stored in [`ExprArena::cases`].
///
/// `SmallVec<[(NodeId, NodeId); 4]>` has a 32-byte inline buffer, making the
/// variant payload 36+ bytes — pooled to keep `ExprNode` ≤ 32 bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseNode {
    pub branches: SmallVec<[(NodeId, NodeId); 4]>,
    pub else_:    NodeId,
}

/// Payload for [`ExprNode::InList`], stored in [`ExprArena::in_lists`].
///
/// `SmallVec<[NodeId; 8]>` has a 32-byte inline buffer, making the variant
/// payload 36+ bytes — pooled to keep `ExprNode` ≤ 32 bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct InListNode {
    pub expr: NodeId,
    pub list: SmallVec<[NodeId; 8]>,
}

// ─── SpanTable ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Span {
    pub start: u32,
    pub end:   u32,
}

#[derive(Debug, Clone, Default)]
pub struct SpanTable {
    spans: Vec<Span>,
}

impl SpanTable {
    pub fn push(&mut self, span: Span) -> SpanId {
        let id = self.spans.len() as SpanId;
        self.spans.push(span);
        id
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.spans.get(id as usize)
    }
}

// ─── ExprArena ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ExprArena {
    nodes:      Vec<ExprNode>,
    span_table: SpanTable,
    /// Pooled literal values — indexed by [`LiteralId`].
    lits:       Vec<Literal<'static>>,
    /// Pooled function-call payloads — indexed by [`FuncId`].
    funcs:      Vec<FuncNode>,
    /// Pooled object-literal payloads — indexed by [`ObjLitId`].
    obj_lits:   Vec<ObjLitNode>,
    /// Pooled window-function payloads — indexed by [`WindowId`].
    windows:    Vec<WindowNode>,
    /// Pooled CASE expression payloads — indexed by [`CaseId`].
    cases:      Vec<CaseNode>,
    /// Pooled IN-list payloads — indexed by [`InListId`].
    in_lists:   Vec<InListNode>,
    /// Pooled SELECT statement payloads — indexed by [`SelectId`].
    selects:    Vec<SelectNode>,
    /// Pooled mutate (INSERT/UPDATE/DELETE/UPSERT) payloads — indexed by [`MutateId`].
    mutates:    Vec<MutateNode>,
    /// Pooled field-reference payloads — indexed by [`FieldId`].
    fields:     Vec<FieldNode>,
}

impl ExprArena {
    pub fn new() -> Self { Self::default() }

    // ── ExprNode pool ────────────────────────────────────────────────────────

    pub fn alloc(&mut self, node: ExprNode) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: NodeId) -> &ExprNode { &self.nodes[id as usize] }

    pub fn len(&self) -> usize { self.nodes.len() }

    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    pub fn attach_span(&mut self, _id: NodeId, span: Span) -> SpanId {
        self.span_table.push(span)
    }

    // ── Literal pool ─────────────────────────────────────────────────────────

    /// Store a [`Literal`] in the pool and return its [`LiteralId`].
    pub fn alloc_lit(&mut self, lit: Literal<'static>) -> LiteralId {
        let id = self.lits.len() as LiteralId;
        self.lits.push(lit);
        id
    }

    /// Retrieve a [`Literal`] by its [`LiteralId`].
    pub fn get_lit(&self, id: LiteralId) -> &Literal<'static> {
        &self.lits[id as usize]
    }

    // ── FuncNode pool ─────────────────────────────────────────────────────────

    /// Store a [`FuncNode`] in the pool and return its [`FuncId`].
    pub fn alloc_func(&mut self, func: FuncNode) -> FuncId {
        let id = self.funcs.len() as FuncId;
        self.funcs.push(func);
        id
    }

    /// Retrieve a [`FuncNode`] by its [`FuncId`].
    pub fn get_func(&self, id: FuncId) -> &FuncNode {
        &self.funcs[id as usize]
    }

    // ── ObjLitNode pool ───────────────────────────────────────────────────────

    /// Store an [`ObjLitNode`] in the pool and return its [`ObjLitId`].
    pub fn alloc_obj_lit(&mut self, obj: ObjLitNode) -> ObjLitId {
        let id = self.obj_lits.len() as ObjLitId;
        self.obj_lits.push(obj);
        id
    }

    /// Retrieve an [`ObjLitNode`] by its [`ObjLitId`].
    pub fn get_obj_lit(&self, id: ObjLitId) -> &ObjLitNode {
        &self.obj_lits[id as usize]
    }

    // ── WindowNode pool ───────────────────────────────────────────────────────

    /// Store a [`WindowNode`] in the pool and return its [`WindowId`].
    pub fn alloc_window(&mut self, win: WindowNode) -> WindowId {
        let id = self.windows.len() as WindowId;
        self.windows.push(win);
        id
    }

    /// Retrieve a [`WindowNode`] by its [`WindowId`].
    pub fn get_window(&self, id: WindowId) -> &WindowNode {
        &self.windows[id as usize]
    }

    // ── CaseNode pool ─────────────────────────────────────────────────────────

    /// Store a [`CaseNode`] in the pool and return its [`CaseId`].
    pub fn alloc_case(&mut self, case: CaseNode) -> CaseId {
        let id = self.cases.len() as CaseId;
        self.cases.push(case);
        id
    }

    /// Retrieve a [`CaseNode`] by its [`CaseId`].
    pub fn get_case(&self, id: CaseId) -> &CaseNode {
        &self.cases[id as usize]
    }

    // ── InListNode pool ───────────────────────────────────────────────────────

    /// Store an [`InListNode`] in the pool and return its [`InListId`].
    pub fn alloc_in_list(&mut self, node: InListNode) -> InListId {
        let id = self.in_lists.len() as InListId;
        self.in_lists.push(node);
        id
    }

    /// Retrieve an [`InListNode`] by its [`InListId`].
    pub fn get_in_list(&self, id: InListId) -> &InListNode {
        &self.in_lists[id as usize]
    }

    // ── SelectNode pool ───────────────────────────────────────────────────────

    /// Store a [`SelectNode`] in the pool and return its [`SelectId`].
    pub fn alloc_select(&mut self, sel: SelectNode) -> SelectId {
        let id = self.selects.len() as SelectId;
        self.selects.push(sel);
        id
    }

    /// Retrieve a [`SelectNode`] by its [`SelectId`].
    pub fn get_select(&self, id: SelectId) -> &SelectNode {
        &self.selects[id as usize]
    }

    // ── MutateNode pool ───────────────────────────────────────────────────────

    /// Store a [`MutateNode`] in the pool and return its [`MutateId`].
    pub fn alloc_mutate(&mut self, mut_node: MutateNode) -> MutateId {
        let id = self.mutates.len() as MutateId;
        self.mutates.push(mut_node);
        id
    }

    /// Retrieve a [`MutateNode`] by its [`MutateId`].
    pub fn get_mutate(&self, id: MutateId) -> &MutateNode {
        &self.mutates[id as usize]
    }

    // ── FieldNode pool ────────────────────────────────────────────────────────

    /// Store a [`FieldNode`] in the pool and return its [`FieldId`].
    pub fn alloc_field(&mut self, field: FieldNode) -> FieldId {
        let id = self.fields.len() as FieldId;
        self.fields.push(field);
        id
    }

    /// Retrieve a [`FieldNode`] by its [`FieldId`].
    pub fn get_field(&self, id: FieldId) -> &FieldNode {
        &self.fields[id as usize]
    }
}

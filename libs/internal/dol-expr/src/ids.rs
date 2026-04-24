/// Index into ExprArena::nodes — never a pointer.
pub type NodeId = u32;
/// Index into Interner::strings — never a &str in AST nodes.
pub type StrId = u32;
/// Index into TypeArena::types — never a Box<DataType>.
pub type TypeId = u32;
/// Index into SpanTable::spans — kept separate from hot data.
pub type SpanId = u32;
/// Index into ExprArena::lits — identifies a pooled Literal<'static>.
pub type LiteralId = u32;
/// Index into ExprArena::funcs — identifies a pooled FuncNode.
pub type FuncId = u32;
/// Index into ExprArena::obj_lits — identifies a pooled ObjLitNode.
pub type ObjLitId = u32;
/// Index into ExprArena::windows — identifies a pooled WindowNode.
pub type WindowId = u32;
/// Index into ExprArena::cases — identifies a pooled CaseNode.
pub type CaseId = u32;
/// Index into ExprArena::in_lists — identifies a pooled InListNode.
pub type InListId = u32;
/// Index into ExprArena::selects — identifies a pooled SelectNode.
pub type SelectId = u32;
/// Index into ExprArena::mutates — identifies a pooled MutateNode.
pub type MutateId = u32;
/// Index into ExprArena::fields — identifies a pooled FieldNode.
pub type FieldId = u32;
/// Sentinel for "no node" — use instead of Option<NodeId> where size matters.
pub const NULL_NODE: NodeId = u32::MAX;

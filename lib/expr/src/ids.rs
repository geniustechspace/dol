//! Typed identifier newtypes used throughout the expression IR.
//!
//! Each `*Id` is a 32-bit index into a specific arena pool — never a raw
//! pointer or borrowed reference. Tagging IDs by purpose prevents accidental
//! cross-pool aliasing while preserving the cache-friendly representation.

/// Index into ExprArena::nodes — never a pointer.
pub type NodeId = u32;
/// Index into Interner::strings — never a &str in AST nodes.
pub type StrId = u32;
/// Index into `TypeArena::types` — never a `Box<DataType>`.
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
/// Index into ExprArena::queries — identifies a pooled QueryNode.
pub type QueryId = u32;
/// Index into ExprArena::inserts — identifies a pooled InsertNode.
pub type InsertId = u32;
/// Index into ExprArena::updates — identifies a pooled UpdateNode.
pub type UpdateId = u32;
/// Index into ExprArena::deletes — identifies a pooled DeleteNode.
pub type DeleteId = u32;
/// Index into ExprArena::upserts — identifies a pooled UpsertNode.
pub type UpsertId = u32;
/// Index into ExprArena::fields — identifies a pooled FieldNode.
pub type FieldId = u32;
/// Sentinel for "no node" — use instead of `Option<NodeId>` where size matters.
pub const NULL_NODE: NodeId = u32::MAX;

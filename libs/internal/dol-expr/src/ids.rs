/// Index into ExprArena::nodes — never a pointer.
pub type NodeId = u32;
/// Index into Interner::strings — never a &str in AST nodes.
pub type StrId = u32;
/// Index into TypeArena::types — never a Box<DataType>.
pub type TypeId = u32;
/// Index into SpanTable::spans — kept separate from hot data.
pub type SpanId = u32;
/// Sentinel for "no node" — use instead of Option<NodeId> where size matters.
pub const NULL_NODE: NodeId = u32::MAX;

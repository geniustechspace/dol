//! Flat expression arena: contiguous storage for [`ExprNode`]s.
//!
//! [`ExprArena`] stores nodes by [`NodeId`] index, eliminating pointer
//! chasing across the tree and enabling iterative traversal in Phase 4.
//!
//! # Usage
//!
//! ```rust,ignore
//! let mut arena    = ExprArena::new();
//! let mut interner = Interner::new();
//!
//! let expr = field("age").gt(int(18i32));
//! let root = arena.lower(&expr, &mut interner);
//!
//! let node = arena.get(root);  // ExprNode::BinaryOp { … }
//! ```

use smallvec::SmallVec;

use super::func_meta::FuncDef;
use super::interner::{Interner, StrId};
use super::node::{
    ArenaOrderBy, CaseNode, CastNode, ExprNode, FuncId, FuncNode, NodeId, OpId, PathIds,
    WindowNode,
};
use super::op_meta::OpDef;
use super::Expr;

/// Flat arena storing expression nodes and the operator/function tables they
/// reference.
///
/// All indices ([`NodeId`], [`OpId`], [`FuncId`]) are scoped to this arena.
///
/// # Reuse
///
/// Call [`reset`](ExprArena::reset) to clear the arena while retaining its
/// allocated backing storage, then lower a new expression into it.  This is
/// used internally by [`BuildSession`](crate::session::BuildSession) to avoid
/// repeated heap allocations during multi-clause validation.
#[derive(Debug, Clone)]
pub struct ExprArena {
    nodes: Vec<ExprNode>,
    ops:   Vec<OpDef>,
    funcs: Vec<FuncDef>,
}

impl ExprArena {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self { nodes: Vec::new(), ops: Vec::new(), funcs: Vec::new() }
    }

    /// Create an arena with pre-allocated capacity for `nodes` node slots.
    ///
    /// Op and function tables default to small initial capacities since they
    /// are deduplicated and rarely exceed 30 entries.
    pub fn with_capacity(nodes: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(nodes),
            ops:   Vec::with_capacity(16),
            funcs: Vec::with_capacity(16),
        }
    }

    /// Clear all nodes, operators, and functions while retaining allocated
    /// storage.
    ///
    /// After `reset()` the arena behaves as if freshly created but the backing
    /// `Vec` memory is reused for the next lowering pass.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.ops.clear();
        self.funcs.clear();
    }

    // ── Read access ──────────────────────────────────────────────────────────

    /// Look up a node by id.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not produced by this arena.
    pub fn get(&self, id: NodeId) -> &ExprNode {
        &self.nodes[id.0 as usize]
    }

    /// Look up an operator definition by id.
    pub fn op(&self, id: OpId) -> &OpDef {
        &self.ops[id.0 as usize]
    }

    /// Look up a function definition by id.
    pub fn func(&self, id: FuncId) -> &FuncDef {
        &self.funcs[id.0 as usize]
    }

    /// Total number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if no nodes have been allocated yet.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    // ── Lowering ─────────────────────────────────────────────────────────────

    /// Lower an [`Expr<'_>`] into the arena, returning the root [`NodeId`].
    ///
    /// All strings are interned into `interner`. Sub-expressions are
    /// recursively lowered; the tree is linearised in post-order (children
    /// before parents).
    pub fn lower(&mut self, expr: &Expr<'_>, interner: &mut Interner) -> NodeId {
        let node = match expr {
            Expr::Ref(path) => {
                let ids = self.intern_path(path.iter(), interner);
                ExprNode::Ref(ids)
            }

            Expr::Access { base, path } => {
                let base_id = self.lower(base, interner);
                let ids = self.intern_path(path.iter(), interner);
                ExprNode::Access { base: base_id, path: ids }
            }

            Expr::Param => ExprNode::Param,

            Expr::Value(lit) => ExprNode::Value(lit.clone().into_static()),

            Expr::Array(elements) => {
                let ids: SmallVec<[NodeId; 4]> =
                    elements.iter().map(|e| self.lower(e, interner)).collect();
                ExprNode::Array(ids)
            }

            Expr::Object(fields) => {
                let pairs: SmallVec<[(StrId, NodeId); 2]> = fields
                    .iter()
                    .map(|(k, v)| {
                        let kid = interner.intern(k.as_str());
                        let vid = self.lower(v, interner);
                        (kid, vid)
                    })
                    .collect();
                ExprNode::Object(pairs)
            }

            Expr::BinaryOp { left, op, right } => {
                let lid = self.lower(left, interner);
                let rid = self.lower(right, interner);
                let oid = self.intern_op(op);
                ExprNode::BinaryOp { left: lid, op: oid, right: rid }
            }

            Expr::UnaryOp { op, expr: inner } => {
                use super::ops::UnaryOp;
                // Canonicalise NOT(InList) → NotInList and NOT(Between) → NotBetween
                // so the post-order scan doesn't need look-ahead.
                if *op == UnaryOp::Not {
                    match inner.as_ref() {
                        Expr::InList { expr, list } => {
                            let eid = self.lower(expr, interner);
                            let ids: Vec<NodeId> =
                                list.iter().map(|e| self.lower(e, interner)).collect();
                            return self.push(ExprNode::NotInList { expr: eid, list: ids });
                        }
                        Expr::Between { expr, low, high } => {
                            let eid = self.lower(expr, interner);
                            let lid = self.lower(low,  interner);
                            let hid = self.lower(high, interner);
                            return self.push(ExprNode::NotBetween { expr: eid, low: lid, high: hid });
                        }
                        _ => {}
                    }
                }
                let inner_id = self.lower(inner, interner);
                ExprNode::UnaryOp { op: *op, expr: inner_id }
            }

            Expr::Func { name, args } => {
                let fid = self.intern_func(name);
                let arg_ids: SmallVec<[NodeId; 4]> =
                    args.iter().map(|a| self.lower(a, interner)).collect();
                ExprNode::Func(Box::new(FuncNode { id: fid, args: arg_ids }))
            }

            Expr::Cast { expr: inner, as_type } => {
                let inner_id = self.lower(inner, interner);
                ExprNode::Cast(Box::new(CastNode {
                    expr:    inner_id,
                    as_type: as_type.clone(),
                }))
            }

            Expr::Case { whens, else_expr } => {
                let when_ids: SmallVec<[(NodeId, NodeId); 2]> = whens
                    .iter()
                    .map(|(c, t)| (self.lower(c, interner), self.lower(t, interner)))
                    .collect();
                let else_id = else_expr.as_deref().map(|e| self.lower(e, interner));
                ExprNode::Case(Box::new(CaseNode { whens: when_ids, else_expr: else_id }))
            }

            Expr::Between { expr, low, high } => {
                let eid = self.lower(expr, interner);
                let lid = self.lower(low,  interner);
                let hid = self.lower(high, interner);
                ExprNode::Between { expr: eid, low: lid, high: hid }
            }

            Expr::InList { expr, list } => {
                let eid = self.lower(expr, interner);
                let ids: Vec<NodeId> = list.iter().map(|e| self.lower(e, interner)).collect();
                ExprNode::InList { expr: eid, list: ids }
            }

            Expr::Alias { expr: inner, alias } => {
                let inner_id = self.lower(inner, interner);
                let aid = interner.intern(alias.as_str());
                ExprNode::Alias { expr: inner_id, alias: aid }
            }

            Expr::Star      => ExprNode::Star,
            Expr::CountStar => ExprNode::CountStar,

            Expr::Window { func, partition_by, order_by, frame } => {
                let fid  = self.lower(func, interner);
                let part: SmallVec<[NodeId; 3]> =
                    partition_by.iter().map(|e| self.lower(e, interner)).collect();
                let ord: SmallVec<[ArenaOrderBy; 2]> = order_by
                    .iter()
                    .map(|ob| ArenaOrderBy {
                        expr:      self.lower(&ob.expr, interner),
                        direction: ob.direction,
                        nulls:     ob.nulls,
                    })
                    .collect();
                ExprNode::Window(Box::new(WindowNode {
                    func:         fid,
                    partition_by: part,
                    order_by:     ord,
                    frame:        frame.clone(),
                }))
            }
        };
        self.push(node)
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn push(&mut self, node: ExprNode) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    fn intern_path<'s>(
        &self,
        segments: impl Iterator<Item = &'s str>,
        interner: &mut Interner,
    ) -> PathIds {
        segments.map(|s| interner.intern(s)).collect()
    }

    fn intern_op(&mut self, op: &OpDef) -> OpId {
        // Linear scan — op tables are tiny (< 30 entries in practice).
        if let Some(pos) = self.ops.iter().position(|o| o == op) {
            return OpId(pos as u32);
        }
        let id = OpId(self.ops.len() as u32);
        self.ops.push(op.clone());
        id
    }

    fn intern_func(&mut self, func: &FuncDef) -> FuncId {
        if let Some(pos) = self.funcs.iter().position(|f| f == func) {
            return FuncId(pos as u32);
        }
        let id = FuncId(self.funcs.len() as u32);
        self.funcs.push(func.clone());
        id
    }
}

impl Default for ExprArena {
    fn default() -> Self {
        Self::new()
    }
}

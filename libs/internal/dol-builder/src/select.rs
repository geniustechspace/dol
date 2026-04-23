use dol_expr::{
    ExprArena, Interner,
    expr::{Order, SelectNode},
    ids::{NodeId, NULL_NODE},
};
use dol_ir::Statement;
use smallvec::SmallVec;

pub struct SelectBuilder {
    pub arena:    ExprArena,
    pub interner: Interner,
    node: SelectNode,
}

impl SelectBuilder {
    pub fn from(table: &str) -> Self {
        let mut interner = Interner::new();
        let from = interner.intern(table);
        Self {
            arena: ExprArena::new(),
            interner,
            node: SelectNode {
                from,
                alias:    None,
                joins:    SmallVec::new(),
                filter:   NULL_NODE,
                columns:  SmallVec::new(),
                group_by: SmallVec::new(),
                having:   NULL_NODE,
                order_by: SmallVec::new(),
                limit:    None,
                offset:   None,
                lock:     None,
            },
        }
    }

    pub fn alias(mut self, alias: &str) -> Self {
        let id = self.interner.intern(alias);
        self.node.alias = Some(id);
        self
    }

    pub fn column(mut self, col: NodeId) -> Self {
        self.node.columns.push(col);
        self
    }

    pub fn filter(mut self, expr: NodeId) -> Self {
        self.node.filter = expr;
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.node.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.node.offset = Some(n);
        self
    }

    pub fn order_by(mut self, expr: NodeId, order: Order) -> Self {
        self.node.order_by.push((expr, order));
        self
    }

    pub fn build(self) -> Statement {
        Statement::Select(self.node)
    }
}

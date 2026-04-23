use dol_expr::{
    ExprArena, Interner,
    expr::{MutateKind, MutateNode},
    ids::{NodeId, NULL_NODE},
};
use dol_ir::Statement;
use smallvec::SmallVec;

pub struct InsertBuilder {
    pub arena:    ExprArena,
    pub interner: Interner,
    node: MutateNode,
}

impl InsertBuilder {
    pub fn into(table: &str) -> Self {
        let mut interner = Interner::new();
        let target = interner.intern(table);
        Self {
            arena: ExprArena::new(),
            interner,
            node: MutateNode {
                kind:      MutateKind::Insert,
                target,
                columns:   SmallVec::new(),
                values:    SmallVec::new(),
                filter:    NULL_NODE,
                returning: SmallVec::new(),
                conflict:  None,
            },
        }
    }

    pub fn column(mut self, col: &str) -> Self {
        let id = self.interner.intern(col);
        self.node.columns.push(id);
        self
    }

    pub fn value(mut self, val: NodeId) -> Self {
        self.node.values.push(val);
        self
    }

    pub fn build(self) -> Statement {
        Statement::Insert(self.node)
    }
}

pub struct UpdateBuilder {
    pub arena:    ExprArena,
    pub interner: Interner,
    node: MutateNode,
}

impl UpdateBuilder {
    pub fn table(table: &str) -> Self {
        let mut interner = Interner::new();
        let target = interner.intern(table);
        Self {
            arena: ExprArena::new(),
            interner,
            node: MutateNode {
                kind:      MutateKind::Update,
                target,
                columns:   SmallVec::new(),
                values:    SmallVec::new(),
                filter:    NULL_NODE,
                returning: SmallVec::new(),
                conflict:  None,
            },
        }
    }

    pub fn set(mut self, col: &str, val: NodeId) -> Self {
        let id = self.interner.intern(col);
        self.node.columns.push(id);
        self.node.values.push(val);
        self
    }

    pub fn filter(mut self, expr: NodeId) -> Self {
        self.node.filter = expr;
        self
    }

    pub fn build(self) -> Statement {
        Statement::Update(self.node)
    }
}

pub struct DeleteBuilder {
    pub arena:    ExprArena,
    pub interner: Interner,
    node: MutateNode,
}

impl DeleteBuilder {
    pub fn from(table: &str) -> Self {
        let mut interner = Interner::new();
        let target = interner.intern(table);
        Self {
            arena: ExprArena::new(),
            interner,
            node: MutateNode {
                kind:      MutateKind::Delete,
                target,
                columns:   SmallVec::new(),
                values:    SmallVec::new(),
                filter:    NULL_NODE,
                returning: SmallVec::new(),
                conflict:  None,
            },
        }
    }

    pub fn filter(mut self, expr: NodeId) -> Self {
        self.node.filter = expr;
        self
    }

    pub fn build(self) -> Statement {
        Statement::Delete(self.node)
    }
}

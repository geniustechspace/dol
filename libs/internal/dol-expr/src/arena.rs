use crate::expr::ExprNode;
use crate::ids::{NodeId, SpanId};

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

#[derive(Debug, Clone, Default)]
pub struct ExprArena {
    nodes:      Vec<ExprNode>,
    span_table: SpanTable,
}

impl ExprArena {
    pub fn new() -> Self { Self::default() }

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
}

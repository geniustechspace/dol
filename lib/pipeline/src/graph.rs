//! Pipeline DAG container.

use smallvec::SmallVec;

use crate::node::Node;

/// Index of a node within a [`Graph`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NodeIdx(pub u32);

/// A directed pipeline graph.
///
/// The graph is stored as a `Vec<Node>` plus an adjacency list of inputs per
/// node. Outputs are inferred (a node `j` is an output of `i` iff `i` appears
/// in `j`'s input list). The graph is intended to be acyclic; cycles are
/// detected by `dol-check` rather than refused at the data-structure layer
/// (so partially-built graphs can be inspected).
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Graph {
    /// Nodes in topological-friendly insertion order.
    pub nodes: alloc::vec::Vec<Node>,
    /// `inputs[i]` lists the [`NodeIdx`] feeding node `i`.
    pub inputs: alloc::vec::Vec<SmallVec<[NodeIdx; 2]>>,
    /// Output sink indices (terminals).
    pub outputs: SmallVec<[NodeIdx; 2]>,
}

impl Graph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node with the given inputs; returns its index.
    pub fn add(&mut self, node: Node, inputs: impl IntoIterator<Item = NodeIdx>) -> NodeIdx {
        let idx = NodeIdx(self.nodes.len() as u32);
        self.nodes.push(node);
        self.inputs
            .push(inputs.into_iter().collect::<SmallVec<[_; 2]>>());
        idx
    }

    /// Mark a node as an output (sink).
    pub fn mark_output(&mut self, idx: NodeIdx) {
        self.outputs.push(idx);
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// `true` if the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, Source};
    use crate::schema::RowSchema;

    #[test]
    fn graph_grows() {
        let mut g = Graph::new();
        let s = g.add(
            Node::Source(Source::Entity {
                target: "users".into(),
                schema: RowSchema::empty(),
            }),
            [],
        );
        assert_eq!(g.len(), 1);
        assert_eq!(s, NodeIdx(0));
    }
}

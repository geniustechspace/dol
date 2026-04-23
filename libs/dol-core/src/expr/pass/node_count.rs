//! [`NodeCountPass`] — fail-fast check for runaway expression trees.

use crate::expr::arena::ExprArena;
use super::{PassCaps, PassError};

/// Validates that the arena does not exceed the node count limit.
///
/// This is the cheapest possible check — no traversal needed.
pub struct NodeCountPass;

impl NodeCountPass {
    /// Check the arena's node count against `caps.max_nodes`.
    pub fn check(&self, arena: &ExprArena, caps: &PassCaps) -> Vec<PassError> {
        let count = arena.len();
        if count > caps.max_nodes {
            vec![PassError::TooManyNodes { count, limit: caps.max_nodes }]
        } else {
            vec![]
        }
    }
}

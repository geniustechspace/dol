//! [`ScopePass`] — validate that field paths exist in the schema.
//!
//! The schema is optional: if `None` is provided, the pass is a no-op.  Full
//! schema integration (column types, multi-table scoping) is wired in Phase 5.

use std::collections::HashSet;

use crate::expr::arena::ExprArena;
use crate::expr::interner::Interner;
use crate::expr::node::ExprNode;
use super::PassError;

/// A flat set of known field paths (dotted strings, e.g. `"users.email"`).
///
/// Built by the query builder from schema metadata. In Phase 5 this will carry
/// type information as well; for now it is purely a membership set.
#[derive(Debug, Default, Clone)]
pub struct Schema {
    fields: HashSet<String>,
}

impl Schema {
    /// Create an empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a field path as known.
    ///
    /// `path` is a dotted identifier string: `"age"`, `"users.email"`, etc.
    pub fn add_field(&mut self, path: impl Into<String>) {
        self.fields.insert(path.into());
    }

    /// Build a schema from an iterator of dotted path strings.
    pub fn from_fields<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self { fields: iter.into_iter().map(Into::into).collect() }
    }

    /// Returns `true` if `path` is a known field.
    pub fn contains(&self, path: &str) -> bool {
        self.fields.contains(path)
    }
}

/// Validates that every [`ExprNode::Ref`] path is present in the schema.
pub struct ScopePass;

impl ScopePass {
    /// Check all `Ref` nodes. If `schema` is `None`, returns empty (no-op).
    pub fn check(
        &self,
        arena:    &ExprArena,
        interner: &Interner,
        schema:   Option<&Schema>,
    ) -> Vec<PassError> {
        let schema = match schema {
            Some(s) => s,
            None    => return vec![],
        };

        let mut errors = Vec::new();
        for idx in 0..arena.len() {
            let node = arena.get(crate::expr::node::NodeId(idx as u32));
            if let ExprNode::Ref(path_ids) = node {
                let path: String = path_ids
                    .iter()
                    .map(|&id| interner.get(id))
                    .collect::<Vec<_>>()
                    .join(".");
                if !schema.contains(&path) {
                    errors.push(PassError::UnknownField { path });
                }
            }
        }
        errors
    }
}

//! [`SecurityPass`] — definitive security gate.
//!
//! Runs last and re-checks anything that could be a security concern:
//! - Function calls must be in the configured allowlist (if one is set).
//! - String literals must not exceed the configured maximum length.

use std::collections::HashSet;

use crate::expr::arena::ExprArena;
use crate::expr::interner::Interner;
use crate::expr::literal::Literal;
use crate::expr::node::ExprNode;
use super::{PassCaps, PassError};

/// An allowlist of function names that may be called.
///
/// If an expression contains a function whose name is not in this set, the
/// [`SecurityPass`] rejects it with [`PassError::DisallowedFunction`].
#[derive(Debug, Clone)]
pub struct AllowList {
    funcs: HashSet<String>,
}

impl AllowList {
    /// Create an empty allowlist (no functions permitted).
    pub fn new() -> Self {
        Self { funcs: HashSet::new() }
    }

    /// Build an allowlist from an iterator of function name strings.
    pub fn from_funcs<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self { funcs: iter.into_iter().map(Into::into).collect() }
    }

    /// Register a function name as allowed.
    pub fn allow(&mut self, name: impl Into<String>) {
        self.funcs.insert(name.into());
    }

    /// Returns `true` if `name` is in the allowlist.
    pub fn permits(&self, name: &str) -> bool {
        self.funcs.contains(name)
    }
}

impl Default for AllowList {
    fn default() -> Self {
        Self::new()
    }
}

/// Final security gate: allowlists and size limits.
pub struct SecurityPass;

impl SecurityPass {
    /// Check all function nodes against `allow` and all string literals against
    /// `caps.max_string_len`.
    ///
    /// If `allow` is `None`, function names are not checked.
    /// If `caps.max_string_len` is `None`, string length is not checked.
    pub fn check(
        &self,
        arena:    &ExprArena,
        interner: &Interner,
        caps:     &PassCaps,
        allow:    Option<&AllowList>,
    ) -> Vec<PassError> {
        let _ = interner; // reserved for future scope checks
        let mut errors = Vec::new();

        for idx in 0..arena.len() {
            let node = arena.get(crate::expr::node::NodeId(idx as u32));
            match node {
                ExprNode::Func(func_node) => {
                    if let Some(list) = allow {
                        let name = arena.func(func_node.id).name();
                        if !list.permits(name) {
                            errors.push(PassError::DisallowedFunction {
                                func_name: name.to_owned(),
                            });
                        }
                    }
                }
                ExprNode::Value(lit) => {
                    if let Some(limit) = caps.max_string_len {
                        let len = string_literal_len(lit);
                        if let Some(len) = len {
                            if len > limit {
                                errors.push(PassError::StringTooLong { length: len, limit });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        errors
    }
}

/// Return the char-count for string-typed literals, `None` for non-strings.
fn string_literal_len(lit: &Literal<'static>) -> Option<usize> {
    match lit {
        Literal::String(s)
        | Literal::Json(s)
        | Literal::Xml(s)
        | Literal::Enum(s) => Some(s.chars().count()),
        _ => None,
    }
}

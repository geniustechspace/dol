//! Expression validation passes — run against a lowered [`ExprArena`].
//!
//! Passes are ordered: each one runs after the previous has succeeded. The
//! ordering is fixed and deliberate:
//!
//! 1. [`NodeCountPass`] — cheapest, fail-fast for runaway expressions
//! 2. [`ArityPass`]     — structural correctness before semantic checks
//! 3. [`ScopePass`]     — field names must exist in the schema (optional)
//! 4. [`SemanticPass`]  — context-sensitive rules (aggregates, windows)
//! 5. [`SecurityPass`]  — definitive gate: allowlists, size limits
//!
//! # Usage
//!
//! ```rust,ignore
//! let errors = run_passes(&arena, &interner, &chain);
//! if !errors.is_empty() { return Err(errors); }
//! ```

pub mod arity;
pub mod node_count;
pub mod scope;
pub mod security;
pub mod semantic;

pub use arity::ArityPass;
pub use node_count::NodeCountPass;
pub use scope::{Schema, ScopePass};
pub use security::{AllowList, SecurityPass};
pub use semantic::{ExprContext, SemanticPass};

use crate::expr::arena::ExprArena;
use crate::expr::interner::Interner;

// ─── PassError ───────────────────────────────────────────────────────────────

/// A structured validation error produced by one of the expression passes.
#[derive(Debug, Clone, PartialEq)]
pub enum PassError {
    /// [`NodeCountPass`]: arena node count exceeds the configured limit.
    TooManyNodes { count: usize, limit: usize },

    /// [`ArityPass`]: a function call has the wrong number of arguments.
    BadArity {
        func_name: String,
        expected:  String,
        actual:    usize,
    },

    /// [`ScopePass`]: a field path is not present in the schema.
    UnknownField { path: String },

    /// [`SemanticPass`]: an aggregate function used in a non-aggregate context.
    AggregateInWrongContext { func_name: String, context: ExprContext },

    /// [`SemanticPass`]: a window function used in a non-windowing context.
    WindowInWrongContext { func_name: String, context: ExprContext },

    /// [`SecurityPass`]: a function name is not in the configured allowlist.
    DisallowedFunction { func_name: String },

    /// [`SecurityPass`]: a string literal exceeds the configured max length.
    StringTooLong { length: usize, limit: usize },
}

impl std::fmt::Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyNodes { count, limit } =>
                write!(f, "expression too large: {count} nodes (limit {limit})"),
            Self::BadArity { func_name, expected, actual } =>
                write!(f, "function '{func_name}': expected {expected} args, got {actual}"),
            Self::UnknownField { path } =>
                write!(f, "unknown field '{path}'"),
            Self::AggregateInWrongContext { func_name, context } =>
                write!(f, "aggregate '{func_name}' not allowed in {context} context"),
            Self::WindowInWrongContext { func_name, context } =>
                write!(f, "window function '{func_name}' not allowed in {context} context"),
            Self::DisallowedFunction { func_name } =>
                write!(f, "function '{func_name}' is not in the allowlist"),
            Self::StringTooLong { length, limit } =>
                write!(f, "string literal too long: {length} chars (limit {limit})"),
        }
    }
}

impl std::error::Error for PassError {}

// ─── PassCaps ────────────────────────────────────────────────────────────────

/// Resource and security limits applied during validation.
#[derive(Debug, Clone)]
pub struct PassCaps {
    /// Maximum number of nodes in a single expression arena.
    ///
    /// Replaces the recursive `MAX_EXPR_DEPTH` bound: node count is a tighter
    /// bound for flat expressions that spread wide rather than deep.
    pub max_nodes: usize,

    /// Maximum byte length of any string literal (if `Some`).
    pub max_string_len: Option<usize>,
}

impl Default for PassCaps {
    fn default() -> Self {
        Self { max_nodes: 4096, max_string_len: None }
    }
}

// ─── Pass chain runner ────────────────────────────────────────────────────────

/// Configuration bundle for the full validation pass chain.
pub struct PassChain<'a> {
    pub caps:     PassCaps,
    pub schema:   Option<&'a Schema>,
    pub context:  ExprContext,
    pub allow:    Option<&'a AllowList>,
}

/// Run all five passes in order, returning the first non-empty error list.
///
/// Stops at the first failing pass so early errors don't mask later noise.
pub fn run_passes(
    arena:    &ExprArena,
    interner: &Interner,
    chain:    &PassChain<'_>,
) -> Vec<PassError> {
    let errs = NodeCountPass.check(arena, &chain.caps);
    if !errs.is_empty() { return errs; }

    let errs = ArityPass.check(arena);
    if !errs.is_empty() { return errs; }

    let errs = ScopePass.check(arena, interner, chain.schema);
    if !errs.is_empty() { return errs; }

    let errs = SemanticPass.check(arena, chain.context);
    if !errs.is_empty() { return errs; }

    SecurityPass.check(arena, interner, &chain.caps, chain.allow)
}

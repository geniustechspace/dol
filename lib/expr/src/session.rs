use smallvec::SmallVec;

use dol_core::policy::{Budget, Limits};

use crate::arena::{ExprArena, FieldNode, FieldStep};
use crate::expr::ExprNode;
use crate::ids::{NodeId, StrId};
use crate::interner::Interner;
use crate::lower::{LowerError, lower_expr_with_budget};
use crate::tree::Expr;

/// Pre-interned ids for high-frequency function/operator names.
pub struct WellKnownNames {
    pub and: StrId,
    pub or: StrId,
    pub not: StrId,
    pub count: StrId,
    pub sum: StrId,
    pub avg: StrId,
    pub min: StrId,
    pub max: StrId,
}

/// A long-lived expression-builder context.
///
/// Holds the arena, interner, and a [`Budget`] that accumulates node
/// charges across every [`BuildSession::lower`] call so a single session
/// can enforce a quota across the whole build pipeline rather than per
/// `lower()` invocation.
pub struct BuildSession {
    pub interner: Interner,
    pub arena: ExprArena,
    pub wkn: WellKnownNames,
    /// Caps applied on every `lower()` call. Mutate via
    /// [`BuildSession::set_limits`].
    limits: Limits,
    /// Long-lived budget seeded from `limits`; node charges accumulate
    /// across calls. Reset on `reset()` and re-seeded by `set_limits`.
    budget: Budget,
}

/// Default limits for a freshly-built session: generous enough for any
/// realistic interactive workload, tight enough to refuse a runaway
/// query before it OOMs the host.
const DEFAULT_LIMITS: Limits = Limits {
    max_nodes: 100_000,
    max_depth: 512,
    // Bytes / string-bytes are not yet charged from `lower`; surface
    // sensible defaults so `BuildSession::set_limits(_)` callers can
    // tighten them when sub-systems start charging.
    max_bytes: 64 * 1024 * 1024,
    max_str_bytes: 1024 * 1024,
};

impl BuildSession {
    /// Build a session with the default [`Limits`] and the well-known
    /// operator-name strings pre-interned.
    pub fn new() -> Self {
        let mut interner = Interner::new();
        let wkn = WellKnownNames {
            and: interner.intern("and"),
            or: interner.intern("or"),
            not: interner.intern("not"),
            count: interner.intern("count"),
            sum: interner.intern("sum"),
            avg: interner.intern("avg"),
            min: interner.intern("min"),
            max: interner.intern("max"),
        };
        Self {
            interner,
            arena: ExprArena::new(),
            wkn,
            limits: DEFAULT_LIMITS,
            budget: Budget::new(DEFAULT_LIMITS),
        }
    }

    /// Reset the arena and interner, and re-seed the budget from the
    /// current limits.
    pub fn reset(&mut self) {
        self.interner.reset();
        self.arena = ExprArena::new();
        self.budget = Budget::new(self.limits);
    }

    /// Borrow the active limits.
    #[inline]
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Borrow the live budget — useful for inspecting how many nodes
    /// have been charged so far.
    #[inline]
    pub fn budget(&self) -> &Budget {
        &self.budget
    }

    /// Replace the active [`Limits`] and re-seed the budget so the new
    /// caps take effect on the next [`BuildSession::lower`] call.
    ///
    /// Re-seeding clears any accumulated node count; if you need to
    /// preserve usage, snapshot `budget()` first.
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
        self.budget = Budget::new(limits);
    }

    /// Lower a tree expression into this session's arena, returning the
    /// root [`NodeId`].
    ///
    /// Unlike the free [`crate::lower::lower_expr`] function, this entry
    /// point honours the session's [`Limits`] and accumulates node
    /// charges across calls. Once a budget is exhausted, the partial
    /// work already pushed into the arena is left in place (lowering is
    /// not transactional — callers that need rollback should snapshot
    /// or clone the arena first).
    pub fn lower(&mut self, expr: &Expr<'_>) -> Result<NodeId, LowerError> {
        lower_expr_with_budget(expr, &mut self.arena, &mut self.interner, &mut self.budget)
    }

    // ── Field / Namespace helpers ─────────────────────────────────────────────

    /// Build a bare (unqualified, no-traversal) field reference.
    ///
    /// Equivalent to a plain column name in SQL: `SELECT id FROM users`.
    pub fn field(&mut self, column: &str) -> NodeId {
        let col_id = self.interner.intern(column);
        let fid = self.arena.alloc_field(FieldNode {
            namespace: None,
            name: col_id,
            steps: SmallVec::new(),
        });
        self.arena.alloc(ExprNode::Field(fid))
    }

    /// Build a qualified field reference (table alias or schema-qualified name).
    ///
    /// Example: `u.id` → `qualified_field("u", "id", [])`.
    pub fn qualified_field(
        &mut self,
        namespace: &str,
        column: &str,
        steps: SmallVec<[FieldStep; 4]>,
    ) -> NodeId {
        let ns_id = self.interner.intern(namespace);
        let col_id = self.interner.intern(column);
        let fid = self.arena.alloc_field(FieldNode {
            namespace: Some(ns_id),
            name: col_id,
            steps,
        });
        self.arena.alloc(ExprNode::Field(fid))
    }

    /// Build a namespace reference (container address without a field).
    ///
    /// Use this when the expression itself *is* a container address, e.g.
    /// when passing a table/bucket path as an operand.
    pub fn namespace(&mut self, path: &str) -> NodeId {
        let id = self.interner.intern(path);
        self.arena.alloc(ExprNode::Namespace(id))
    }

    /// Intern a string and return its [`StrId`] (e.g. for building
    /// [`FieldStep::Key`] values before calling [`qualified_field`]).
    ///
    /// [`qualified_field`]: Self::qualified_field
    pub fn intern(&mut self, s: &str) -> StrId {
        self.interner.intern(s)
    }
}

impl Default for BuildSession {
    fn default() -> Self {
        Self::new()
    }
}

use smallvec::SmallVec;

use crate::arena::{ExprArena, FieldNode, FieldStep};
use crate::expr::ExprNode;
use crate::ids::{NodeId, StrId};
use crate::interner::Interner;
use crate::lower::{LowerError, lower_expr_bounded};
use crate::tree::Expr;

pub struct WellKnownNames {
    pub and: u32,
    pub or: u32,
    pub not: u32,
    pub count: u32,
    pub sum: u32,
    pub avg: u32,
    pub min: u32,
    pub max: u32,
}

pub struct BuildSession {
    pub interner: Interner,
    pub arena: ExprArena,
    pub wkn: WellKnownNames,
    fuel: u32,
    depth: u32,
}

const DEFAULT_FUEL: u32 = 100_000;
const DEFAULT_DEPTH: u32 = 512;

impl BuildSession {
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
            fuel: DEFAULT_FUEL,
            depth: DEFAULT_DEPTH,
        }
    }

    pub fn reset(&mut self) {
        self.interner.reset();
        self.arena = ExprArena::new();
        self.fuel = DEFAULT_FUEL;
        self.depth = DEFAULT_DEPTH;
    }

    pub fn remaining_fuel(&self) -> u32 {
        self.fuel
    }
    pub fn max_depth(&self) -> u32 {
        self.depth
    }

    /// Override the per-lowering allocation budget.
    ///
    /// Each arena allocation during lowering decrements `fuel`; when it
    /// hits zero, lowering returns [`LowerError::FuelExhausted`]. Use this
    /// to harden the build path against pathologically large or
    /// adversarial inputs without changing the default for benign ones.
    pub fn set_fuel(&mut self, fuel: u32) {
        self.fuel = fuel;
    }

    /// Override the maximum recursion depth allowed during lowering.
    ///
    /// Protects the host stack from deep `OR`/`AND` chains. Returns
    /// [`LowerError::DepthExceeded`] when the limit is reached.
    pub fn set_max_depth(&mut self, depth: u32) {
        self.depth = depth;
    }

    /// Lower a tree expression into this session's arena, returning the
    /// root [`NodeId`].
    ///
    /// Unlike the free [`crate::lower::lower_expr`] function, this entry
    /// point honours the session's `fuel` and `depth` budgets, decrementing
    /// `fuel` on every arena allocation and tracking depth across recursive
    /// calls. Once a budget is exhausted, the partial work already pushed
    /// into the arena is left in place (lowering is not transactional —
    /// callers that need rollback should snapshot/clone the arena first).
    pub fn lower(&mut self, expr: &Expr<'_>) -> Result<NodeId, LowerError> {
        // Snapshot the limits and run the bounded entry point. We pass
        // the limits by mutable ref so the lowering helpers can decrement
        // them in place; the session's view stays consistent because we
        // copy the surviving values back when done.
        let mut fuel = self.fuel;
        let mut depth_left = self.depth;
        let res = lower_expr_bounded(
            expr,
            &mut self.arena,
            &mut self.interner,
            &mut fuel,
            &mut depth_left,
            self.depth,
        );
        // Always copy back, even on error, so subsequent operations see the
        // remaining budget.
        self.fuel = fuel;
        res
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

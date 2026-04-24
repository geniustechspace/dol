use smallvec::SmallVec;

use crate::arena::{ExprArena, FieldNode, FieldStep};
use crate::expr::ExprNode;
use crate::ids::{NodeId, StrId};
use crate::interner::Interner;

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

    // ── Field / Namespace helpers ─────────────────────────────────────────────

    /// Build a bare (unqualified, no-traversal) field reference.
    ///
    /// Equivalent to a plain column name in SQL: `SELECT id FROM users`.
    pub fn field(&mut self, column: &str) -> NodeId {
        let col_id = self.interner.intern(column);
        let fid = self.arena.alloc_field(FieldNode {
            namespace: None,
            column: col_id,
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
            column: col_id,
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

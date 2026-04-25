//! [`Program`] — a [`Statement`] paired with the expression arena and
//! interner needed to resolve any [`dol_expr`] arena references it carries.
//!
//! All builders that lower tree expressions return a `Program` so that
//! backends and renderers can be handed a single, self-contained value
//! instead of a `(Statement, ExprArena, Interner)` tuple.

use crate::statement::Statement;

/// A compiled DOL [`Statement`] together with the expression arena and
/// interner required to render any arena-based variants it may contain
/// (DML nodes, `DefinePolicy`, `ObjectSource::FromExpr`, …).
///
/// The fields are public so callers can pattern-match or move out
/// individual parts. Use [`Program::into_parts`] when a tuple is needed.
#[derive(Debug)]
pub struct Program {
    pub stmt: Statement,
    pub arena: dol_expr::ExprArena,
    pub interner: dol_expr::Interner,
}

impl Program {
    /// Construct a `Program` from its constituent parts.
    pub fn new(stmt: Statement, arena: dol_expr::ExprArena, interner: dol_expr::Interner) -> Self {
        Self {
            stmt,
            arena,
            interner,
        }
    }

    /// Construct a `Program` from a fully-owned statement using empty/default
    /// expression storage.
    ///
    /// Useful for statements that do not reference the expression arena
    /// (for example, pure DDL, transaction control, or storage operations),
    /// so callers do not need to manually create an [`ExprArena`] and
    /// [`Interner`] just to satisfy the [`Program`] shape.
    ///
    /// [`ExprArena`]: dol_expr::arena::ExprArena
    /// [`Interner`]:  dol_expr::interner::Interner
    pub fn from_stmt(stmt: Statement) -> Self {
        Self::new(stmt, dol_expr::ExprArena::new(), dol_expr::Interner::new())
    }

    /// Decompose into `(stmt, arena, interner)`.
    pub fn into_parts(self) -> (Statement, dol_expr::ExprArena, dol_expr::Interner) {
        (self.stmt, self.arena, self.interner)
    }
}

impl From<(Statement, dol_expr::ExprArena, dol_expr::Interner)> for Program {
    fn from((stmt, arena, interner): (Statement, dol_expr::ExprArena, dol_expr::Interner)) -> Self {
        Self::new(stmt, arena, interner)
    }
}

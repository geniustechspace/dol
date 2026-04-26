//! [`Program`] — a sequence of v2 [`Operation`]s paired with the expression
//! arena, interner, and (optional) schema catalog.
//!
//! v1 callers that pass a single [`Statement`] continue to work via
//! [`Program::from_stmt`] / `program.stmt`: those helpers wrap the v1
//! statement, expose it through the legacy `stmt` accessor, and synthesise
//! the equivalent v2 [`Operation`] in the `operations` vec via the compat
//! shim in [`crate::compat::statement`].
//!
//! All builders that lower tree expressions return a `Program` so that
//! backends and renderers can be handed a single, self-contained value
//! instead of a `(Statement, ExprArena, Interner)` tuple.

use crate::operation::Operation;
use crate::schema_catalog::SchemaCatalog;
use crate::statement::Statement;

/// A compiled DOL program: a sequence of v2 [`Operation`]s plus the
/// expression arena and interner needed to render any arena references they
/// carry.
///
/// The `stmt` field is preserved for compatibility with v1 callers that have
/// not yet migrated to the [`operations`](Self::operations) vec. New code
/// should prefer `operations` and treat `stmt` as a derived legacy view.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Program {
    /// v2 operation sequence.
    pub operations: alloc::vec::Vec<Operation>,
    /// Legacy v1 single-statement view. Retained for compat with consumers
    /// that have not yet migrated to the [`operations`](Self::operations)
    /// vec. Will be removed in a follow-up once `dol-query` /
    /// `dol-pipeline` / `dol-stream` emit `Operation` directly.
    pub stmt: Statement,
    pub arena: dol_expr::ExprArena,
    pub interner: dol_expr::Interner,
    /// Optional schema catalog. `None` means schemas are addressed solely
    /// through `SchemaBinding::Inferred` / `Opaque`.
    pub schema_catalog: Option<SchemaCatalog>,
}

extern crate alloc;

impl Program {
    /// Construct a `Program` from a v1 [`Statement`] (back-compat shape).
    ///
    /// The `stmt` is preserved verbatim; an equivalent [`Operation`] is also
    /// synthesised through [`crate::compat::statement::statement_to_operation`]
    /// and pushed onto `operations` so v2-aware backends see the same
    /// program.
    #[allow(deprecated)]
    pub fn new(stmt: Statement, arena: dol_expr::ExprArena, interner: dol_expr::Interner) -> Self {
        let op = crate::compat::statement::statement_to_operation(&stmt);
        Self {
            operations: alloc::vec![op],
            stmt,
            arena,
            interner,
            schema_catalog: None,
        }
    }

    /// Construct a `Program` from a fully-owned v1 statement using
    /// empty/default expression storage.
    pub fn from_stmt(stmt: Statement) -> Self {
        Self::new(stmt, dol_expr::ExprArena::new(), dol_expr::Interner::new())
    }

    /// Construct a v2 `Program` from a single [`Operation`] using
    /// empty/default expression storage.
    #[allow(deprecated)]
    pub fn from_operation(op: Operation) -> Self {
        Self {
            operations: alloc::vec![op],
            // Provide a placeholder Statement for the legacy view.
            stmt: Statement::Raw(alloc::string::String::new()),
            arena: dol_expr::ExprArena::new(),
            interner: dol_expr::Interner::new(),
            schema_catalog: None,
        }
    }

    /// Construct a v2 `Program` from a full operation sequence.
    #[allow(deprecated)]
    pub fn from_operations(
        ops: alloc::vec::Vec<Operation>,
        arena: dol_expr::ExprArena,
        interner: dol_expr::Interner,
    ) -> Self {
        Self {
            operations: ops,
            stmt: Statement::Raw(alloc::string::String::new()),
            arena,
            interner,
            schema_catalog: None,
        }
    }

    /// Replace / set the schema catalog.
    pub fn with_catalog(mut self, catalog: SchemaCatalog) -> Self {
        self.schema_catalog = Some(catalog);
        self
    }

    /// Decompose into `(stmt, arena, interner)` (legacy shape).
    #[allow(deprecated)]
    pub fn into_parts(self) -> (Statement, dol_expr::ExprArena, dol_expr::Interner) {
        (self.stmt, self.arena, self.interner)
    }

    /// Borrowed [`crate::ProgramRef`] view.
    pub fn as_ref(&self) -> crate::ProgramRef<'_> {
        crate::ProgramRef::from(self)
    }
}

impl From<(Statement, dol_expr::ExprArena, dol_expr::Interner)> for Program {
    fn from((stmt, arena, interner): (Statement, dol_expr::ExprArena, dol_expr::Interner)) -> Self {
        Self::new(stmt, arena, interner)
    }
}

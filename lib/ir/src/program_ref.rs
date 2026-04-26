//! [`ProgramRef`] — borrowed view into a [`crate::Program`].
//!
//! `ProgramRef` is what backends receive. It carries borrowed references to
//! the operations, expression arena, interner, and schema catalog so backends
//! avoid cloning the arena into every invocation.

use crate::operation::Operation;
use crate::program::Program;
use crate::schema_catalog::SchemaCatalog;

/// Borrowed view into a [`Program`].
#[derive(Clone, Copy, Debug)]
pub struct ProgramRef<'a> {
    pub operations: &'a [Operation],
    pub arena: &'a dol_expr::ExprArena,
    pub interner: &'a dol_expr::Interner,
    pub schema_catalog: Option<&'a SchemaCatalog>,
}

impl<'a> ProgramRef<'a> {
    /// First operation (convenience for single-op programs).
    pub fn first(&self) -> Option<&'a Operation> {
        self.operations.first()
    }
}

impl<'a> From<&'a Program> for ProgramRef<'a> {
    fn from(p: &'a Program) -> Self {
        ProgramRef {
            operations: p.operations.as_slice(),
            arena: &p.arena,
            interner: &p.interner,
            schema_catalog: p.schema_catalog.as_ref(),
        }
    }
}

//! [`Program`] — a sequence of [`Operation`]s paired with the expression
//! arena, interner, and (optional) schema catalog.

extern crate alloc;

use crate::operation::Operation;
use crate::schema_catalog::SchemaCatalog;

/// A compiled DOL program: a sequence of [`Operation`]s plus the expression
/// arena and interner needed to render any arena references they carry.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{Insert, InsertSource};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Program;
///
/// let insert = Insert {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::new(0))),
///     source: InsertSource::Bindings,
///     returning: None,
/// };
///
/// let program = Program::from_operation(insert.into());
/// assert_eq!(program.operations.len(), 1);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Program {
    /// Operation sequence.
    pub operations: alloc::vec::Vec<Operation>,
    /// Expression arena containing any expression trees.
    pub arena: dol_expr::ExprArena,
    /// String interner for resolving symbols.
    pub interner: dol_expr::Interner,
    /// Optional schema catalog. `None` means schemas are addressed solely
    /// through `SchemaBinding::Inferred` / `Opaque`.
    pub schema_catalog: Option<SchemaCatalog>,
}

impl Program {
    /// Construct a `Program` from a single [`Operation`] with the supplied
    /// expression storage.
    pub fn new(op: Operation, arena: dol_expr::ExprArena, interner: dol_expr::Interner) -> Self {
        Self {
            operations: alloc::vec![op],
            arena,
            interner,
            schema_catalog: None,
        }
    }

    /// Construct a `Program` from a single [`Operation`] using empty/default
    /// expression storage.
    pub fn from_operation(op: Operation) -> Self {
        Self::new(op, dol_expr::ExprArena::new(), dol_expr::Interner::new())
    }

    /// Construct a `Program` from a full operation sequence.
    pub fn from_operations(
        ops: alloc::vec::Vec<Operation>,
        arena: dol_expr::ExprArena,
        interner: dol_expr::Interner,
    ) -> Self {
        Self {
            operations: ops,
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

    /// Borrowed [`crate::ProgramRef`] view.
    pub fn as_ref(&self) -> crate::ProgramRef<'_> {
        crate::ProgramRef::from(self)
    }
}

impl From<(Operation, dol_expr::ExprArena, dol_expr::Interner)> for Program {
    fn from((op, arena, interner): (Operation, dol_expr::ExprArena, dol_expr::Interner)) -> Self {
        Self::new(op, arena, interner)
    }
}

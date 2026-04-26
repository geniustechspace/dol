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

    /// Append the operations of `other` to this program.
    ///
    /// This is the recommended way to compose `tx_begin()`, DML, and
    /// `tx_commit()` into a single program, replacing the previous pattern
    /// of three independent [`Program`]s with three independent arenas.
    ///
    /// # Preconditions
    ///
    /// At least one side must be **arena/interner-empty** — i.e. its
    /// `arena.len() == 0` and `interner.len() == 0`. This covers every
    /// helper in [`dol_query::control`] that produces a control-only
    /// `Program` (`tx_begin`, `tx_commit`, plain `tx_rollback` whose
    /// interner stays empty when the catalog supplies no savepoint label).
    ///
    /// Composing two programs that **both** carry arena nodes or interned
    /// strings would require id remapping (a visitor across every
    /// `Operation` variant); when the IR shape lands as a unified `WriteBody`
    /// (Stage 2 follow-up PR) we can lift that restriction. Until then,
    /// prefer [`crate::operation::TxOp::Atomic`] (composed via
    /// [`dol_query::control::tx_atomic`]) for combining DML payloads.
    ///
    /// # Panics
    ///
    /// Panics with a clear diagnostic if both sides carry arena nodes or
    /// interned strings; this is a design-time error.
    ///
    /// # Examples
    ///
    /// ```
    /// use dol_ir::operation::{Insert, InsertSource, Operation, TxBegin, TxOp, TxOptions};
    /// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
    /// use dol_ir::Program;
    ///
    /// let begin = Program::from_operation(Operation::Tx(Box::new(TxOp::Begin(
    ///     TxBegin { opts: TxOptions::default() },
    /// ))));
    /// let commit = Program::from_operation(Operation::Tx(Box::new(TxOp::Commit)));
    /// let dml = Program::from_operation(
    ///     Insert {
    ///         target: Target::new(TargetKind::Relation, Locator::new(Symbol::new(0))),
    ///         source: InsertSource::Bindings,
    ///         returning: None,
    ///     }
    ///     .into(),
    /// );
    ///
    /// let mut prog = begin;
    /// prog.extend(dml).extend(commit);
    /// assert_eq!(prog.operations.len(), 3);
    /// ```
    pub fn extend(&mut self, other: Program) -> &mut Self {
        let self_empty = self.arena.is_empty() && self.interner.is_empty();
        let other_empty = other.arena.is_empty() && other.interner.is_empty();
        if !self_empty && !other_empty {
            panic!(
                "Program::extend: both programs carry arena nodes or interned \
                 strings (self.arena={}, self.interner={}, other.arena={}, \
                 other.interner={}); id remapping is not yet implemented. \
                 Use TxOp::Atomic (dol_query::control::tx_atomic) to compose \
                 DML payloads.",
                self.arena.len(),
                self.interner.len(),
                other.arena.len(),
                other.interner.len(),
            );
        }
        let Program {
            mut operations,
            arena,
            interner,
            schema_catalog,
        } = other;
        if self_empty {
            // Adopt other's storage.
            self.arena = arena;
            self.interner = interner;
        }
        if self.schema_catalog.is_none() {
            self.schema_catalog = schema_catalog;
        }
        self.operations.append(&mut operations);
        self
    }
}

impl From<(Operation, dol_expr::ExprArena, dol_expr::Interner)> for Program {
    fn from((op, arena, interner): (Operation, dol_expr::ExprArena, dol_expr::Interner)) -> Self {
        Self::new(op, arena, interner)
    }
}

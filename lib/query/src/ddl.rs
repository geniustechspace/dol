//! Schema-definition (DDL) helpers — produce [`Operation::Schema`],
//! [`Operation::Field`], [`Operation::Lookup`] programs.
//!
//! Higher-level builders return [`dol_ir::Program`] directly. They take
//! ownership of name / namespace / field metadata and lower it onto the
//! [`Target`](dol_ir::Target) + [`SchemaRef`] shape.

use dol_expr::{ExprArena, Interner};
use dol_ir::operation::{FieldOp, IndexOp, LookupOp, SchemaOp};
use dol_ir::{Operation, Program, SchemaRef, TargetKind};
use dol_schema::Entity;

use crate::target::target_from_parts;

/// Emit a `CREATE TABLE` / `CREATE COLLECTION` / … operation against a
/// relation target.
///
/// `schema` should reference an entry in the program's
/// [`SchemaCatalog`](dol_ir::SchemaCatalog). When the catalog is unknown,
/// pass [`SchemaRef::default`] and supply a `SchemaBinding::Inferred`
/// target instead via [`define_entity_inferred`].
pub fn define_entity(
    name: &str,
    namespace: Option<&str>,
    schema: SchemaRef,
    if_not_exists: bool,
) -> Program {
    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, name, namespace);
    let op: Operation = SchemaOp::create_entity(target, schema, if_not_exists).into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit a `CREATE TABLE … IF NOT EXISTS` against an inferred-schema target.
pub fn define_entity_inferred(name: &str, namespace: Option<&str>, if_not_exists: bool) -> Program {
    define_entity(name, namespace, SchemaRef::default(), if_not_exists)
}

/// Emit a `DROP TABLE` operation against a relation target.
pub fn drop_entity(name: &str, namespace: Option<&str>) -> Program {
    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, name, namespace);
    let op: Operation = SchemaOp::drop_(target).into();
    Program::new(op, ExprArena::new(), interner)
}

/// Lower an [`Entity`] to a `define_entity` call. The schema body itself is
/// expected to live in the catalog; this helper only emits the structural
/// operation that points at it.
pub fn define_from_entity(entity: &Entity, schema: SchemaRef, if_not_exists: bool) -> Program {
    define_entity(
        entity.name.as_ref(),
        entity.namespace.as_deref(),
        schema,
        if_not_exists,
    )
}

// ── Lookup (index) ────────────────────────────────────────────────────────

/// Emit a `CREATE INDEX` (`Operation::Lookup { verb: Create, … }`).
pub fn define_lookup(
    table: &str,
    namespace: Option<&str>,
    name: &str,
    columns: &[&str],
    unique: bool,
    if_not_exists: bool,
) -> Program {
    use dol_ir::operation::{LookupMethod, StructuralVerb};
    use smallvec::SmallVec;

    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, table, namespace);
    let name_sym = crate::target::intern_symbol(&mut interner, name);
    let cols: SmallVec<[dol_ir::Symbol; 2]> = columns
        .iter()
        .map(|c| crate::target::intern_symbol(&mut interner, c))
        .collect();
    let op: Operation = LookupOp {
        verb: StructuralVerb::Create,
        target,
        name: name_sym,
        method: LookupMethod::Tree,
        fields: cols,
        unique,
        if_not_exists,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit a `DROP INDEX` operation.
pub fn drop_lookup(table: &str, namespace: Option<&str>, name: &str, if_exists: bool) -> Program {
    use dol_ir::operation::{LookupMethod, StructuralVerb};
    use smallvec::SmallVec;

    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, table, namespace);
    let name_sym = crate::target::intern_symbol(&mut interner, name);
    let op: Operation = LookupOp {
        verb: StructuralVerb::Drop,
        target,
        name: name_sym,
        method: LookupMethod::Tree,
        fields: SmallVec::new(),
        unique: false,
        // Reuse the same flag as `if_exists` for the Drop verb (see
        // `LookupOp::if_not_exists`).
        if_not_exists: if_exists,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

// ── Field-level alters ────────────────────────────────────────────────────

/// Emit an `ALTER TABLE … DROP COLUMN`.
pub fn drop_field(table: &str, namespace: Option<&str>, field: &str) -> Program {
    use dol_ir::operation::StructuralVerb;

    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, table, namespace);
    let field_sym = crate::target::intern_symbol(&mut interner, field);
    let op: Operation = FieldOp {
        verb: StructuralVerb::Drop,
        target,
        field: field_sym,
        def: None,
        new_name: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit an `ALTER TABLE … RENAME COLUMN`.
pub fn rename_field(table: &str, namespace: Option<&str>, from: &str, to: &str) -> Program {
    use dol_ir::operation::StructuralVerb;

    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, table, namespace);
    let from_sym = crate::target::intern_symbol(&mut interner, from);
    let to_sym = crate::target::intern_symbol(&mut interner, to);
    let op: Operation = FieldOp {
        verb: StructuralVerb::Rename,
        target,
        field: from_sym,
        def: None,
        new_name: Some(to_sym),
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit a `CREATE INDEX` via the [`IndexOp`] surface (the secondary index
/// noun, distinct from `LookupOp`).
pub fn define_index(
    table: &str,
    namespace: Option<&str>,
    name: &str,
    keys: smallvec::SmallVec<[dol_ir::operation::IndexKey; 2]>,
    unique: bool,
    method: dol_ir::operation::IndexMethod,
    if_not_exists: bool,
) -> Program {
    use dol_ir::operation::StructuralVerb;

    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, table, namespace);
    let name_sym = crate::target::intern_symbol(&mut interner, name);
    let op: Operation = IndexOp {
        verb: StructuralVerb::Create,
        target,
        name: name_sym,
        method,
        keys,
        unique,
        predicate: None,
        if_not_exists,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

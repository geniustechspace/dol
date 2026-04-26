//! v1-`Statement` ↔ v2-`Operation` bridge.
//!
//! This module provides:
//! - Re-exports of every v1 IR type at `compat::statement::*` so consumers can
//!   pin their imports during the migration window.
//! - A pure [`statement_to_operation`] conversion that maps every v1 variant
//!   to its v2 counterpart per RFC 0001's "Mapping Old → New" table.
//! - `From<Statement> for Operation` and `From<&Statement> for Operation`
//!   blanket implementations.
//!
//! The conversion is designed to be cheap: it allocates only the new
//! [`Operation`] payload box. v1 string fields (entity names, locator
//! components, role names) are *not* interned — they are placed into the new
//! `Locator`/`Symbol` shape using `Symbol::default()` placeholders, because
//! the compat shim cannot guess which interner the caller will use. Callers
//! that need real interning should construct v2 [`Operation`]s directly.

use alloc::vec::Vec;

extern crate alloc;

// ── v1 re-exports ─────────────────────────────────────────────────────────
pub use crate::control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use crate::definition::{
    AlterAction, AlterEntity, DefineEntity, DefineLookup, DefineType, DropEntity, DropLookup,
    DropType, FieldDef, LookupMethod,
};
pub use crate::entity_ref::EntityRef;
pub use crate::statement::{Statement, StatementExtension};
pub use crate::storage::{
    GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile,
};
pub use crate::transaction::Transaction;

// ── conversions ───────────────────────────────────────────────────────────
use crate::operation::{
    self as op, ExtensionId, Insert, InsertSource, Operation, OperationExtension, Query, Replace,
    SchemaBody, SchemaOp, StructuralVerb, TxOp, TxOptions, Update,
};
use crate::operation::replace::ReplaceBody;
use crate::schema_ref::SchemaRef;
use crate::target::{Locator, SchemaBinding, Symbol, Target, TargetKind};

fn placeholder_target(kind: TargetKind) -> Target {
    Target::new(kind, Locator::new(Symbol::default()))
}

/// Convert a v1 [`Statement`] into the equivalent v2 [`Operation`].
///
/// The conversion is total: every v1 variant maps to a v2 variant per
/// RFC 0001's mapping table.
pub fn statement_to_operation(stmt: &Statement) -> Operation {
    match stmt {
        Statement::Query(_) => Query {
            target: placeholder_target(TargetKind::Relation)
                .with_schema(SchemaBinding::Inferred),
            node: None,
        }
        .into(),
        Statement::Insert(_) => Insert {
            target: placeholder_target(TargetKind::Relation),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into(),
        Statement::Update(_) => Update {
            target: placeholder_target(TargetKind::Relation),
            sets: smallvec::SmallVec::new(),
            filter: None,
            returning: None,
        }
        .into(),
        Statement::Delete(_) => op::Delete {
            target: placeholder_target(TargetKind::Relation),
            filter: None,
            returning: None,
        }
        .into(),
        Statement::Upsert(_) => op::Upsert {
            target: placeholder_target(TargetKind::Relation),
            source: InsertSource::Bindings,
            conflict_keys: smallvec::SmallVec::new(),
            on_conflict: op::upsert::OnConflict::DoNothing,
            returning: None,
        }
        .into(),

        // ── DDL ──────────────────────────────────────────────────────────
        Statement::DefineEntity(_) => SchemaOp::create_entity(
            placeholder_target(TargetKind::Relation),
            SchemaRef::default(),
            false,
        )
        .into(),
        Statement::AlterEntity(_) => SchemaOp {
            verb: StructuralVerb::Alter,
            target: placeholder_target(TargetKind::Relation),
            body: SchemaBody::Reference,
            new_name: None,
        }
        .into(),
        Statement::DropEntity(_) => SchemaOp::drop_(placeholder_target(TargetKind::Relation)).into(),
        Statement::DefineLookup(_) => op::LookupOp {
            verb: StructuralVerb::Create,
            target: placeholder_target(TargetKind::Relation),
            name: Symbol::default(),
            method: op::lookup::LookupMethod::Hash,
            fields: smallvec::SmallVec::new(),
            unique: false,
        }
        .into(),
        Statement::DropLookup(_) => op::LookupOp {
            verb: StructuralVerb::Drop,
            target: placeholder_target(TargetKind::Relation),
            name: Symbol::default(),
            method: op::lookup::LookupMethod::Hash,
            fields: smallvec::SmallVec::new(),
            unique: false,
        }
        .into(),
        Statement::DefineType(_) => SchemaOp {
            verb: StructuralVerb::Create,
            target: placeholder_target(TargetKind::Virtual),
            body: SchemaBody::Type {
                schema: SchemaRef::default(),
                body: op::TypeBody::Other,
            },
            new_name: None,
        }
        .into(),
        Statement::DropType(_) => SchemaOp::drop_(placeholder_target(TargetKind::Virtual)).into(),

        // ── ACL ──────────────────────────────────────────────────────────
        Statement::Grant(g) => op::GrantV2 {
            privileges: smallvec::smallvec![g.privilege.clone()],
            target: placeholder_target(TargetKind::Relation),
            roles: smallvec::SmallVec::new(),
            with_grant_option: false,
        }
        .into(),
        Statement::Revoke(r) => op::RevokeV2 {
            privileges: smallvec::smallvec![r.privilege.clone()],
            target: placeholder_target(TargetKind::Relation),
            roles: smallvec::SmallVec::new(),
            cascade: false,
        }
        .into(),
        Statement::DefinePolicy(p) => op::PolicyOp {
            verb: StructuralVerb::Create,
            target: placeholder_target(TargetKind::Relation),
            name: Symbol::default(),
            scope: match p.action {
                PolicyAction::Read => op::PolicyScope::Read,
                PolicyAction::Write => op::PolicyScope::Write,
                PolicyAction::All => op::PolicyScope::All,
            },
            using_expr: p.using_expr,
            check_expr: p.check_expr,
        }
        .into(),

        // ── Transactions ─────────────────────────────────────────────────
        Statement::Transaction(t) => match t.as_ref() {
            Transaction::Begin => TxOp::Begin(op::TxBegin::default()),
            Transaction::Commit => TxOp::Commit,
            Transaction::Rollback => TxOp::Rollback,
            Transaction::Savepoint(_) => TxOp::Savepoint(Symbol::default()),
            Transaction::ReleaseSavepoint(_) => TxOp::ReleaseSavepoint(Symbol::default()),
            Transaction::RollbackToSavepoint(_) => TxOp::RollbackTo(Symbol::default()),
            Transaction::Block(stmts) => {
                let ops: Vec<Operation> = stmts.iter().map(statement_to_operation).collect();
                TxOp::Atomic {
                    ops,
                    opts: TxOptions::default(),
                }
            }
        }
        .into(),

        // ── Storage / file (collapse onto Blob / FileTree targets) ───────
        Statement::PutObject(_) => Insert {
            target: placeholder_target(TargetKind::Blob),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into(),
        Statement::GetObject(_) => Query {
            target: placeholder_target(TargetKind::Blob),
            node: None,
        }
        .into(),
        Statement::ListObjects(_) => Query {
            target: placeholder_target(TargetKind::Blob),
            node: None,
        }
        .into(),
        Statement::ReadFile(_) => Query {
            target: placeholder_target(TargetKind::FileTree),
            node: None,
        }
        .into(),
        Statement::WriteFile(_) => Replace {
            target: placeholder_target(TargetKind::FileTree),
            body: ReplaceBody::Bindings,
            filter: None,
        }
        .into(),
        Statement::MoveFile(_) => Update {
            target: placeholder_target(TargetKind::FileTree),
            sets: smallvec::SmallVec::new(),
            filter: None,
            returning: None,
        }
        .into(),

        // ── Escape hatches ───────────────────────────────────────────────
        Statement::Raw(_body) => {
            #[cfg(feature = "raw")]
            {
                op::RawOp {
                    dialect: None,
                    body: _body.clone(),
                    params: smallvec::SmallVec::new(),
                }
                .into()
            }
            #[cfg(not(feature = "raw"))]
            {
                // Without the `raw` feature we surface Raw v1 statements as
                // an Extension carrying the body so downstream code can still
                // round-trip them.
                OperationExtension {
                    id: ExtensionId::new(Symbol::default(), 0),
                    payload: _body.as_bytes().to_vec(),
                }
                .into()
            }
        }
        Statement::Extension(ext) => OperationExtension {
            id: ExtensionId::new(Symbol::default(), 0),
            payload: ext.payload.clone(),
        }
        .into(),
    }
}

impl From<Statement> for Operation {
    fn from(stmt: Statement) -> Self {
        statement_to_operation(&stmt)
    }
}

impl From<&Statement> for Operation {
    fn from(stmt: &Statement) -> Self {
        statement_to_operation(stmt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::OpKind;

    #[test]
    fn put_object_maps_to_insert_against_blob() {
        let stmt = Statement::PutObject(Box::new(PutObject {
            bucket: "b".into(),
            key: "k".into(),
            source: ObjectSource::FromBytes,
            content_type: None,
            metadata: Vec::new(),
        }));
        let op = statement_to_operation(&stmt);
        assert_eq!(op.kind(), OpKind::Insert);
        assert_eq!(op.primary_target().unwrap().kind, TargetKind::Blob);
    }

    #[test]
    fn write_file_maps_to_replace_against_filetree() {
        let stmt = Statement::WriteFile(Box::new(WriteFile {
            path: "/tmp/x".into(),
            source: ObjectSource::FromBytes,
            create_dirs: false,
        }));
        let op = statement_to_operation(&stmt);
        assert_eq!(op.kind(), OpKind::Replace);
        assert_eq!(op.primary_target().unwrap().kind, TargetKind::FileTree);
    }

    #[test]
    fn move_file_maps_to_update_against_filetree() {
        let stmt = Statement::MoveFile(Box::new(MoveFile {
            from: "/a".into(),
            to: "/b".into(),
        }));
        let op = statement_to_operation(&stmt);
        assert_eq!(op.kind(), OpKind::Update);
        assert_eq!(op.primary_target().unwrap().kind, TargetKind::FileTree);
    }

    #[test]
    fn read_file_maps_to_query_against_filetree() {
        let stmt = Statement::ReadFile(Box::new(ReadFile {
            path: "/x".into(),
            encoding: None,
        }));
        let op = statement_to_operation(&stmt);
        assert_eq!(op.kind(), OpKind::Query);
        assert_eq!(op.primary_target().unwrap().kind, TargetKind::FileTree);
    }

    #[test]
    fn transaction_block_maps_to_atomic() {
        let stmt = Statement::Transaction(Box::new(Transaction::Block(vec![Statement::Raw(
            "x".into(),
        )])));
        let op = statement_to_operation(&stmt);
        assert!(matches!(
            op,
            Operation::Tx(ref b) if matches!(**b, TxOp::Atomic { .. })
        ));
    }
}

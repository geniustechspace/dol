use dol_expr::expr::{DeleteNode, InsertNode, QueryNode, UpdateNode, UpsertNode};

use crate::control::{DefinePolicy, Grant, Revoke};
use crate::definition::{
    AlterEntity, DefineEntity, DefineIndex, DefineType, DropEntity, DropIndex, DropType,
};
use crate::storage::{GetObject, ListObjects, MoveFile, PutObject, ReadFile, WriteFile};
use crate::transaction::Transaction;

/// The universal DOL statement — dispatch enum for all operation types.
///
/// Variants are split into three groups:
/// - **DML** (arena-based): `Query`, `Insert`, `Update`, `Delete`, `Upsert` — contain
///   [`dol_expr`] arena IDs and require an [`ExprArena`] + [`Interner`] for rendering.
/// - **DDL / control / storage / transaction**: contain only owned data; renderable
///   without an arena.
/// - **Raw**: an escape hatch for pre-built SQL / KV / other backend strings.
///
/// [`ExprArena`]: dol_expr::arena::ExprArena
/// [`Interner`]:  dol_expr::interner::Interner
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Statement {
    // ── DML (arena-based) ──────────────────────────────────────────────────
    Query(QueryNode),
    Insert(InsertNode),
    Update(UpdateNode),
    Delete(DeleteNode),
    Upsert(UpsertNode),

    // ── DDL ───────────────────────────────────────────────────────────────
    DefineEntity(Box<DefineEntity>),
    AlterEntity(AlterEntity),
    DropEntity(DropEntity),
    DefineIndex(DefineIndex),
    DropIndex(DropIndex),
    DefineType(DefineType),
    DropType(DropType),

    // ── Access control ────────────────────────────────────────────────────
    Grant(Grant),
    Revoke(Revoke),
    DefinePolicy(DefinePolicy),

    // ── Transaction ───────────────────────────────────────────────────────
    Transaction(Transaction),

    // ── Storage ───────────────────────────────────────────────────────────
    PutObject(PutObject),
    GetObject(GetObject),
    ListObjects(ListObjects),
    ReadFile(ReadFile),
    WriteFile(WriteFile),
    MoveFile(MoveFile),

    // ── Escape hatch ──────────────────────────────────────────────────────
    Raw(String),
}

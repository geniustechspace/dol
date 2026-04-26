use dol_expr::expr::{DeleteNode, InsertNode, QueryNode, UpdateNode, UpsertNode};

use crate::control::{DefinePolicy, Grant, Revoke};
use crate::definition::{
    AlterEntity, DefineEntity, DefineLookup, DefineType, DropEntity, DropLookup, DropType,
};
use crate::storage::{GetObject, ListObjects, MoveFile, PutObject, ReadFile, WriteFile};
use crate::transaction::Transaction;

/// The universal DOL statement — dispatch enum for all operation types.
///
/// Variants are split into three groups:
/// - **DML** (arena-based): `Query`, `Insert`, `Update`, `Delete`, `Upsert` — contain
///   [`dol_expr`] arena IDs and require an [`ExprArena`] + [`Interner`] for rendering.
/// - **DDL / control / storage / transaction**: mostly owned data, but some control
///   and storage variants can also contain expression arena IDs (for example,
///   `DefinePolicy` or storage sources derived from expressions) and may therefore
///   require an [`ExprArena`] + [`Interner`] for rendering.
/// - **Raw**: an escape hatch for pre-built SQL / KV / other backend strings.
///
/// ## Size budget
///
/// `Statement` keeps `size_of::<Statement>() ≤ 64` (verified at build time by
/// `xtask size`). Every "heavy" payload — anything whose owned representation
/// would push the variant over that budget — is held behind a `Box`. The
/// pattern-matching ergonomics are unaffected: `Statement::Query(q)` still
/// produces a `q: &Box<QueryNode>` that derefs to `&QueryNode`.
///
/// [`ExprArena`]: dol_expr::arena::ExprArena
/// [`Interner`]:  dol_expr::interner::Interner
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Statement {
    // ── DML (arena-based) ──────────────────────────────────────────────────
    Query(Box<QueryNode>),
    Insert(Box<InsertNode>),
    Update(Box<UpdateNode>),
    Delete(Box<DeleteNode>),
    Upsert(Box<UpsertNode>),

    // ── DDL ───────────────────────────────────────────────────────────────
    DefineEntity(Box<DefineEntity>),
    AlterEntity(Box<AlterEntity>),
    DropEntity(Box<DropEntity>),
    DefineLookup(Box<DefineLookup>),
    DropLookup(Box<DropLookup>),
    DefineType(Box<DefineType>),
    DropType(Box<DropType>),

    // ── Access control ────────────────────────────────────────────────────
    Grant(Box<Grant>),
    Revoke(Box<Revoke>),
    DefinePolicy(Box<DefinePolicy>),

    // ── Transaction ───────────────────────────────────────────────────────
    Transaction(Box<Transaction>),

    // ── Storage ───────────────────────────────────────────────────────────
    PutObject(Box<PutObject>),
    GetObject(Box<GetObject>),
    ListObjects(Box<ListObjects>),
    ReadFile(Box<ReadFile>),
    WriteFile(Box<WriteFile>),
    MoveFile(Box<MoveFile>),

    // ── Escape hatch ──────────────────────────────────────────────────────
    Raw(String),

    // ── Extension seam (open) ─────────────────────────────────────────────
    /// Open extension carrying a registered identifier and an opaque payload.
    ///
    /// Higher-level crates (notably `dol-stream` and `dol-pipeline`) attach
    /// new verbs to the IR through this variant rather than extending the
    /// closed enum, so streaming/IoT vocabulary can evolve independently of
    /// the core. The `id` is a stable `&'static str` registered by the
    /// emitting crate; the `payload` is its postcard-encoded body.
    Extension(Box<StatementExtension>),
}

// ── Ergonomic constructors ────────────────────────────────────────────────
//
// These `From` impls let callers write `Statement::from(qnode)` instead of
// `Statement::Query(Box::new(qnode))` and are the recommended way to build a
// `Statement` from an owned payload. The matching `Statement::Query(...)`
// constructors are unchanged and still take a `Box<QueryNode>` directly.

macro_rules! impl_stmt_from {
    ($( $variant:ident($payload:ty) ; )+) => {
        $(
            impl From<$payload> for Statement {
                #[inline]
                fn from(value: $payload) -> Self {
                    Statement::$variant(Box::new(value))
                }
            }
        )+
    };
}

impl_stmt_from! {
    Query(QueryNode);
    Insert(InsertNode);
    Update(UpdateNode);
    Delete(DeleteNode);
    Upsert(UpsertNode);
    DefineEntity(DefineEntity);
    AlterEntity(AlterEntity);
    DropEntity(DropEntity);
    DefineLookup(DefineLookup);
    DropLookup(DropLookup);
    DefineType(DefineType);
    DropType(DropType);
    Grant(Grant);
    Revoke(Revoke);
    DefinePolicy(DefinePolicy);
    Transaction(Transaction);
    PutObject(PutObject);
    GetObject(GetObject);
    ListObjects(ListObjects);
    ReadFile(ReadFile);
    WriteFile(WriteFile);
    MoveFile(MoveFile);
    Extension(StatementExtension);
}

/// Open extension payload attached via [`Statement::Extension`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatementExtension {
    /// Stable extension identifier (e.g. `"dol-stream/window"`).
    pub id: String,
    /// Opaque, codec-encoded payload understood by the registering crate.
    pub payload: Vec<u8>,
}

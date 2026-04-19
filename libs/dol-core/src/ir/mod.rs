//! Backward compatibility — re-exports from [`crate::op`].

pub mod control;
pub mod definition;
pub mod mutation;
pub mod query;
pub mod storage;
pub mod transaction;

// ── Re-export every canonical type from `op` ──

pub use crate::op::{EntityRef, BackendError, Operation, Statement};

pub use crate::op::{
    // query
    Query, CompoundQuery, Join, JoinKind, SetOp, OffsetLimit, LockMode,
    // mutation
    Insert, InsertSelect, Update, Remove, Upsert,
    // definition
    DefineEntity, AlterEntity, DropEntity, DefineIndex, DropIndex, DefineType, DropType,
    FieldDef, AlterAction, IndexMethod, OwnedEntityConstraint, OwnedForeignKeyRef,
    Constraint, ForeignKeyDef,
    // control
    Grant, Revoke, DefinePolicy, PolicyAction, Privilege,
    // storage
    PutObject, GetObject, ListObjects, ReadFile, WriteFile, MoveFile, ObjectSource,
    // transaction
    Transaction,
};

// ── Deprecated type aliases for old IR-suffixed names ──

pub type QueryIR<'a> = Query<'a>;
pub type CompoundQueryIR<'a> = CompoundQuery<'a>;
pub type JoinIR = Join;
pub type InsertIR = Insert;
pub type InsertSelectIR = InsertSelect;
pub type UpdateIR<'a> = Update<'a>;
pub type RemoveIR<'a> = Remove<'a>;
pub type UpsertIR<'a> = Upsert<'a>;
pub type DefineEntityIR = DefineEntity;
pub type AlterEntityIR = AlterEntity;
pub type DropEntityIR = DropEntity;
pub type DefineIndexIR = DefineIndex;
pub type DropIndexIR = DropIndex;
pub type DefineTypeIR = DefineType;
pub type DropTypeIR = DropType;
pub type GrantIR = Grant;
pub type RevokeIR = Revoke;
pub type DefinePolicyIR<'a> = DefinePolicy<'a>;
pub type PutObjectIR<'a> = PutObject<'a>;
pub type GetObjectIR = GetObject;
pub type ListObjectsIR = ListObjects;
pub type ReadFileIR = ReadFile;
pub type WriteFileIR<'a> = WriteFile<'a>;
pub type MoveFileIR = MoveFile;
pub type TransactionIR<'a> = Transaction<'a>;

// Aliases for renamed non-IR types
pub type JoinType = JoinKind;
pub type SetOpKind = SetOp;

//! Builder API — composable method-chain builders that produce IR.
//!
//! These builders take a `&Entity` reference and produce backend-agnostic IR.

pub mod control;
pub mod mutation;
pub mod query;
pub mod storage;
pub mod transaction;

pub use control::{DefinePolicyBuilder, GrantBuilder, RevokeBuilder};
pub use mutation::{
    InsertBuilder, InsertSelect, InsertSelectBuilder, DeleteBuilder, UpdateBuilder, UpsertBuilder,
};
#[allow(deprecated)]
pub use mutation::RemoveBuilder;
pub use query::GetBuilder;
pub use storage::{
    GetObjectBuilder, ListObjectsBuilder, MoveFileBuilder, PutObjectBuilder, ReadFileBuilder,
    WriteFileBuilder,
};
pub use transaction::TransactionBuilder;

pub use control::Privilege;

// ---------------------------------------------------------------------------
// EntityBuilderExt — CRUD + DDL builder entry-points on Entity
// ---------------------------------------------------------------------------

use dol_schema::{Entity, AlterEntityBuilder, CreateFromMeta, DropEntityBuilder};

/// Extension trait providing builder entry-point methods on [`Entity`].
///
/// Import this trait to use `entity.get()`, `entity.insert()`, etc.
pub trait EntityBuilderExt {
    fn get(&self) -> GetBuilder<'_>;
    fn insert(&self) -> InsertBuilder<'_>;
    fn insert_select(&self) -> InsertSelectBuilder<'_>;
    fn update(&self) -> UpdateBuilder<'_>;
    fn delete(&self) -> DeleteBuilder<'_>;
    fn upsert(&self) -> UpsertBuilder<'_>;
    fn alter(&self) -> AlterEntityBuilder<'_>;
    fn drop_entity(&self) -> DropEntityBuilder<'_>;
    fn create(&self) -> CreateFromMeta<'_>;
}

impl EntityBuilderExt for Entity {
    fn get(&self) -> GetBuilder<'_> {
        GetBuilder::new(self)
    }

    fn insert(&self) -> InsertBuilder<'_> {
        InsertBuilder::new(self)
    }

    fn insert_select(&self) -> InsertSelectBuilder<'_> {
        InsertSelectBuilder::new(self)
    }

    fn update(&self) -> UpdateBuilder<'_> {
        UpdateBuilder::new(self)
    }

    fn delete(&self) -> DeleteBuilder<'_> {
        DeleteBuilder::new(self)
    }

    fn upsert(&self) -> UpsertBuilder<'_> {
        UpsertBuilder::new(self)
    }

    fn alter(&self) -> AlterEntityBuilder<'_> {
        AlterEntityBuilder::new(self)
    }

    fn drop_entity(&self) -> DropEntityBuilder<'_> {
        DropEntityBuilder::new(self)
    }

    fn create(&self) -> CreateFromMeta<'_> {
        CreateFromMeta::new(self)
    }
}

//! # dol-builder — DOL Builder API
//!
//! Composable method-chain APIs that produce DOL IR.
//!
//! Every builder follows the same pattern:
//! 1. Create via `Model::get()`, `Model::insert()`, etc.
//! 2. Chain configuration methods
//! 3. Call `.build()` to produce IR
//!
//! For SQL rendering, import the extension traits from `dol-sql`.

pub mod control;
pub mod definition;
pub mod mutation;
pub mod query;
pub mod storage;
pub mod transaction;

pub use control::{GrantBuilder, RevokeBuilder};
pub use definition::{
    AlterModelBuilder, DefineIndexBuilder, DefineModelBuilder, DropIndexBuilder, DropModelBuilder,
    ModelDefineExt,
};
pub use mutation::{
    InsertBuilder, InsertSelectBuilder, RemoveBuilder, UpdateBuilder, UpsertBuilder,
};
pub use query::GetBuilder;
pub use storage::{
    GetObjectBuilder, ListObjectsBuilder, MoveFileBuilder, PutObjectBuilder, ReadFileBuilder,
    WriteFileBuilder,
};
pub use transaction::TransactionBuilder;

use dol_model::Model;

// ---------------------------------------------------------------------------
// Model entry points — extension trait for builder access
// ---------------------------------------------------------------------------

/// Extension trait providing builder entry-point methods on [`Model`].
///
/// Import this trait to use `model.get()`, `model.insert()`, etc.
pub trait ModelBuilderExt {
    fn get(&self) -> GetBuilder<'_>;
    fn insert(&self) -> InsertBuilder<'_>;
    fn insert_select(&self) -> InsertSelectBuilder<'_>;
    fn update(&self) -> UpdateBuilder<'_>;
    fn remove(&self) -> RemoveBuilder<'_>;
    fn upsert(&self) -> UpsertBuilder<'_>;
    fn alter(&self) -> AlterModelBuilder<'_>;
    fn drop_model(&self) -> DropModelBuilder<'_>;
    fn create(&self) -> definition::CreateFromMeta<'_>;
}

impl ModelBuilderExt for Model {
    /// Start building a GET (SELECT) query.
    fn get(&self) -> GetBuilder<'_> {
        GetBuilder::new(self)
    }

    /// Start building an INSERT.
    fn insert(&self) -> InsertBuilder<'_> {
        InsertBuilder::new(self)
    }

    /// Start building an INSERT ... SELECT.
    fn insert_select(&self) -> InsertSelectBuilder<'_> {
        InsertSelectBuilder::new(self)
    }

    /// Start building an UPDATE.
    fn update(&self) -> UpdateBuilder<'_> {
        UpdateBuilder::new(self)
    }

    /// Start building a REMOVE (DELETE).
    fn remove(&self) -> RemoveBuilder<'_> {
        RemoveBuilder::new(self)
    }

    /// Start building an UPSERT (INSERT ... ON CONFLICT).
    fn upsert(&self) -> UpsertBuilder<'_> {
        UpsertBuilder::new(self)
    }

    /// Start building an ALTER MODEL (ALTER TABLE).
    fn alter(&self) -> AlterModelBuilder<'_> {
        AlterModelBuilder::new(self)
    }

    /// Start building a DROP MODEL (DROP TABLE).
    fn drop_model(&self) -> DropModelBuilder<'_> {
        DropModelBuilder::new(self)
    }

    /// Build a CREATE TABLE from this model's static metadata.
    fn create(&self) -> definition::CreateFromMeta<'_> {
        definition::CreateFromMeta::new(self)
    }
}

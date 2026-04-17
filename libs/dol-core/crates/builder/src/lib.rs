//! # dol-builder — DOL Builder API
//!
//! EntityBuilderExt trait and DDL/definition builders.
//! CRUD, control, storage, and transaction builders are re-exported from dol-query.

#![deny(unsafe_code)]

pub mod definition;

pub use definition::{
    AlterEntityBuilder, DefineEntityBuilder, DefineIndexBuilder, DefineTypeBuilder,
    DropEntityBuilder, DropIndexBuilder, DropTypeBuilder, EntityDefineExt,
};

// Re-export CRUD, control, storage, transaction builders from dol-query.
pub use dol_query::builder::control;
pub use dol_query::builder::mutation;
pub use dol_query::builder::query;
pub use dol_query::builder::storage;
pub use dol_query::builder::transaction;

pub use dol_query::builder::{
    InsertBuilder, InsertSelectBuilder, RemoveBuilder, UpdateBuilder, UpsertBuilder,
    GetBuilder, DefinePolicyBuilder, GrantBuilder, RevokeBuilder, Privilege,
    GetObjectBuilder, ListObjectsBuilder, MoveFileBuilder, PutObjectBuilder, ReadFileBuilder,
    WriteFileBuilder, TransactionBuilder,
};

use dol_entity::Entity;
use dol_query::builder::{GetBuilder as GB, InsertBuilder as IB, InsertSelectBuilder as ISB};
use dol_query::builder::{UpdateBuilder as UB, RemoveBuilder as RB, UpsertBuilder as UpB};

// ---------------------------------------------------------------------------
// Model entry points — extension trait for builder access
// ---------------------------------------------------------------------------

/// Extension trait providing builder entry-point methods on [`Entity`].
///
/// Import this trait to use `model.get()`, `model.insert()`, etc.
pub trait EntityBuilderExt {
    fn get(&self) -> GB<'_>;
    fn insert(&self) -> IB<'_>;
    fn insert_select(&self) -> ISB<'_>;
    fn update(&self) -> UB<'_>;
    fn remove(&self) -> RB<'_>;
    fn upsert(&self) -> UpB<'_>;
    fn alter(&self) -> AlterEntityBuilder<'_>;
    fn drop_entity(&self) -> DropEntityBuilder<'_>;
    fn create(&self) -> definition::CreateFromMeta<'_>;
}

impl EntityBuilderExt for Entity {
    fn get(&self) -> GB<'_> {
        GB::new(self)
    }

    fn insert(&self) -> IB<'_> {
        IB::new(self)
    }

    fn insert_select(&self) -> ISB<'_> {
        ISB::new(self)
    }

    fn update(&self) -> UB<'_> {
        UB::new(self)
    }

    fn remove(&self) -> RB<'_> {
        RB::new(self)
    }

    fn upsert(&self) -> UpB<'_> {
        UpB::new(self)
    }

    fn alter(&self) -> AlterEntityBuilder<'_> {
        AlterEntityBuilder::new(self)
    }

    fn drop_entity(&self) -> DropEntityBuilder<'_> {
        DropEntityBuilder::new(self)
    }

    fn create(&self) -> definition::CreateFromMeta<'_> {
        definition::CreateFromMeta::new(self)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests;

//! EntityBuilderExt — CRUD + DDL builder entry-points on Entity.
//!
//! This trait lives in the umbrella crate to avoid a circular dependency
//! between dol-entity and dol-query.

use dol_entity::{Entity, AlterEntityBuilder, CreateFromMeta, DropEntityBuilder};
use dol_query::builder::{GetBuilder, InsertBuilder, InsertSelectBuilder, UpdateBuilder, RemoveBuilder, UpsertBuilder};

/// Extension trait providing builder entry-point methods on [`Entity`].
///
/// Import this trait to use `entity.get()`, `entity.insert()`, etc.
pub trait EntityBuilderExt {
    fn get(&self) -> GetBuilder<'_>;
    fn insert(&self) -> InsertBuilder<'_>;
    fn insert_select(&self) -> InsertSelectBuilder<'_>;
    fn update(&self) -> UpdateBuilder<'_>;
    fn remove(&self) -> RemoveBuilder<'_>;
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

    fn remove(&self) -> RemoveBuilder<'_> {
        RemoveBuilder::new(self)
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

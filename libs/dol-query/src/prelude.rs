//! Curated re-exports — the stable, recommended surface for downstream users.
//!
//! ```ignore
//! use dol_query::prelude::*;
//! ```

pub use crate::ddl::{
    AlterEntityBuilder, CreateFromMeta, DefineEntityBuilder, DefineIndexBuilder, DefineTypeBuilder,
    DropEntityBuilder, DropIndexBuilder, DropTypeBuilder, EntityDefineExt,
};
pub use crate::{
    DeleteQuery, GetQuery, InsertQuery, JoinKind, LockMode, Query, UpdateQuery, UpsertQuery,
};

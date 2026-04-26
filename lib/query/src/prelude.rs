//! Curated re-exports — the stable, recommended surface for downstream users.
//!
//! ```ignore
//! use dol_query::prelude::*;
//! ```

pub use crate::ddl::{
    AlterEntityBuilder, CreateFromMeta, DefineEntityBuilder, DefineLookupBuilder,
    DefineTypeBuilder, DropEntityBuilder, DropLookupBuilder, DropTypeBuilder, EntityDefineExt,
};
pub use crate::{DeleteQuery, GetQuery, InsertQuery, JoinKind, Query, UpdateQuery, UpsertQuery};

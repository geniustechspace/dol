//! Curated re-exports for `dol-query`.
//!
//! This prelude includes the IR types accepted by `dol-query` public
//! functions (`Privilege`, `IsolationLevel`, `PolicyScope`, `SchemaBinding`),
//! so `use dol_query::prelude::*;` is sufficient for the documented examples
//! without having to also import from `dol_ir`.

pub use crate::{
    DeleteQuery, GetQuery, InsertQuery, JoinKind, Query, UpdateQuery, UpsertQuery,
};
pub use crate::ddl::{
    define_entity, define_entity_inferred, define_from_entity, define_lookup, drop_entity,
    drop_lookup, drop_field, rename_field,
};
pub use crate::storage::{
    get_blob, list_blobs, move_file, put_blob, put_blob_from_path, read_file, write_file,
    write_file_from_path,
};
pub use crate::control::{define_policy, grant, revoke, tx_atomic, tx_begin, tx_commit, tx_rollback};

// IR types referenced by the public `dol-query` surface.
pub use dol_ir::{
    operation::{IsolationLevel, PolicyScope},
    privilege::Privilege,
    target::SchemaBinding,
};

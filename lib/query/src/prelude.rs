//! Curated re-exports for `dol-query`.
//!
//! This prelude includes the IR types accepted by `dol-query` public
//! functions (`Privilege`, `IsolationLevel`, `PolicyScope`, `SchemaBinding`),
//! so `use dol_query::prelude::*;` is sufficient for the documented examples
//! without having to also import from `dol_command`.

pub use crate::{DeleteQuery, GetQuery, InsertQuery, JoinKind, Query, UpdateQuery, UpsertQuery};

// IR types and program-emitting builders referenced by the public
// `dol-query` surface — DDL (`define_entity`, …), ACL / Tx
// (`grant`, `tx_begin`, …), and storage (`get_blob`, `read_file`, …)
// helpers all live in `dol_command::builders` since PR 5 of the v2 layout
// refactor; re-export them here so callers keep a single import site.
pub use dol_command::builders::{
    define_entity, define_entity_inferred, define_from_entity, define_lookup, define_policy,
    drop_entity, drop_field, drop_lookup, get_blob, grant, list_blobs, move_file, put_blob,
    put_blob_from_path, read_file, rename_field, revoke, tx_atomic, tx_begin, tx_commit,
    tx_rollback, write_file, write_file_from_path,
};
pub use dol_command::{
    operation::{IsolationLevel, PolicyScope},
    privilege::Privilege,
    target::SchemaBinding,
};

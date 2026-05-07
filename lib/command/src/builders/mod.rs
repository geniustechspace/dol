//! Top-down builders that emit ready-made [`Program`](crate::program::Program)
//! values for the verbs that don't need the fluent query DSL: DDL
//! (`define_entity`, `drop_entity`, `define_index`, …), ACL / control
//! (`grant`, `revoke`, `define_policy`, `tx_begin`, `tx_commit`,
//! `tx_atomic`, …), and storage helpers (`get_blob`, `put_blob`,
//! `read_file`, `write_file`, …).
//!
//! These helpers were previously hosted in `dol-query`; they were moved
//! down into `dol-command` because they construct IR directly without using
//! the fluent query DSL. `dol-query` keeps only the actual query
//! builders (`GetQuery`, `InsertQuery`, `UpdateQuery`, `DeleteQuery`,
//! `UpsertQuery`) plus the streaming / pipeline submodules.

extern crate alloc;

pub mod control;
pub mod ddl;
pub mod storage;
pub mod target;

pub use control::{define_policy, grant, revoke, tx_atomic, tx_begin, tx_commit, tx_rollback};
pub use ddl::{
    define_entity, define_entity_inferred, define_index, define_lookup, drop_entity, drop_field,
    drop_lookup, rename_field,
};
#[cfg(feature = "schema")]
pub use ddl::define_from_entity;
pub use storage::{
    get_blob, list_blobs, move_file, put_blob, put_blob_from_path, read_file, write_file,
    write_file_from_path,
};
pub use target::{intern_symbol, locator_from_parts, target_from_parts};

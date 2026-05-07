//! Curated re-exports for `dol-query`.
//!
//! `dol-query` is now a pure data crate and no longer re-exports from
//! `dol-command`. The previously-available IR helpers (`define_entity`,
//! `grant`, `tx_begin`, `read_file`, …) and IR types (`IsolationLevel`,
//! `PolicyScope`, `Privilege`, `SchemaBinding`) live in
//! `dol_command::builders` and `dol_command::prelude` respectively;
//! import them directly from there. The lowering trait
//! [`BuildProgram`](dol_command::lower_query::BuildProgram) — which
//! re-creates the historical chained `builder.try_build()` ergonomics —
//! also lives in `dol-command`.

pub use crate::{
    DeleteQuery, GetQuery, InsertQuery, JoinClause, JoinKind, Query, UpdateQuery, UpsertQuery,
};

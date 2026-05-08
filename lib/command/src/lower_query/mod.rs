//! Lower the fluent [`dol_query`] DSL builders into [`crate::program::Program`]s.
//!
//! Every `try_build`-style call site previously hosted on the
//! `dol-query` builders themselves now lives here. This inverts the
//! historical `dol-query → dol-command` edge: the DSL crate owns the
//! data shape (`GetQuery`, `InsertQuery`, …), and the IR crate owns the
//! lowering that turns those data shapes into `Operation`s and
//! `Program`s.
//!
//! # Why this lives in `dol-command`
//!
//! The lowering produces `dol_command::operation::Operation` and
//! `dol_command::program::Program` values. Hosting it from the DSL crate
//! used to require `dol-query` to depend on `dol-command`, which violated
//! the workspace DAG (`query` was supposed to sit *below* `command`, not
//! above it). The dependency is now `command → query` (gated by the
//! `query` feature, default-on), which matches the documented graph.
//!
//! # API shape
//!
//! Two equivalent forms are exposed:
//!
//! - **Free functions** [`lower_get`], [`lower_insert`], [`lower_update`],
//!   [`lower_delete`], [`lower_upsert`] — explicit, no trait import needed.
//! - **The [`BuildProgram`] trait** — with a single `try_build` method,
//!   re-creating the chained builder ergonomics of the old API. Callers
//!   that want `query.get().filter(...).try_build()` only need
//!   `use dol_command::lower_query::BuildProgram;` in scope.
//!
//! Both forms share the same underlying lowering and threading of
//! [`Budget`](dol_core::policy::Budget) through `dol-expr`.

extern crate alloc;

mod delete;
mod error;
mod get;
mod insert;
mod update;
mod upsert;

pub use delete::lower_delete;
pub use error::BuildError;
pub use get::lower_get;
pub use insert::lower_insert;
pub use update::lower_update;
pub use upsert::lower_upsert;

use crate::program::Program;

/// Convenience trait re-creating the chained `builder.try_build()`
/// ergonomics from the old `dol-query` API.
///
/// `dol-query` used to host `try_build` methods directly on each builder;
/// hoisting them out lets the DSL crate stay free of `dol-command`. With
/// this trait imported, callers regain the original chain:
///
/// ```ignore
/// use dol_command::lower_query::BuildProgram;
/// use dol_query::Query;
///
/// let prog = Query::from("users").get().try_build()?;
/// ```
///
/// Equivalent to calling [`lower_get`] (or the matching `lower_*` for the
/// builder's variant) directly.
pub trait BuildProgram {
    /// Consume the builder and return a [`Program`] containing one
    /// [`crate::operation::Operation`].
    fn try_build(self) -> Result<Program, BuildError>;
}

impl BuildProgram for dol_query::GetQuery {
    #[inline]
    fn try_build(self) -> Result<Program, BuildError> {
        lower_get(self)
    }
}

impl BuildProgram for dol_query::InsertQuery {
    #[inline]
    fn try_build(self) -> Result<Program, BuildError> {
        lower_insert(self)
    }
}

impl BuildProgram for dol_query::UpdateQuery {
    #[inline]
    fn try_build(self) -> Result<Program, BuildError> {
        lower_update(self)
    }
}

impl BuildProgram for dol_query::DeleteQuery {
    #[inline]
    fn try_build(self) -> Result<Program, BuildError> {
        lower_delete(self)
    }
}

impl BuildProgram for dol_query::UpsertQuery {
    #[inline]
    fn try_build(self) -> Result<Program, BuildError> {
        lower_upsert(self)
    }
}

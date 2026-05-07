//! Errors returned by the fallible `try_build()` family on every query builder.
//!
//! v2 invariant: production code may not panic. The five query builders
//! ([`crate::DeleteQuery`], [`crate::UpdateQuery`], [`crate::InsertQuery`],
//! [`crate::UpsertQuery`], [`crate::GetQuery`]) used to call
//! `dol_expr::lower::lower_*` and unwrap the result, which meant a deep
//! filter expression or a runaway projection could panic the caller. Each
//! builder now exposes `try_build(self) -> Result<Program, BuildError>` and
//! threads a [`Budget`](dol_core::policy::Budget) through the lowering so
//! adversarial input is rejected with a clean error.

extern crate alloc;

use alloc::string::String;
use core::fmt;

use dol_expr::lower::LowerError;

/// Reasons a query builder may refuse to produce a [`Program`](dol_ir::program::Program).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// Lowering an expression in the WHERE clause failed.
    Filter(LowerError),
    /// Lowering an expression in the HAVING clause failed.
    Having(LowerError),
    /// Lowering a projection expression failed.
    Projection(LowerError),
    /// Lowering a `GROUP BY` expression failed.
    GroupBy(LowerError),
    /// Lowering an `ORDER BY` expression failed.
    OrderBy(LowerError),
    /// Lowering an `UPDATE … SET <col> = <expr>` right-hand-side failed.
    SetValue {
        /// Name of the column whose assignment failed.
        column: String,
        /// Underlying lowering error.
        cause: LowerError,
    },
    /// Lowering a JOIN ON condition failed.
    JoinOn(LowerError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filter(e) => write!(f, "dol-query: WHERE clause lowering failed: {e}"),
            Self::Having(e) => write!(f, "dol-query: HAVING clause lowering failed: {e}"),
            Self::Projection(e) => write!(f, "dol-query: projection lowering failed: {e}"),
            Self::GroupBy(e) => write!(f, "dol-query: GROUP BY lowering failed: {e}"),
            Self::OrderBy(e) => write!(f, "dol-query: ORDER BY lowering failed: {e}"),
            Self::SetValue { column, cause } => write!(
                f,
                "dol-query: SET {column} = <expr> lowering failed: {cause}"
            ),
            Self::JoinOn(e) => write!(f, "dol-query: JOIN ON lowering failed: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BuildError {}

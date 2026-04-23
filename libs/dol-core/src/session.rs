//! [`BuildSession`] — validation configuration wired into the render path.
//!
//! A `BuildSession` bundles the pass-chain configuration ([`PassCaps`],
//! optional [`Schema`], optional [`AllowList`]) and exposes a
//! [`validate_expr`] convenience method that lowers expressions into a
//! reusable scratch arena and runs the five validation passes.
//!
//! ## Scratch-space reuse
//!
//! The session holds a `RefCell<Scratch>` that contains a pre-allocated
//! [`ExprArena`] and [`Interner`].  Each validation call `reset()`s the
//! scratch rather than allocating fresh storage, so:
//!
//! - A [`render_with`] call that validates N clauses performs **one** heap
//!   allocation (at session construction) rather than 2·N.
//! - [`validate_exprs`] processing M expressions in the same context resets
//!   and reuses the same backing storage M times.
//!
//! [`render_with`]: crate::session::BuildSession::check_exprs
//! [`validate_exprs`]: crate::session::BuildSession::validate_exprs

use std::cell::RefCell;

use crate::expr::Expr;
use crate::expr::arena::ExprArena;
use crate::expr::interner::Interner;
use crate::expr::pass::{AllowList, PassCaps, PassChain, PassError, Schema, run_passes};
use crate::expr::pass::semantic::ExprContext;
use crate::op::BackendError;

// ─── BuildError ──────────────────────────────────────────────────────────────

/// Error returned by [`render_with`](crate::session::BuildSession::check_exprs).
///
/// Either the validation passes rejected the expression tree, or the backend
/// renderer itself produced an error.
#[derive(Debug)]
pub enum BuildError {
    /// One or more validation passes failed.
    Validation(Vec<PassError>),
    /// The backend renderer returned an error.
    Render(BackendError),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(errs) => {
                write!(f, "validation failed:")?;
                for e in errs {
                    write!(f, "\n  - {e}")?;
                }
                Ok(())
            }
            Self::Render(e) => write!(f, "render error: {e}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Render(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BackendError> for BuildError {
    fn from(e: BackendError) -> Self {
        Self::Render(e)
    }
}

// ─── Scratch ─────────────────────────────────────────────────────────────────

/// Reusable scratch space for expression lowering.
///
/// Holds a pre-allocated arena and interner that are `reset()` before each
/// lowering pass rather than re-allocated from scratch.
#[derive(Debug, Default)]
struct Scratch {
    arena:    ExprArena,
    interner: Interner,
}

// ─── BuildSession ─────────────────────────────────────────────────────────────

/// Validation configuration used by [`render_with`] on query/mutation builders.
///
/// Holds resource caps, an optional field schema for scope checking, and an
/// optional function allowlist for security checking.
///
/// The session pre-allocates reusable scratch storage at construction time.
/// Clone produces a fresh session with an empty scratch — existing validated
/// state from the original is not inherited.
#[derive(Debug)]
pub struct BuildSession {
    pub caps:      PassCaps,
    pub schema:    Option<Schema>,
    pub allowlist: Option<AllowList>,
    /// Reusable arena + interner.  `RefCell` lets `&self` validation methods
    /// mutate the scratch without requiring `&mut self`.
    scratch: RefCell<Scratch>,
}

impl BuildSession {
    /// Create a session with default caps and no schema/allowlist.
    pub fn new() -> Self {
        Self {
            caps:      PassCaps::default(),
            schema:    None,
            allowlist: None,
            scratch:   RefCell::new(Scratch::default()),
        }
    }

    /// Override the resource/security caps.
    pub fn with_caps(mut self, caps: PassCaps) -> Self {
        self.caps = caps;
        self
    }

    /// Attach a field schema for scope checking.
    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Attach a function allowlist for security checking.
    pub fn with_allowlist(mut self, allowlist: AllowList) -> Self {
        self.allowlist = Some(allowlist);
        self
    }

    /// Validate a single expression in `context`.
    ///
    /// Resets the session's scratch arena and interner, lowers `expr` into
    /// them, and runs all five passes.  Returns the first non-empty error
    /// list, or `vec![]` if the expression is valid.
    ///
    /// For validating multiple expressions in the same context, prefer
    /// [`validate_exprs`](Self::validate_exprs) to avoid redundant scratch
    /// setup.
    pub fn validate_expr(&self, expr: &Expr<'_>, context: ExprContext) -> Vec<PassError> {
        let chain = self.make_chain(context);
        let mut borrow = self.scratch.borrow_mut();
        // Reborrow as plain `&mut Scratch` so Rust's field-split rule applies.
        let s: &mut Scratch = &mut *borrow;
        s.arena.reset();
        s.interner.reset();
        s.arena.lower(expr, &mut s.interner);
        run_passes(&s.arena, &s.interner, &chain)
    }

    /// Validate a slice of expressions, all in the same `context`.
    ///
    /// Reuses the session's scratch arena and interner across all expressions:
    /// the backing memory is allocated once and `reset()` between each
    /// expression rather than reallocated.  Returns the first non-empty error
    /// list, or `vec![]` if all pass.
    pub fn validate_exprs(&self, exprs: &[Expr<'_>], context: ExprContext) -> Vec<PassError> {
        if exprs.is_empty() {
            return vec![];
        }
        let chain = self.make_chain(context);
        let mut borrow = self.scratch.borrow_mut();
        let s: &mut Scratch = &mut *borrow;
        for expr in exprs {
            s.arena.reset();
            s.interner.reset();
            s.arena.lower(expr, &mut s.interner);
            let errors = run_passes(&s.arena, &s.interner, &chain);
            if !errors.is_empty() {
                return errors;
            }
        }
        vec![]
    }

    /// Validate `exprs` and return `Err(BuildError::Validation(_))` on failure.
    ///
    /// Convenience wrapper called by builder `render_with` impls.
    pub fn check_exprs(
        &self,
        exprs: &[Expr<'_>],
        context: ExprContext,
    ) -> Result<(), BuildError> {
        let errs = self.validate_exprs(exprs, context);
        if errs.is_empty() { Ok(()) } else { Err(BuildError::Validation(errs)) }
    }

    // ── Private ──────────────────────────────────────────────────────────────

    fn make_chain(&self, context: ExprContext) -> PassChain<'_> {
        PassChain {
            caps:    self.caps.clone(),
            schema:  self.schema.as_ref(),
            context,
            allow:   self.allowlist.as_ref(),
        }
    }
}

impl Default for BuildSession {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for BuildSession {
    /// Clones the session configuration (caps, schema, allowlist) but starts
    /// with a fresh empty scratch rather than copying the original's transient
    /// arena state.
    fn clone(&self) -> Self {
        Self {
            caps:      self.caps.clone(),
            schema:    self.schema.clone(),
            allowlist: self.allowlist.clone(),
            scratch:   RefCell::new(Scratch::default()),
        }
    }
}

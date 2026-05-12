//! Evaluation scope — `Context<'a>`.
//!
//! Per `dol-rewrite-plan-v2.md` §8.3. A [`Context`] describes the
//! dynamic scope inside which an expression is evaluated when wrapped
//! in [`Expr::Scoped`]:
//!
//! - **`partition_by`** — keys that partition the input into independent
//!   groups (SQL `PARTITION BY`, dataframe `groupby`, IoT stream key,
//!   pipeline shard).
//! - **`order_by`** — ordering applied within each partition before the
//!   frame and the scoped expression are evaluated.
//! - **`frame`** — optional bounded window over the ordered partition
//!   (rows / range / groups, with [`Boundary`] ends). When `None`,
//!   the scope is the entire partition.
//!
//! `Context` is the universal "evaluate inside a scope" descriptor —
//! the same shape covers SQL `OVER (…)`, dataframe groupby+sort, IoT
//! stream slices, time-series segments, and pipeline windows. There is
//! no separate `WindowNode` side pool.
//!
//! ## Status
//!
//! M3c-γ shipped the data shapes. M3c-δ adds the fluent
//! [`ContextBuilder`] (builds an [`Expr::Scoped`]) and
//! [`ConditionalBuilder`] (builds an [`Expr::Match`]). Lowering
//! `Expr<'a>` → `ExprArena` arrives in the next slice.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::expr::frame::{Boundary, Frame, FrameUnit};
use crate::expr::order::OrderByExpr;
use crate::expr::tree::Expr;

/// The dynamic scope wrapping a [`Scoped`](crate::expr::tree::Expr::Scoped)
/// expression.
///
/// All three fields default to "absent" (empty / `None`). An empty
/// `Context` is valid and means "evaluate against the whole input,
/// in input order, with no windowing" — most backends will treat this
/// as a degenerate scope and elide it during emission.
///
/// ## Validity
///
/// `Context` does not enforce relationships among its fields at
/// construction time:
///
/// - A non-`None` `frame` without an `order_by` is permitted but its
///   meaning is backend-defined; lowering may reject or normalise it.
/// - Empty `partition_by` means "single partition over all input".
///
/// All such checks are the lowering layer's responsibility, where
/// they can produce a proper diagnostic with span context.
#[derive(Debug, Clone, PartialEq)]
pub struct Context<'a> {
    /// Partitioning keys. Empty means a single partition over the
    /// whole input.
    pub partition_by: Vec<Expr<'a>>,
    /// Ordering applied within each partition. Empty means "input
    /// order" (backend-defined for unordered inputs).
    pub order_by: Vec<OrderByExpr<'a>>,
    /// Optional bounded frame within the ordered partition. `None`
    /// means the scope is the entire partition.
    pub frame: Option<Frame>,
}

impl<'a> Context<'a> {
    /// An empty scope: no partitioning, no ordering, no frame.
    ///
    /// Equivalent to evaluating the wrapped expression against the
    /// whole input. Backends typically elide this during emission.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: None,
        }
    }

    /// Returns `true` iff the scope carries no partitioning, no
    /// ordering, and no frame.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.partition_by.is_empty() && self.order_by.is_empty() && self.frame.is_none()
    }
}

impl Default for Context<'_> {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

// ─── ContextBuilder ──────────────────────────────────────────────────

/// Fluent builder for an [`Expr::Scoped`] expression.
///
/// Per `dol-rewrite-plan-v2.md` §8.3. A [`ContextBuilder`] bundles the
/// expression to be evaluated together with the [`Context`] that
/// describes its dynamic scope. The builder's `build()` returns
/// `Expr::Scoped { expr, context }` ready for lowering.
///
/// All setter methods are idempotent — calling `partitioning` /
/// `ordering` / `between_positional` twice replaces (not appends) the
/// previous value. This matches typical fluent-builder ergonomics
/// where the configuration is described once.
///
/// # Example
///
/// ```
/// use dol_ir::expr::context::ContextBuilder;
/// use dol_ir::expr::frame::Boundary;
/// use dol_ir::expr::order::OrderByExpr;
/// use dol_ir::expr::tree::Expr;
/// use dol_core::path::Path;
///
/// // COUNT(*) OVER (PARTITION BY user
/// //                ORDER BY ts
/// //                ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)
/// let scoped = ContextBuilder::new(Expr::CountAll)
///     .partitioning([Expr::Ref(Path::new("user"))])
///     .ordering([OrderByExpr::new(Expr::Ref(Path::new("ts")))])
///     .between_positional(Boundary::unbounded_preceding(), Boundary::Current)
///     .build();
///
/// assert!(matches!(scoped, Expr::Scoped { .. }));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ContextBuilder<'a> {
    expr: Expr<'a>,
    partition_by: Vec<Expr<'a>>,
    order_by: Vec<OrderByExpr<'a>>,
    frame: Option<Frame>,
}

impl<'a> ContextBuilder<'a> {
    /// Start a new scoped-expression builder around `expr`.
    ///
    /// `expr` is the expression evaluated within the scope. With no
    /// further setter calls, `build()` returns `Expr::Scoped { expr,
    /// context: Context::empty() }`.
    #[inline]
    #[must_use]
    pub fn new(expr: Expr<'a>) -> Self {
        Self {
            expr,
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: None,
        }
    }

    /// Set (replace) the partitioning keys.
    #[inline]
    #[must_use]
    pub fn partitioning(mut self, keys: impl IntoIterator<Item = Expr<'a>>) -> Self {
        self.partition_by = keys.into_iter().collect();
        self
    }

    /// Set (replace) the ordering list.
    #[inline]
    #[must_use]
    pub fn ordering(mut self, exprs: impl IntoIterator<Item = OrderByExpr<'a>>) -> Self {
        self.order_by = exprs.into_iter().collect();
        self
    }

    /// Set (replace) a positional row-based frame: `ROWS BETWEEN start
    /// AND end`.
    ///
    /// Defaults the frame unit to [`FrameUnit::Rows`] — the only unit
    /// the spec exposes through this builder. For `RANGE` or `GROUPS`
    /// frames, set [`Context::frame`] directly with [`Frame::range`]
    /// or [`Frame::groups`].
    #[inline]
    #[must_use]
    pub fn between_positional(mut self, start: Boundary, end: Boundary) -> Self {
        self.frame = Some(Frame {
            unit: FrameUnit::Rows,
            start,
            end,
        });
        self
    }

    /// Consume the builder and return [`Expr::Scoped`].
    #[inline]
    #[must_use]
    pub fn build(self) -> Expr<'a> {
        Expr::Scoped {
            expr: Box::new(self.expr),
            context: Context {
                partition_by: self.partition_by,
                order_by: self.order_by,
                frame: self.frame,
            },
        }
    }
}

// ─── ConditionalBuilder ──────────────────────────────────────────────

/// Fluent builder for an [`Expr::Match`] expression.
///
/// Per `dol-rewrite-plan-v2.md` §8.3. Arms are evaluated in insertion
/// order; the first matching arm wins. The optional `fallback`
/// supplies the result when no arm matches; with `fallback: None` the
/// backend substitutes its native null value.
///
/// # Example
///
/// ```
/// use dol_ir::expr::context::ConditionalBuilder;
/// use dol_ir::expr::tree::Expr;
/// use dol_core::literal::Literal;
///
/// // CASE WHEN true THEN 1 ELSE 0 END
/// let m = ConditionalBuilder::new()
///     .when(Expr::Lit(Literal::Bool(true)), Expr::Lit(Literal::Int64(1)))
///     .fallback(Expr::Lit(Literal::Int64(0)))
///     .build();
///
/// assert!(matches!(m, Expr::Match { .. }));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBuilder<'a> {
    arms: Vec<(Expr<'a>, Expr<'a>)>,
    fallback: Option<Expr<'a>>,
}

impl<'a> ConditionalBuilder<'a> {
    /// Start a new empty multi-arm conditional builder.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            arms: Vec::new(),
            fallback: None,
        }
    }

    /// Append a `WHEN cond THEN result` arm.
    ///
    /// Arms are evaluated in insertion order; the first matching arm
    /// wins.
    #[inline]
    #[must_use]
    pub fn when(mut self, cond: Expr<'a>, result: Expr<'a>) -> Self {
        self.arms.push((cond, result));
        self
    }

    /// Set (replace) the `ELSE` fallback.
    ///
    /// With no fallback the backend substitutes its native null value
    /// when no arm matches.
    #[inline]
    #[must_use]
    pub fn fallback(mut self, expr: Expr<'a>) -> Self {
        self.fallback = Some(expr);
        self
    }

    /// Consume the builder and return [`Expr::Match`].
    #[inline]
    #[must_use]
    pub fn build(self) -> Expr<'a> {
        Expr::Match {
            arms: self.arms,
            fallback: self.fallback.map(Box::new),
        }
    }
}

impl Default for ConditionalBuilder<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::frame::{Boundary, Frame};
    use crate::expr::order::{NullsOrder, SortDirection};
    use dol_core::path::Path;

    #[test]
    fn empty_context_is_empty() {
        let c = Context::empty();
        assert!(c.is_empty());
        assert!(c.partition_by.is_empty());
        assert!(c.order_by.is_empty());
        assert!(c.frame.is_none());
    }

    #[test]
    fn default_matches_empty() {
        assert_eq!(Context::default(), Context::empty());
    }

    #[test]
    fn populated_context_is_not_empty() {
        let c = Context {
            partition_by: alloc::vec![Expr::Ref(Path::new("user"))],
            order_by: alloc::vec![],
            frame: None,
        };
        assert!(!c.is_empty());
    }

    #[test]
    fn context_round_trips_through_clone_and_eq() {
        let c = Context {
            partition_by: alloc::vec![Expr::Ref(Path::new("user"))],
            order_by: alloc::vec![OrderByExpr {
                expr: Expr::Ref(Path::new("ts")),
                dir: SortDirection::Desc,
                nulls: NullsOrder::First,
            }],
            frame: Some(Frame::rows(
                Boundary::unbounded_preceding(),
                Boundary::Current,
            )),
        };
        let c2 = c.clone();
        assert_eq!(c, c2);
        assert!(!c.is_empty());
    }

    // ── ContextBuilder ────────────────────────────────────────────

    #[test]
    fn context_builder_minimal_yields_scoped_with_empty_context() {
        let scoped = ContextBuilder::new(Expr::CountAll).build();
        let Expr::Scoped { expr, context } = scoped else {
            panic!("not Scoped")
        };
        assert_eq!(*expr, Expr::CountAll);
        assert!(context.is_empty());
    }

    #[test]
    fn context_builder_full_chain_populates_all_fields() {
        let scoped = ContextBuilder::new(Expr::CountAll)
            .partitioning([Expr::Ref(Path::new("user"))])
            .ordering([OrderByExpr::new(Expr::Ref(Path::new("ts")))])
            .between_positional(Boundary::unbounded_preceding(), Boundary::Current)
            .build();
        let Expr::Scoped { expr, context } = scoped else {
            panic!("not Scoped")
        };
        assert_eq!(*expr, Expr::CountAll);
        assert_eq!(context.partition_by.len(), 1);
        assert_eq!(context.order_by.len(), 1);
        assert_eq!(
            context.frame,
            Some(Frame {
                unit: FrameUnit::Rows,
                start: Boundary::unbounded_preceding(),
                end: Boundary::Current,
            })
        );
    }

    #[test]
    fn context_builder_setters_replace_not_append() {
        let scoped = ContextBuilder::new(Expr::CountAll)
            .partitioning([Expr::Ref(Path::new("a"))])
            .partitioning([Expr::Ref(Path::new("b")), Expr::Ref(Path::new("c"))])
            .ordering([OrderByExpr::new(Expr::Ref(Path::new("x")))])
            .ordering([OrderByExpr::new(Expr::Ref(Path::new("y")))])
            .between_positional(Boundary::Current, Boundary::Current)
            .between_positional(
                Boundary::unbounded_preceding(),
                Boundary::unbounded_following(),
            )
            .build();
        let Expr::Scoped { context, .. } = scoped else {
            panic!()
        };
        // Replaced, not appended.
        assert_eq!(context.partition_by.len(), 2);
        assert_eq!(context.order_by.len(), 1);
        assert_eq!(
            context.frame.unwrap().start,
            Boundary::unbounded_preceding()
        );
    }

    // ── ConditionalBuilder ────────────────────────────────────────

    #[test]
    fn conditional_builder_default_matches_new() {
        assert_eq!(ConditionalBuilder::default(), ConditionalBuilder::new());
    }

    #[test]
    fn conditional_builder_with_no_arms_builds_empty_match() {
        let m = ConditionalBuilder::new().build();
        let Expr::Match { arms, fallback } = m else {
            panic!("not Match")
        };
        assert!(arms.is_empty());
        assert!(fallback.is_none());
    }

    #[test]
    fn conditional_builder_appends_arms_in_order_and_carries_fallback() {
        use dol_core::literal::Literal;
        let m = ConditionalBuilder::new()
            .when(Expr::Lit(Literal::Bool(true)), Expr::Lit(Literal::Int64(1)))
            .when(
                Expr::Lit(Literal::Bool(false)),
                Expr::Lit(Literal::Int64(2)),
            )
            .fallback(Expr::Lit(Literal::Int64(3)))
            .build();
        let Expr::Match { arms, fallback } = m else {
            panic!()
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].1, Expr::Lit(Literal::Int64(1)));
        assert_eq!(arms[1].1, Expr::Lit(Literal::Int64(2)));
        assert_eq!(*fallback.unwrap(), Expr::Lit(Literal::Int64(3)));
    }

    #[test]
    fn conditional_builder_fallback_replaces() {
        use dol_core::literal::Literal;
        let m = ConditionalBuilder::new()
            .fallback(Expr::Lit(Literal::Int64(1)))
            .fallback(Expr::Lit(Literal::Int64(2)))
            .build();
        let Expr::Match { fallback, .. } = m else {
            panic!()
        };
        assert_eq!(*fallback.unwrap(), Expr::Lit(Literal::Int64(2)));
    }
}

//! Evaluation scope and conditional branching for DOL expressions.
//!
//! This module provides two categories of types:
//!
//! **Data types** — plain structs that are part of the [`Expr`] IR and the
//! `serde` surface:
//!
//! - [`Context`] — the resolved evaluation scope stored inside
//!   [`Expr::Scoped`]. Holds partitioning keys, ordering expressions, and an
//!   optional [`ContextFrame`].
//! - [`ContextFrame`] — describes *how far* a bounded context reaches, in
//!   terms of [`Boundary`] values and a [`ContextMode`].
//! - [`Boundary`], [`Extent`], [`ContextMode`] — the primitive descriptors
//!   for context boundaries.
//!
//! **Builder types** — transient construction state that produces [`Expr`]
//! nodes. Builders intentionally **do not** implement [`serde::Serialize`] or
//! [`serde::Deserialize`]; serialise the produced [`Expr`] instead.
//!
//! - [`ContextBuilder`] — fluent builder that produces [`Expr::Scoped`].
//! - [`ConditionalBuilder`] — fluent builder that produces [`Expr::Match`].
//!
//! # Store-neutral vocabulary
//!
//! All names are store-neutral. The table below shows how each DOL concept
//! maps to backend-specific terminology:
//!
//! | DOL                | SQL                         | Other backends               |
//! |--------------------|-----------------------------|------------------------------|
//! | `Context`          | `OVER (…)`                  | groupby+sort, stream window  |
//! | `partitioning`     | `PARTITION BY`              | groupby key, shard key       |
//! | `ordering`         | `ORDER BY`                  | sort key, event order        |
//! | `ContextFrame`     | `ROWS … BETWEEN`            | sliding window, time slice   |
//! | `Boundary::Before` | `n PRECEDING / UNBOUNDED`   | start offset                 |
//! | `Boundary::After`  | `n FOLLOWING / UNBOUNDED`   | end offset                   |
//! | `ContextMode`      | `ROWS` vs `RANGE`           | positional vs value-based    |

use alloc::{boxed::Box, vec::Vec};

use super::Expr;
use super::order::OrderByExpr;

// ── Frame primitives ─────────────────────────────────────────────────────────

/// A finite or unbounded reach away from the current evaluation point.
///
/// `Extent` is the payload of [`Boundary::Before`] and [`Boundary::After`].
/// It describes *how many units* the boundary extends relative to the current
/// point, where "units" are defined by the active [`ContextMode`].
///
/// # Examples
///
/// ```rust
/// use dol_expr::tree::context::Extent;
///
/// let three_units    = Extent::Finite(3);
/// let all_available  = Extent::Unbounded;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Extent {
    /// A bounded reach measured in the units of the active [`ContextMode`].
    ///
    /// For [`ContextMode::Positional`] this is item offsets; for
    /// [`ContextMode::ValueDistance`] this is a comparable value distance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::Extent;
    ///
    /// let five = Extent::Finite(5);
    /// ```
    Finite(u32),

    /// A reach that extends as far as the input context allows.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::Extent;
    ///
    /// let everything = Extent::Unbounded;
    /// ```
    Unbounded,
}

/// A boundary relative to the current evaluation point.
///
/// `Boundary` intentionally avoids row-oriented wording. A boundary may
/// refer to positions, values, timestamps, event offsets, or any other
/// ordered domain supported by a backend through [`ContextMode`].
///
/// # Examples
///
/// ```rust
/// use dol_expr::tree::context::{Boundary, Extent};
///
/// let from_beginning = Boundary::Before(Extent::Unbounded);
/// let five_before    = Boundary::Before(Extent::Finite(5));
/// let here           = Boundary::Current;
/// let two_after      = Boundary::After(Extent::Finite(2));
/// let through_end    = Boundary::After(Extent::Unbounded);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Boundary {
    /// A boundary located `extent` units *before* the current point.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::{Boundary, Extent};
    ///
    /// let prev_three     = Boundary::Before(Extent::Finite(3));
    /// let from_beginning = Boundary::Before(Extent::Unbounded);
    /// ```
    Before(Extent),

    /// The current evaluation point itself.
    ///
    /// Backend equivalent: `CURRENT ROW`, the active cursor, or the focused
    /// item in an ordered context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::Boundary;
    ///
    /// let here = Boundary::Current;
    /// ```
    Current,

    /// A boundary located `extent` units *after* the current point.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::{Boundary, Extent};
    ///
    /// let next_two   = Boundary::After(Extent::Finite(2));
    /// let through_end = Boundary::After(Extent::Unbounded);
    /// ```
    After(Extent),
}

/// How context [`Boundary`] values are interpreted.
///
/// A backend maps each mode to its own execution model. SQL backends typically
/// lower [`Positional`](ContextMode::Positional) to `ROWS` and
/// [`ValueDistance`](ContextMode::ValueDistance) to `RANGE`; non-SQL backends
/// may map them to offsets, sequence windows, or event-distance windows.
///
/// # Example
///
/// ```rust
/// use dol_expr::tree::context::ContextMode;
///
/// let by_position = ContextMode::Positional;
/// let by_value    = ContextMode::ValueDistance;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ContextMode {
    /// Boundaries are interpreted as item offsets in the ordered input.
    Positional,

    /// Boundaries are interpreted as a value distance in the ordering domain.
    ///
    /// The distance may be numeric, temporal, lexical, or any other
    /// comparable measure supported by the backend.
    ValueDistance,
}

/// A bounded evaluation frame for a [`Context`].
///
/// `ContextFrame` describes which neighbouring input items (or value ranges)
/// participate in a computation relative to the current evaluation point. It is
/// always paired with a [`ContextMode`] that determines how the boundaries are
/// measured.
///
/// # Examples
///
/// ```rust
/// use dol_expr::tree::context::{Boundary, ContextFrame, ContextMode, Extent};
///
/// // Trailing window: three items back through the current point (positional)
/// let trailing = ContextFrame {
///     mode:  ContextMode::Positional,
///     start: Boundary::Before(Extent::Finite(3)),
///     end:   Some(Boundary::Current),
/// };
///
/// // Full context: everything before and after (positional)
/// let full = ContextFrame {
///     mode:  ContextMode::Positional,
///     start: Boundary::Before(Extent::Unbounded),
///     end:   Some(Boundary::After(Extent::Unbounded)),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ContextFrame {
    /// How [`start`](Self::start) and [`end`](Self::end) are measured.
    pub mode: ContextMode,

    /// The starting boundary of the evaluation frame.
    pub start: Boundary,

    /// The optional ending boundary of the evaluation frame.
    ///
    /// `None` represents a one-sided frame open at the end, if the
    /// surrounding expression semantics permit it.
    pub end: Option<Boundary>,
}

// ── Context — the resolved IR data type ──────────────────────────────────────

/// The resolved evaluation scope stored in [`Expr::Scoped`].
///
/// `Context` is a plain data struct and is part of the `serde` surface. It
/// is produced by [`ContextBuilder::build`] and never constructed directly
/// in normal usage.
///
/// # Fields
///
/// - `partitioning` — expressions that divide the input into independent
///   groups, each evaluated separately (backend: `PARTITION BY`, groupby key).
/// - `ordering` — expressions that establish the order within each group,
///   defining "current", "before", and "after" (backend: `ORDER BY`).
/// - `frame` — optional boundary descriptor that limits which items within
///   the ordered group participate in the computation.
///
/// # Example
///
/// Prefer [`ContextBuilder`] over direct construction:
///
/// ```rust
/// use dol_core::Path;
/// use dol_expr::tree::{Expr, context::{ContextBuilder, Boundary, Extent}};
///
/// # fn example<'a>(
/// #     sum_fn: dol_expr::tree::func::meta::FuncDef,
/// #     by_date: dol_expr::tree::order::OrderByExpr<'a>,
/// # ) -> Expr<'a> {
/// ContextBuilder::new(Expr::Call { func: sum_fn, args: vec![
///     Expr::Ref(Path::new("orders").get("total")),
/// ]})
/// .partitioning([Expr::Ref(Path::new("orders").get("region"))])
/// .ordering([by_date])
/// .between_positional(
///     Boundary::Before(Extent::Unbounded),
///     Boundary::Current,
/// )
/// .build()
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Context<'a> {
    /// Expressions that partition the input into independently evaluated groups.
    pub partitioning: Vec<Expr<'a>>,

    /// Expressions that define the order within each partition.
    pub ordering: Vec<OrderByExpr<'a>>,

    /// Optional bounded frame within the ordered partition.
    ///
    /// `None` means the full ordered group participates in the computation.
    pub frame: Option<ContextFrame>,
}

// ── ContextBuilder ────────────────────────────────────────────────────────────

/// Fluent builder for [`Expr::Scoped`] — a function evaluated over a
/// partitioned, ordered, and optionally bounded evaluation context.
///
/// `ContextBuilder` is transient construction state. It deliberately does
/// **not** implement `Serialize` or `Deserialize`. Call [`build`](Self::build)
/// to obtain the [`Expr`] that is part of the IR and `serde` surface.
///
/// # Method order
///
/// All setter methods are independent and may appear in any order.
/// [`build`](Self::build) must be called last.
///
/// # Examples
///
/// ```rust
/// use dol_core::Path;
/// use dol_expr::tree::{Expr, context::{ContextBuilder, Boundary, Extent}};
///
/// # fn example<'a>(
/// #     rank_fn: dol_expr::tree::func::meta::FuncDef,
/// #     by_score: dol_expr::tree::order::OrderByExpr<'a>,
/// # ) -> Expr<'a> {
/// // RANK() OVER (PARTITION BY region ORDER BY score DESC ROWS UNBOUNDED PRECEDING)
/// ContextBuilder::new(Expr::Call { func: rank_fn, args: vec![] })
///     .partitioning([Expr::Ref(Path::new("region"))])
///     .ordering([by_score])
///     .between_positional(
///         Boundary::Before(Extent::Unbounded),
///         Boundary::Current,
///     )
///     .build()
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ContextBuilder<'a> {
    func: Expr<'a>,
    partitioning: Vec<Expr<'a>>,
    ordering: Vec<OrderByExpr<'a>>,
    frame: Option<ContextFrame>,
}

impl<'a> ContextBuilder<'a> {
    /// Create a new builder for `func` — the expression to evaluate within
    /// the context.
    ///
    /// `func` is typically a [`Call`](Expr::Call) node (aggregate, analytic
    /// function), but any [`Expr`] is accepted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::{Expr, context::ContextBuilder};
    ///
    /// # fn example<'a>(sum: dol_expr::tree::func::meta::FuncDef, amount: Expr<'a>) -> ContextBuilder<'a> {
    /// ContextBuilder::new(Expr::Call { func: sum, args: vec![amount] })
    /// # }
    /// ```
    pub fn new(func: Expr<'a>) -> Self {
        Self {
            func,
            partitioning: Vec::new(),
            ordering: Vec::new(),
            frame: None,
        }
    }

    /// Set the expressions that divide the input into independently evaluated
    /// groups. Each distinct combination of partition values is evaluated
    /// separately by backends that support partitioned execution.
    ///
    /// Calling this method a second time replaces the previous partitioning.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Path;
    /// use dol_expr::tree::{Expr, context::ContextBuilder};
    ///
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// builder.partitioning([
    ///     Expr::Ref(Path::new("orders").get("region")),
    ///     Expr::Ref(Path::new("orders").get("category")),
    /// ])
    /// # }
    /// ```
    pub fn partitioning(mut self, exprs: impl IntoIterator<Item = Expr<'a>>) -> Self {
        self.partitioning = exprs.into_iter().collect();
        self
    }

    /// Set the expressions that establish the order within each partition,
    /// defining the current evaluation point and any relative frame offsets.
    ///
    /// Calling this method a second time replaces the previous ordering.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::{context::ContextBuilder, order::OrderByExpr};
    ///
    /// # fn example<'a>(
    /// #     builder: ContextBuilder<'a>,
    /// #     by_time: OrderByExpr<'a>,
    /// # ) -> ContextBuilder<'a> {
    /// builder.ordering([by_time])
    /// # }
    /// ```
    pub fn ordering(mut self, exprs: impl IntoIterator<Item = OrderByExpr<'a>>) -> Self {
        self.ordering = exprs.into_iter().collect();
        self
    }

    /// Set an explicit bounded frame using a specific [`ContextMode`].
    ///
    /// This is the most general frame helper.
    /// [`between_positional`](Self::between_positional) and
    /// [`between_value_distance`](Self::between_value_distance) are
    /// convenience wrappers around this method.
    ///
    /// Calling this method a second time replaces the previous frame.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::{Boundary, ContextBuilder, ContextMode, Extent};
    ///
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// builder.between(
    ///     Boundary::Before(Extent::Finite(5)),
    ///     Boundary::Current,
    ///     ContextMode::Positional,
    /// )
    /// # }
    /// ```
    pub fn between(mut self, start: Boundary, end: Boundary, mode: ContextMode) -> Self {
        self.frame = Some(ContextFrame {
            mode,
            start,
            end: Some(end),
        });
        self
    }

    /// Set a positional bounded frame: boundaries are measured in item offsets.
    ///
    /// Convenience wrapper for [`between`](Self::between) with
    /// [`ContextMode::Positional`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::{Boundary, ContextBuilder, Extent};
    ///
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// // 3 items back through the current point
    /// builder.between_positional(
    ///     Boundary::Before(Extent::Finite(3)),
    ///     Boundary::Current,
    /// )
    /// # }
    /// ```
    pub fn between_positional(mut self, start: Boundary, end: Boundary) -> Self {
        self.frame = Some(ContextFrame {
            mode: ContextMode::Positional,
            start,
            end: Some(end),
        });
        self
    }

    /// Set a value-distance bounded frame: boundaries are measured by
    /// comparable value distance in the ordering domain (numeric, temporal,
    /// lexical, etc.).
    ///
    /// Convenience wrapper for [`between`](Self::between) with
    /// [`ContextMode::ValueDistance`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::{Boundary, ContextBuilder, Extent};
    ///
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// // All items whose ordering value is within 10 units of the current value
    /// builder.between_value_distance(
    ///     Boundary::Before(Extent::Finite(10)),
    ///     Boundary::After(Extent::Finite(10)),
    /// )
    /// # }
    /// ```
    pub fn between_value_distance(mut self, start: Boundary, end: Boundary) -> Self {
        self.frame = Some(ContextFrame {
            mode: ContextMode::ValueDistance,
            start,
            end: Some(end),
        });
        self
    }

    /// Build the [`Expr::Scoped`] node.
    ///
    /// Consumes the builder. The produced [`Expr`] is the serialisable,
    /// arena-lowerable IR node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Path;
    /// use dol_expr::tree::{Expr, context::{ContextBuilder, Boundary, Extent}};
    ///
    /// # fn example<'a>(
    /// #     rank_fn: dol_expr::tree::func::meta::FuncDef,
    /// #     by_score: dol_expr::tree::order::OrderByExpr<'a>,
    /// # ) -> Expr<'a> {
    /// ContextBuilder::new(Expr::Call { func: rank_fn, args: vec![] })
    ///     .partitioning([Expr::Ref(Path::new("region"))])
    ///     .ordering([by_score])
    ///     .between_positional(
    ///         Boundary::Before(Extent::Unbounded),
    ///         Boundary::Current,
    ///     )
    ///     .build()
    /// # }
    /// ```
    pub fn build(self) -> Expr<'a> {
        Expr::Scoped {
            expr: Box::new(self.func),
            context: Context {
                partitioning: self.partitioning,
                ordering: self.ordering,
                frame: self.frame,
            },
        }
    }
}

// ── ConditionalBuilder ────────────────────────────────────────────────────────

/// Fluent builder for [`Expr::Match`] — a multi-arm conditional expression.
///
/// Arms are `(condition, result)` pairs evaluated top-to-bottom; the first
/// matching condition determines the result. An optional fallback is returned
/// when no arm matches.
///
/// `ConditionalBuilder` is transient construction state. It deliberately does
/// **not** implement `Serialize` or `Deserialize`. Call [`build`](Self::build)
/// to obtain the [`Expr`] that is part of the IR and `serde` surface.
///
/// For simple two-branch conditionals, construct [`Expr::If`] directly.
///
/// # Examples
///
/// ```rust
/// use dol_core::{Literal, Path};
/// use dol_expr::tree::{Expr, context::ConditionalBuilder};
///
/// // CASE WHEN score >= 90 THEN 'A' WHEN score >= 60 THEN 'B' ELSE 'C'
/// # fn example<'a>(gte_90: Expr<'a>, gte_60: Expr<'a>) -> Expr<'a> {
/// ConditionalBuilder::new()
///     .when(gte_90, Expr::Lit(Literal::str("A")))
///     .when(gte_60, Expr::Lit(Literal::str("B")))
///     .fallback(Expr::Lit(Literal::str("C")))
///     .build()
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBuilder<'a> {
    arms: Vec<(Expr<'a>, Expr<'a>)>,
    fallback: Option<Expr<'a>>,
}

impl<'a> Default for ConditionalBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ConditionalBuilder<'a> {
    /// Create an empty conditional builder.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::context::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            arms: Vec::new(),
            fallback: None,
        }
    }

    /// Add a conditional arm: if `condition` holds, the result is `outcome`.
    ///
    /// Arms are evaluated in insertion order. The first condition that holds
    /// determines the result; subsequent arms are not evaluated.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::{Expr, context::ConditionalBuilder};
    ///
    /// # fn example<'a>(is_admin: Expr<'a>, is_staff: Expr<'a>) -> ConditionalBuilder<'a> {
    /// ConditionalBuilder::new()
    ///     .when(is_admin, Expr::Lit(Literal::str("admin")))
    ///     .when(is_staff, Expr::Lit(Literal::str("staff")))
    /// # }
    /// ```
    pub fn when(mut self, condition: impl Into<Expr<'a>>, outcome: impl Into<Expr<'a>>) -> Self {
        self.arms.push((condition.into(), outcome.into()));
        self
    }

    /// Set the fallback result returned when no arm's condition holds.
    ///
    /// Without a fallback, the backend substitutes its native null / absent
    /// value for an unmatched input.
    ///
    /// Calling this method a second time replaces the previous fallback.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Literal;
    /// use dol_expr::tree::{Expr, context::ConditionalBuilder};
    ///
    /// # fn example<'a>(builder: ConditionalBuilder<'a>) -> ConditionalBuilder<'a> {
    /// builder.fallback(Expr::Lit(Literal::str("unknown")))
    /// # }
    /// ```
    pub fn fallback(mut self, result: impl Into<Expr<'a>>) -> Self {
        self.fallback = Some(result.into());
        self
    }

    /// Build the [`Expr::Match`] node.
    ///
    /// Consumes the builder. The produced [`Expr`] is the serialisable,
    /// arena-lowerable IR node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::{Expr, context::ConditionalBuilder};
    ///
    /// # fn example<'a>(active: Expr<'a>, suspended: Expr<'a>) -> Expr<'a> {
    /// ConditionalBuilder::new()
    ///     .when(active,    Expr::Lit(Literal::str("active")))
    ///     .when(suspended, Expr::Lit(Literal::str("suspended")))
    ///     .fallback(Expr::Lit(Literal::str("unknown")))
    ///     .build()
    /// # }
    /// ```
    pub fn build(self) -> Expr<'a> {
        Expr::Match {
            arms: self.arms,
            fallback: self.fallback.map(Box::new),
        }
    }
}

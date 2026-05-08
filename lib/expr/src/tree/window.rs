//! Neutral builders for contextual computations and conditional expressions.
//!
//! This module avoids storage-specific terminology such as SQL windows,
//! rows, ranges, CASE clauses, tables, or columns. Instead, it models:
//!
//! - a contextual computation over an ordered/partitioned input scope
//! - optional bounded evaluation contexts
//! - conditional branching expressions
//!
//! Backends may lower these neutral structures into SQL, dataframe plans,
//! stream processors, vector engines, or custom execution runtimes.

use alloc::{boxed::Box, vec::Vec};

use super::Expr;
use super::order::OrderByExpr;

/// A finite or unbounded extent away from the current evaluation point.
///
/// `Extent` is used by [`Boundary::Before`] and [`Boundary::After`] to describe
/// how far a context boundary reaches relative to [`Boundary::Current`].
///
/// # Examples
///
/// ```rust
/// # use dol_expr::tree::{Boundary, Extent};
/// let previous_three = Boundary::Before(Extent::Finite(3));
/// let all_previous = Boundary::Before(Extent::Unbounded);
/// let next_two = Boundary::After(Extent::Finite(2));
/// let all_next = Boundary::After(Extent::Unbounded);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Extent {
    /// A bounded extent measured in the units defined by the active
    /// [`ContextMode`].
    ///
    /// For [`ContextMode::Positional`], this usually means item offsets.
    /// For [`ContextMode::ValueDistance`], this usually means comparable value
    /// distance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::Extent;
    /// let three_units = Extent::Finite(3);
    /// ```
    Finite(u32),

    /// An extent that reaches as far as the input context allows.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::Extent;
    /// let everything_available = Extent::Unbounded;
    /// ```
    Unbounded,
}

/// A boundary relative to the current evaluation point.
///
/// `Boundary` intentionally avoids row-oriented wording. A boundary may refer
/// to positions, values, timestamps, event offsets, or any other ordered domain
/// supported by a backend through [`ContextMode`].
///
/// # Examples
///
/// ```rust
/// # use dol_expr::tree::{Boundary, Extent};
/// let start = Boundary::Before(Extent::Unbounded);
/// let end = Boundary::Current;
///
/// let local_start = Boundary::Before(Extent::Finite(5));
/// let local_end = Boundary::After(Extent::Finite(5));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Boundary {
    /// A boundary before the current evaluation point.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, Extent};
    /// let five_before = Boundary::Before(Extent::Finite(5));
    /// let from_beginning = Boundary::Before(Extent::Unbounded);
    /// ```
    Before(Extent),

    /// The current evaluation point.
    ///
    /// This is the neutral equivalent of the active cursor, anchor, or focused
    /// item in an ordered context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::Boundary;
    /// let here = Boundary::Current;
    /// ```
    Current,

    /// A boundary after the current evaluation point.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, Extent};
    /// let two_after = Boundary::After(Extent::Finite(2));
    /// let through_end = Boundary::After(Extent::Unbounded);
    /// ```
    After(Extent),
}

/// The mode used to interpret context boundaries.
///
/// A backend decides how each mode maps to its own execution model. For
/// example, SQL backends may lower [`ContextMode::Positional`] to `ROWS` and
/// [`ContextMode::ValueDistance`] to `RANGE`, while non-SQL backends may lower
/// them to offsets, sequence windows, event-distance windows, or native
/// analytical operators.
///
/// # Examples
///
/// ```rust
/// # use dol_expr::tree::ContextMode;
/// let by_position = ContextMode::Positional;
/// let by_value_distance = ContextMode::ValueDistance;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ContextMode {
    /// Boundaries are interpreted positionally.
    ///
    /// # Example
    ///
    /// A context from three items before the current point through the current
    /// point would use [`ContextMode::Positional`] with:
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, ContextFrame, ContextMode, Extent};
    /// let frame = ContextFrame {
    ///     mode: ContextMode::Positional,
    ///     start: Boundary::Before(Extent::Finite(3)),
    ///     end: Some(Boundary::Current),
    /// };
    /// ```
    Positional,

    /// Boundaries are interpreted by value distance in the active ordering
    /// domain.
    ///
    /// # Example
    ///
    /// A backend may interpret this as a numeric, temporal, lexical, or
    /// otherwise comparable distance from the current ordered value.
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, ContextFrame, ContextMode, Extent};
    /// let frame = ContextFrame {
    ///     mode: ContextMode::ValueDistance,
    ///     start: Boundary::Before(Extent::Finite(10)),
    ///     end: Some(Boundary::Current),
    /// };
    /// ```
    ValueDistance,
}

/// A bounded evaluation context for a contextual computation.
///
/// `ContextFrame` describes which neighboring input items or values participate
/// in a computation relative to the current evaluation point.
///
/// # Examples
///
/// ```rust
/// # use dol_expr::tree::{Boundary, ContextFrame, ContextMode, Extent};
/// let trailing_context = ContextFrame {
///     mode: ContextMode::Positional,
///     start: Boundary::Before(Extent::Finite(3)),
///     end: Some(Boundary::Current),
/// };
///
/// let full_context = ContextFrame {
///     mode: ContextMode::Positional,
///     start: Boundary::Before(Extent::Unbounded),
///     end: Some(Boundary::After(Extent::Unbounded)),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ContextFrame {
    /// How [`start`](Self::start) and [`end`](Self::end) are interpreted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::ContextMode;
    /// let mode = ContextMode::Positional;
    /// ```
    pub mode: Option<ContextMode>,

    /// The starting boundary of the evaluation context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, Extent};
    /// let start = Boundary::Before(Extent::Unbounded);
    /// ```
    pub start: Boundary,

    /// The optional ending boundary of the evaluation context.
    ///
    /// `None` may be used by builders or backends to represent a one-sided
    /// context if supported by the surrounding expression semantics.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::Boundary;
    /// let end = Some(Boundary::Current);
    /// ```
    pub end: Option<Boundary>,
}

/// Builder for contextual function computations.
///
/// `ContextBuilder` represents a function evaluated over a logical context.
/// That context may be partitioned, ordered, and optionally bounded by a
/// [`ContextFrame`].
///
/// This is the neutral equivalent of an analytical/window computation, but it
/// does not require the target backend to be SQL-based.
///
/// # Examples
///
/// ```rust
/// # use dol_expr::tree::{Boundary, ContextBuilder, ContextMode, Extent, Expr};
/// # fn example<'a>(func: Expr<'a>, account: Expr<'a>, timestamp_order: dol_expr::tree::order::OrderByExpr<'a>) -> Expr<'a> {
/// ContextBuilder::new(func)
///     .partitioning([account])
///     .ordering([timestamp_order])
///     .between(
///         Boundary::Before(Extent::Finite(3)),
///         Boundary::Current,
///         Some(ContextMode::Positional),
///     )
///     .build()
/// # }
/// ```
///
/// **Note:** Builder helper types (`ContextBuilder`, [`ConditionalBuilder`]) are
/// transient construction state and intentionally **not** part of the `serde`
/// surface. Persist or transmit the produced [`Expr`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextBuilder<'a> {
    func: Expr<'a>,
    partitioning: Vec<Expr<'a>>,
    ordering: Vec<OrderByExpr<'a>>,
    frame: Option<ContextFrame>,
}

impl<'a> ContextBuilder<'a> {
    /// Create a new contextual computation builder for `func`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{ContextBuilder, Expr};
    /// # fn example<'a>(func: Expr<'a>) -> ContextBuilder<'a> {
    /// ContextBuilder::new(func)
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

    /// Set the expressions that partition the input context.
    ///
    /// Each distinct partition is evaluated independently by backends that
    /// support partitioned execution.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{ContextBuilder, Expr};
    /// # fn example<'a>(builder: ContextBuilder<'a>, account_id: Expr<'a>) -> ContextBuilder<'a> {
    /// builder.partitioning([account_id])
    /// # }
    /// ```
    pub fn partitioning(mut self, exprs: impl IntoIterator<Item = Expr<'a>>) -> Self {
        self.partitioning = exprs.into_iter().collect();
        self
    }

    /// Set the ordering used to define the current evaluation point and any
    /// relative boundaries.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::ContextBuilder;
    /// # use dol_expr::tree::order::OrderByExpr;
    /// # fn example<'a>(builder: ContextBuilder<'a>, by_time: OrderByExpr<'a>) -> ContextBuilder<'a> {
    /// builder.ordering([by_time])
    /// # }
    /// ```
    pub fn ordering(mut self, exprs: impl IntoIterator<Item = OrderByExpr<'a>>) -> Self {
        self.ordering = exprs.into_iter().collect();
        self
    }

    /// Set an explicit bounded evaluation context.
    ///
    /// This is the most general context-frame helper. Convenience helpers such
    /// as [`positional_between`](Self::positional_between) and
    /// [`value_distance_between`](Self::value_distance_between) call this
    /// method internally.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, ContextBuilder, ContextMode, Extent};
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// builder.between(
    ///     Boundary::Before(Extent::Finite(5)),
    ///     Boundary::Current,
    ///     Some(ContextMode::Positional),
    /// )
    /// # }
    /// ```
    pub fn between(mut self, start: Boundary, end: Boundary, mode: Option<ContextMode>) -> Self {
        self.frame = Some(ContextFrame {
            mode,
            start,
            end: Some(end),
        });
        self
    }

    /// Set a positional bounded evaluation context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, ContextBuilder, Extent};
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// builder.between_positional(
    ///     Boundary::Before(Extent::Finite(3)),
    ///     Boundary::Current,
    /// )
    /// # }
    /// ```
    pub fn between_positional(mut self, start: Boundary, end: Boundary) -> Self {
        self.frame = Some(ContextFrame {
            mode: Some(ContextMode::Positional),
            start,
            end: Some(end),
        });
        self
    }

    /// Set a value-distance bounded evaluation context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{Boundary, ContextBuilder, Extent};
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> ContextBuilder<'a> {
    /// builder.between_distance(
    ///     Boundary::Before(Extent::Finite(10)),
    ///     Boundary::Current,
    /// )
    /// # }
    /// ```
    pub fn between_distance(mut self, start: Boundary, end: Boundary) -> Self {
        self.frame = Some(ContextFrame {
            mode: Some(ContextMode::ValueDistance),
            start,
            end: Some(end),
        });
        self
    }

    /// Build the final contextual computation expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{ContextBuilder, Expr};
    /// # fn example<'a>(builder: ContextBuilder<'a>) -> Expr<'a> {
    /// builder.build()
    /// # }
    /// ```
    pub fn build(self) -> Expr<'a> {
        Expr::Window {
            func: Box::new(self.func),
            partitioning: self.partitioning,
            ordering: self.ordering,
            frame: self.frame,
        }
    }
}

/// Builder for conditional branching expressions.
///
/// `ConditionalBuilder` constructs expressions that choose a result by testing
/// conditions in order. The first matching condition determines the result; if
/// no condition matches, the optional default result is used.
///
/// This is the neutral equivalent of SQL `CASE`, functional `match` guards, or
/// rule-based conditional selection.
///
/// # Examples
///
/// ```rust
/// # use dol_expr::tree::{ConditionalBuilder, Expr};
/// # fn example<'a>(condition: Expr<'a>, result: Expr<'a>, fallback: Expr<'a>) -> Expr<'a> {
/// ConditionalBuilder::new()
///     .if_(condition, result)
///     .default(fallback)
///     .build()
/// # }
/// ```
///
/// **Note:** Builder helper types ([`ContextBuilder`], `ConditionalBuilder`) are
/// transient construction state and intentionally **not** part of the `serde`
/// surface. Persist or transmit the produced [`Expr`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBuilder<'a> {
    whens: Vec<(Expr<'a>, Expr<'a>)>,
    else_expr: Option<Expr<'a>>,
}

impl<'a> Default for ConditionalBuilder<'a> {
    /// Create an empty conditional builder.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::ConditionalBuilder;
    /// let builder = ConditionalBuilder::default();
    /// ```
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
    /// # use dol_expr::tree::ConditionalBuilder;
    /// let builder = ConditionalBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            whens: Vec::new(),
            else_expr: None,
        }
    }

    /// Add a conditional branch.
    ///
    /// Branches are evaluated in insertion order. The first condition that
    /// matches determines the resulting expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{ConditionalBuilder, Expr};
    /// # fn example<'a>(condition: Expr<'a>, expr: Expr<'a>) -> ConditionalBuilder<'a> {
    /// ConditionalBuilder::new().when(condition, expr)
    /// # }
    /// ```
    pub fn when(mut self, condition: impl Into<Expr<'a>>, expr: impl Into<Expr<'a>>) -> Self {
        self.whens.push((condition.into(), expr.into()));
        self
    }

    /// Set the default result used when no branch condition matches.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{ConditionalBuilder, Expr};
    /// # fn example<'a>(builder: ConditionalBuilder<'a>, fallback: Expr<'a>) -> ConditionalBuilder<'a> {
    /// builder.default(fallback)
    /// # }
    /// ```
    pub fn default(mut self, result: impl Into<Expr<'a>>) -> Self {
        self.else_expr = Some(result.into());
        self
    }

    /// Build the final conditional expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dol_expr::tree::{ConditionalBuilder, Expr};
    /// # fn example<'a>(builder: ConditionalBuilder<'a>) -> Expr<'a> {
    /// builder.build()
    /// # }
    /// ```
    pub fn build(self) -> Expr<'a> {
        Expr::Case {
            whens: self.whens,
            else_expr: self.else_expr.map(Box::new),
        }
    }
}

//! Expression AST — the single composable, backend-agnostic node type for DOL.
//!
//! [`Expr<'a>`] is the surface type through which callers build queries,
//! predicates, projections, and computations. It is intentionally store-neutral:
//! no SQL keywords, no REST verbs, no document-store concepts appear in variant
//! or field names.
//!
//! The lifetime `'a` threads through [`Literal<'a>`], enabling zero-copy borrows
//! of string and byte data during AST construction. Lowering into the packed
//! [`ExprArena`] erases the lifetime and produces a fully owned, pointer-free
//! representation.
//!
//! # Reference model
//!
//! All addressable targets are expressed through the single
//! [`Ref`](Expr::Ref) variant backed by [`Path`]. The presence or absence of a
//! field chain on the [`Path`] carries the container-vs-leaf distinction:
//!
//! | Construction                                          | Meaning                 |
//! |-------------------------------------------------------|-------------------------|
//! | `Ref(Path::new("orders"))`                            | Container / entity      |
//! | `Ref(Path::new("orders").get("total"))`               | Leaf attribute          |
//! | `Ref(Path::new("orders").get("meta").get("tag"))`     | Nested leaf             |
//! | `Ref(Path::new("orders").namespace("billing"))`       | Scoped container        |
//!
//! # Negation convention
//!
//! [`InRange`](Expr::InRange) and [`MemberOf`](Expr::MemberOf) have no dedicated
//! negated variants. For `NOT BETWEEN` / `NOT IN` semantics, wrap with
//! `Unary { op: UnaryOp::Not, expr: … }`, consistent with how `NOT LIKE` is
//! modelled.
//!
//! # Quick start
//!
//! ```rust
//! use dol_core::{Literal, Path};
//! use dol_core::strings::Name;
//! use dol_expr::tree::Expr;
//!
//! // orders.total > 100
//! let total    = Expr::Ref(Path::new("orders").get("total"));
//! let hundred  = Expr::Lit(Literal::int64(100));
//!
//! // status IN ('active', 'pending')
//! let status = Expr::MemberOf {
//!     expr: Box::new(Expr::Ref(Path::new("status"))),
//!     set: vec![
//!         Expr::Lit(Literal::str("active")),
//!         Expr::Lit(Literal::str("pending")),
//!     ],
//! };
//!
//! // score BETWEEN 0 AND 100
//! let range = Expr::InRange {
//!     expr: Box::new(Expr::Ref(Path::new("score"))),
//!     low:  Box::new(Expr::Lit(Literal::int64(0))),
//!     high: Box::new(Expr::Lit(Literal::int64(100))),
//! };
//! ```

use alloc::{boxed::Box, vec::Vec};

use dol_core::strings::Name;
use dol_core::{Literal, Path};

use super::context::Context;
use super::func::meta::FuncDef;
use super::op::UnaryOp;
use super::op::meta::OpDef;

/// A composable, backend-agnostic expression node.
///
/// See the [module documentation](self) for the reference model, negation
/// convention, and construction examples.
///
/// `'a` allows zero-copy [`Literal`] borrows during AST construction. Call
/// `.into_owned()` or lower into an [`ExprArena`] to erase it.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Expr<'a> {
    // ── References ──────────────────────────────────────────────────────────
    /// A structured path to a container, scoped container, or leaf attribute.
    ///
    /// The three roles are encoded in the [`Path`] itself:
    ///
    /// - **No field chain** → container / entity address.
    /// - **One or more field segments** → leaf attribute or nested leaf.
    /// - **Namespace present** → authority / scope prefix on the above.
    ///
    /// Use [`Path::new`] for plain targets and chain [`.get()`](Path::get)
    /// for nested access. [`.namespace()`](Path::namespace) attaches a scope.
    ///
    /// Lowers to `ExprNode::Ref` in the arena.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use dol_core::Path;
    /// use dol_expr::tree::Expr;
    ///
    /// // Container / entity address
    /// let entity = Expr::Ref(Path::new("orders"));
    ///
    /// // Leaf attribute: orders.total
    /// let leaf = Expr::Ref(Path::new("orders").get("total"));
    ///
    /// // Nested leaf: orders.meta.tag
    /// let nested = Expr::Ref(Path::new("orders").get("meta").get("tag"));
    ///
    /// // Scoped container: billing.orders
    /// let scoped = Expr::Ref(Path::new("orders").namespace("billing"));
    ///
    /// // Scoped leaf: billing.orders.total
    /// let scoped_leaf = Expr::Ref(
    ///     Path::new("orders").namespace("billing").get("total")
    /// );
    /// ```
    Ref(Path),

    // ── Values ───────────────────────────────────────────────────────────────
    /// A positional bind parameter: `$1`, `?`, `@p1`, `:1`, etc.
    ///
    /// DOL tracks only the *presence* of a parameter slot. Backends number
    /// parameters from their own call sites; the syntax varies per store.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::Expr;
    ///
    /// let slot = Expr::Param; // backend renders as $1, ?, :p1, …
    /// ```
    Param,

    /// A typed literal value from the unified `dol_core` type system.
    ///
    /// [`Literal<'a>`] covers all scalar types: integers, decimals, booleans,
    /// strings, bytes, dates, IP addresses, UUIDs, and more. The lifetime
    /// allows zero-copy borrows during AST construction; lower into an
    /// [`ExprArena`] or call `.into_owned()` to obtain a `'static` literal.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use dol_core::Literal;
    /// use dol_expr::tree::Expr;
    ///
    /// let n    = Expr::Lit(Literal::int64(42));
    /// let s    = Expr::Lit(Literal::str("hello"));
    /// let flag = Expr::Lit(Literal::bool(true));
    /// ```
    Lit(Literal<'a>),

    /// An ordered sequence of values: `[elem₀, elem₁, …]`.
    ///
    /// Backends may render this as an array literal, tuple, multi-value
    /// constructor, or ordered list depending on context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Literal;
    /// use dol_expr::tree::Expr;
    ///
    /// let seq = Expr::Seq(vec![
    ///     Expr::Lit(Literal::int64(1)),
    ///     Expr::Lit(Literal::int64(2)),
    ///     Expr::Lit(Literal::int64(3)),
    /// ]);
    /// ```
    Seq(Vec<Expr<'a>>),

    /// A key-value mapping literal: `{ key: expr, … }`.
    ///
    /// Keys are [`Name`]s — short, intern-friendly strings. Backends may
    /// render this as a JSON object, map constructor, named-tuple, or
    /// document literal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, strings::Name};
    /// use dol_expr::tree::Expr;
    ///
    /// let address = Expr::Map(vec![
    ///     (Name::new("city"),    Expr::Lit(Literal::str("Accra"))),
    ///     (Name::new("country"), Expr::Lit(Literal::str("GH"))),
    /// ]);
    /// ```
    Map(Vec<(Name, Expr<'a>)>),

    // ── Operations ───────────────────────────────────────────────────────────
    /// A binary operation: `left op right`.
    ///
    /// [`OpDef`] carries the operation's identity, precedence, associativity,
    /// and any dialect hints, so backends do not need to pattern-match on a
    /// flat enum for rendering decisions.
    ///
    /// Negated binary forms (`NOT LIKE`, `NOT ILIKE`, …) are expressed by
    /// wrapping the result in `Unary { op: UnaryOp::Not, … }`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::{Expr, op::{OpDef, BinOp}};
    ///
    /// let age_ref = Expr::Ref(Path::new("users").get("age"));
    /// let limit   = Expr::Lit(Literal::int64(18));
    ///
    /// // age > 18
    /// let check = Expr::Binary {
    ///     left:  Box::new(age_ref),
    ///     op:    OpDef::from(BinOp::Gt),
    ///     right: Box::new(limit),
    /// };
    /// ```
    Binary {
        /// Left-hand operand.
        left: Box<Expr<'a>>,
        /// The operation, including precedence and dialect metadata.
        op: OpDef,
        /// Right-hand operand.
        right: Box<Expr<'a>>,
    },

    /// A unary operation: `op expr`.
    ///
    /// Covers logical negation (`Not`), arithmetic negation (`Neg`), bitwise
    /// complement (`BitNot`), and null-check predicates (`IsNull`,
    /// `IsNotNull`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Path;
    /// use dol_expr::tree::{Expr, op::UnaryOp};
    ///
    /// // IS NULL
    /// let is_null = Expr::Unary {
    ///     op:   UnaryOp::IsNull,
    ///     expr: Box::new(Expr::Ref(Path::new("users").get("deleted_at"))),
    /// };
    ///
    /// // NOT (status IN ('a', 'b'))  — negated MemberOf
    /// # fn example<'a>(member_of: Expr<'a>) -> Expr<'a> {
    /// Expr::Unary {
    ///     op:   UnaryOp::Not,
    ///     expr: Box::new(member_of),
    /// }
    /// # }
    /// ```
    Unary {
        /// The unary operator.
        op: UnaryOp,
        /// The operand.
        expr: Box<Expr<'a>>,
    },

    // ── Calls ────────────────────────────────────────────────────────────────
    /// A function or aggregate call: `func(args…)`.
    ///
    /// [`FuncDef`] carries the function's identity, return type, and backend
    /// hints (aggregate, deterministic, etc.). `args` is empty for
    /// zero-argument functions such as `NOW()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Path;
    /// use dol_expr::tree::Expr;
    ///
    /// // coalesce(users.nickname, users.name)
    /// # fn example<'a>(
    /// #     coalesce: dol_expr::tree::func::meta::FuncDef,
    /// # ) -> Expr<'a> {
    /// Expr::Call {
    ///     func: coalesce,
    ///     args: vec![
    ///         Expr::Ref(Path::new("users").get("nickname")),
    ///         Expr::Ref(Path::new("users").get("name")),
    ///     ],
    /// }
    /// # }
    /// ```
    Call {
        /// The function identity and metadata.
        func: FuncDef,
        /// Positional arguments, evaluated left to right.
        args: Vec<Expr<'a>>,
    },

    // ── Structural ───────────────────────────────────────────────────────────
    /// A type coercion: `expr` cast to `target_type`.
    ///
    /// Backends render this as `CAST(expr AS T)`, an implicit coercion, or a
    /// type constructor, depending on their capability set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::Path;
    /// use dol_expr::tree::Expr;
    /// use dol_expr::types::DataType;
    ///
    /// let cast = Expr::Cast {
    ///     expr:        Box::new(Expr::Ref(Path::new("events").get("ts"))),
    ///     target_type: DataType::Timestamp,
    /// };
    /// ```
    Cast {
        /// The expression to coerce.
        expr: Box<Expr<'a>>,
        /// The target type.
        target_type: crate::types::DataType,
    },

    /// A multi-arm conditional: first matching arm wins.
    ///
    /// `arms` are `(condition, result)` pairs evaluated top-to-bottom. When
    /// no condition holds, `fallback` is returned. `fallback: None` lets the
    /// backend substitute its native null / absent value.
    ///
    /// Prefer [`ConditionalBuilder`](super::context::ConditionalBuilder) for
    /// ergonomic, incremental construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::{Expr, context::ConditionalBuilder};
    ///
    /// // CASE WHEN score >= 90 THEN 'A' WHEN score >= 60 THEN 'B' ELSE 'C'
    /// let grade = ConditionalBuilder::new()
    ///     .when(
    ///         Expr::Ref(Path::new("score")), // simplified — real: score >= 90
    ///         Expr::Lit(Literal::str("A")),
    ///     )
    ///     .when(
    ///         Expr::Ref(Path::new("score")),
    ///         Expr::Lit(Literal::str("B")),
    ///     )
    ///     .fallback(Expr::Lit(Literal::str("C")))
    ///     .build();
    /// ```
    Match {
        /// Ordered condition–result pairs; first match wins.
        arms: Vec<(Expr<'a>, Expr<'a>)>,
        /// Result when no arm matches. `None` yields the backend's null /
        /// absent value.
        fallback: Option<Box<Expr<'a>>>,
    },

    /// A two-branch conditional: `IF cond THEN then_expr ELSE else_expr`.
    ///
    /// Use this instead of a single-arm [`Match`](Expr::Match) when both
    /// branches are known at construction time. Backends may lower it to a
    /// ternary expression, `IIF`, or a single-arm `CASE`, as appropriate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::Expr;
    ///
    /// // IF user.active THEN 'welcome' ELSE 'locked'
    /// let msg = Expr::If {
    ///     cond:      Box::new(Expr::Ref(Path::new("user").get("active"))),
    ///     then_expr: Box::new(Expr::Lit(Literal::str("welcome"))),
    ///     else_expr: Box::new(Expr::Lit(Literal::str("locked"))),
    /// };
    /// ```
    If {
        /// The condition to evaluate.
        cond: Box<Expr<'a>>,
        /// Result when `cond` is truthy.
        then_expr: Box<Expr<'a>>,
        /// Result when `cond` is falsy.
        else_expr: Box<Expr<'a>>,
    },

    /// Inclusive range containment: `low ≤ expr ≤ high`.
    ///
    /// Both bounds are always inclusive. For the negated form, wrap with
    /// `Unary { op: UnaryOp::Not, … }`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::Expr;
    ///
    /// // orders.total BETWEEN 10 AND 500
    /// let in_range = Expr::InRange {
    ///     expr: Box::new(Expr::Ref(Path::new("orders").get("total"))),
    ///     low:  Box::new(Expr::Lit(Literal::int64(10))),
    ///     high: Box::new(Expr::Lit(Literal::int64(500))),
    /// };
    ///
    /// // NOT BETWEEN — wrap with Unary
    /// let not_in_range = Expr::Unary {
    ///     op:   UnaryOp::Not,
    ///     expr: Box::new(in_range),
    /// };
    /// ```
    InRange {
        /// The expression tested for containment.
        expr: Box<Expr<'a>>,
        /// Inclusive lower bound.
        low: Box<Expr<'a>>,
        /// Inclusive upper bound.
        high: Box<Expr<'a>>,
    },

    /// Set membership: `expr ∈ set`.
    ///
    /// For the negated form (`NOT IN`), wrap with
    /// `Unary { op: UnaryOp::Not, … }`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Literal, Path};
    /// use dol_expr::tree::Expr;
    ///
    /// // orders.status IN ('pending', 'processing')
    /// let member = Expr::MemberOf {
    ///     expr: Box::new(Expr::Ref(Path::new("orders").get("status"))),
    ///     set: vec![
    ///         Expr::Lit(Literal::str("pending")),
    ///         Expr::Lit(Literal::str("processing")),
    ///     ],
    /// };
    ///
    /// // NOT IN — wrap with Unary
    /// let not_member = Expr::Unary {
    ///     op:   UnaryOp::Not,
    ///     expr: Box::new(member),
    /// };
    /// ```
    MemberOf {
        /// The expression whose value is tested for membership.
        expr: Box<Expr<'a>>,
        /// The candidate set. Evaluation may short-circuit on the first match.
        set: Vec<Expr<'a>>,
    },

    // ── Projection decorators ────────────────────────────────────────────────
    /// Attach an output label to an expression: `expr AS name`.
    ///
    /// The label is used as the output field name in projections, derived
    /// relations, and response envelopes. This is the neutral replacement for
    /// SQL's `AS alias`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_core::{Path, strings::Name};
    /// use dol_expr::tree::Expr;
    ///
    /// // orders.total AS amount
    /// let labelled = Expr::Label {
    ///     expr: Box::new(Expr::Ref(Path::new("orders").get("total"))),
    ///     name: Name::new("amount"),
    /// };
    /// ```
    Label {
        /// The expression to label.
        expr: Box<Expr<'a>>,
        /// The output field name.
        name: Name,
    },

    /// All fields in the current projection scope.
    ///
    /// Backends expand this to every field visible at the current scope, or
    /// apply their own wildcard semantics. Neutral replacement for SQL `*`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::Expr;
    ///
    /// let all = Expr::Wildcard;
    /// ```
    Wildcard,

    /// Aggregate over the complete input with no field argument.
    ///
    /// Neutral replacement for SQL `COUNT(*)`. Kept separate from
    /// [`Call`](Expr::Call) because it carries no operand and backends must
    /// not apply `DISTINCT`, filter pushdown, or partial-aggregate rewrites
    /// to it.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dol_expr::tree::Expr;
    ///
    /// let n = Expr::CountAll;
    /// ```
    CountAll,

    // ── Scoping ──────────────────────────────────────────────────────────────
    /// Evaluate `expr` within an explicit dynamic scope.
    ///
    /// [`Context`] carries the partitioning keys, ordering expressions, and
    /// optional bounded frame that together define the evaluation environment.
    /// Backends translate this into their native construct: a SQL `OVER (…)`
    /// clause, a dataframe groupby+sort, an IoT stream slice, a time-series
    /// segment, etc.
    ///
    /// Prefer [`ContextBuilder`](super::context::ContextBuilder) for fluent
    /// construction.
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
    Scoped {
        /// The expression evaluated within the dynamic scope.
        expr: Box<Expr<'a>>,
        /// Partitioning, ordering, and optional frame — the resolved context.
        context: Context<'a>,
    },
}

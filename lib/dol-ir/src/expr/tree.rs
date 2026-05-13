//! Tree DSL — the `Expr<'a>` AST.
//!
//! Per `dol-rewrite-plan-v2.md` §8.2. This is the user-facing builder
//! tree: a recursive enum that captures every shape DOL exposes
//! before lowering into the flat [`ExprArena`](crate::expr::arena::ExprArena).
//!
//! The lifetime `'a` threads through [`Literal<'a>`] for zero-copy
//! string and byte borrows during construction. Lowering erases it.
//!
//! ## Design choices (per plan §8.2 rationale)
//!
//! - [`Expr::Ref`] carries a [`Path`] — schema resolution to a
//!   `FieldId` happens at lowering, not at construction. The tree
//!   stays usable even when no schema is loaded.
//! - [`Expr::Scoped`] is the universal "evaluate inside an explicit
//!   scope" operator. Backends translate it to SQL `OVER (…)`,
//!   dataframe groupby+sort, IoT stream slice, time-series segment,
//!   pipeline window — there is no separate `WindowNode` side pool.
//! - [`Expr::Match`] (multi-arm) and [`Expr::If`] (two-branch) coexist:
//!   `If` having both branches at construction time deserves its own
//!   variant so optimizers and backends pattern-match without inspecting
//!   single-arm `Match`.
//! - [`Expr::CountAll`] is separate from [`Expr::Call`] — it carries no
//!   operand and backends must not apply DISTINCT, filter pushdown, or
//!   partial-aggregate rewrites to it.
//! - Negated binary forms (`NOT LIKE`, `NOT IN`, `NOT BETWEEN`) wrap
//!   with `Expr::Unary { op: UnaryOp::Not, … }` — the binary-op table
//!   stays minimal and the wire format does not duplicate opcodes.
//!
//! ## Status
//!
//! M3c-γ ships the **data shapes only**. Construction is via struct
//! literals or future fluent builders (`ContextBuilder`,
//! `ConditionalBuilder`) which arrive in M3c-δ together with the
//! lowering pipeline `Expr<'a> → ExprArena`.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use dol_core::data_type::DataType;
use dol_core::literal::Literal;
use dol_core::path::Path;
use dol_core::strings::Name;

use crate::expr::context::Context;
use crate::expr::meta::{FuncDef, OpDef};
use crate::expr::ops::UnaryOp;

/// The tree DSL — a structurally typed expression as seen by the
/// builder layer.
///
/// See the [module docs](self) for the rationale behind the variant
/// list.
///
/// `'a` is the borrow lifetime of any [`Literal::String`] / `Bytes` /
/// `Json` / `Xml` / `Enum` payloads carried inside [`Expr::Lit`].
/// Owned literals satisfy `'static`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Expr<'a> {
    // ── References ────────────────────────────────────────────────
    /// A structured path to a container, scoped container, or leaf
    /// attribute.
    ///
    /// - No field chain → container / entity address
    /// - One or more fields → leaf attribute or nested leaf
    /// - Namespace present → authority / scope prefix on the above
    ///
    /// Lowers to [`OpFamily::FieldRef`](crate::expr::ops::OpFamily::FieldRef)
    /// with an interned [`Path<Lid<StrTag>>`](dol_core::path::Path).
    Ref(Path),

    // ── Values ────────────────────────────────────────────────────
    /// A positional bind parameter: `$1`, `?`, `@p1`, `:1`, etc.
    /// DOL tracks only the presence of a slot; backends assign
    /// numbering during emission.
    Param,

    /// A typed literal value from the unified [`dol_core::literal`]
    /// type system.
    Lit(Literal<'a>),

    /// An ordered sequence of values: `[elem₀, elem₁, …]`.
    ///
    /// Backends render as array, tuple, multi-value constructor, or
    /// list literal.
    Seq(Vec<Expr<'a>>),

    /// A key-value mapping literal: `{ key: expr, … }`.
    ///
    /// Backends render as JSON object, map constructor, or document
    /// literal.
    Map(Vec<(Name, Expr<'a>)>),

    // ── Operations ────────────────────────────────────────────────
    /// A binary operation: `left op right`.
    ///
    /// [`OpDef`] carries identity + category. Negated binary forms
    /// (`NOT LIKE`, `NOT ILIKE`, …) compose with [`Expr::Unary`]
    /// using [`UnaryOp::Not`].
    Binary {
        /// Left operand.
        left: Box<Expr<'a>>,
        /// Operator definition.
        op: OpDef,
        /// Right operand.
        right: Box<Expr<'a>>,
    },

    /// A unary operation: `op expr`.
    ///
    /// Covers `Not`, `Neg`, `BitNot`, `IsNull`, `IsNotNull`.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        expr: Box<Expr<'a>>,
    },

    // ── Calls ─────────────────────────────────────────────────────
    /// A function or aggregate call: `func(args…)`.
    ///
    /// [`FuncDef`] carries identity, arity, and kind (Scalar /
    /// Aggregate / …). `args` is empty for zero-argument functions
    /// such as `NOW()`.
    Call {
        /// Function definition.
        func: FuncDef,
        /// Argument list.
        args: Vec<Expr<'a>>,
    },

    // ── Structural ────────────────────────────────────────────────
    /// A type coercion: `CAST(expr AS target_type)`.
    Cast {
        /// Expression to coerce.
        expr: Box<Expr<'a>>,
        /// Target type.
        target_type: DataType,
    },

    /// A multi-arm conditional: first matching arm wins.
    ///
    /// `fallback: None` lets the backend substitute its native null
    /// value. Use `ConditionalBuilder` (M3c-δ) for ergonomic
    /// incremental construction.
    Match {
        /// `(condition, result)` pairs evaluated in order.
        arms: Vec<(Expr<'a>, Expr<'a>)>,
        /// Optional fallback when no arm matches.
        fallback: Option<Box<Expr<'a>>>,
    },

    /// A two-branch conditional: `IF cond THEN then_expr ELSE
    /// else_expr`.
    ///
    /// Distinct from a single-arm `Match`: both branches are known at
    /// construction time, so optimizers and backends may lower to
    /// ternary, IIF, or CASE without scrutinising a `Match` shape.
    If {
        /// Branch condition.
        cond: Box<Expr<'a>>,
        /// Value if `cond` is true.
        then_expr: Box<Expr<'a>>,
        /// Value if `cond` is false.
        else_expr: Box<Expr<'a>>,
    },

    /// Inclusive range containment: `low ≤ expr ≤ high`.
    ///
    /// For `NOT BETWEEN`, wrap with [`Expr::Unary`] +
    /// [`UnaryOp::Not`].
    InRange {
        /// Expression under test.
        expr: Box<Expr<'a>>,
        /// Lower bound (inclusive).
        low: Box<Expr<'a>>,
        /// Upper bound (inclusive).
        high: Box<Expr<'a>>,
    },

    /// Set membership: `expr ∈ set`.
    ///
    /// For `NOT IN`, wrap with [`Expr::Unary`] + [`UnaryOp::Not`].
    /// Evaluation may short-circuit on the first match.
    MemberOf {
        /// Expression under test.
        expr: Box<Expr<'a>>,
        /// Candidate set.
        set: Vec<Expr<'a>>,
    },

    // ── Projection decorators ─────────────────────────────────────
    /// Attach an output label to an expression: `expr AS name`.
    /// Neutral replacement for SQL `AS alias`.
    Label {
        /// Expression being labeled.
        expr: Box<Expr<'a>>,
        /// Output label.
        name: Name,
    },

    /// All fields in the current projection scope.
    /// Backends expand to every visible field. Neutral replacement
    /// for `*`.
    Wildcard,

    /// Aggregate over the complete input with no field argument.
    /// Neutral replacement for `COUNT(*)`.
    ///
    /// Kept separate from [`Expr::Call`] so that backends cannot
    /// apply DISTINCT, filter pushdown, or partial-aggregate
    /// rewrites to it.
    CountAll,

    // ── Scoping ───────────────────────────────────────────────────
    /// Evaluate `expr` within an explicit dynamic scope.
    ///
    /// [`Context`] carries partitioning keys, ordering expressions,
    /// and an optional bounded frame. Backends translate this to
    /// their native construct: SQL `OVER (…)`, dataframe
    /// groupby+sort, IoT stream slice, time-series segment, pipeline
    /// window.
    ///
    /// Use `ContextBuilder` (M3c-δ) for fluent construction.
    Scoped {
        /// Expression evaluated in the scope.
        expr: Box<Expr<'a>>,
        /// Scope description.
        context: Context<'a>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::frame::{Boundary, Frame};
    use crate::expr::meta::{Arity, FuncKind, OpCategory};
    use crate::expr::order::{NullsOrder, SortDirection};
    use dol_core::strings::Name;

    // Helper: construct a trivial integer literal expression.
    fn lit_i64(n: i64) -> Expr<'static> {
        Expr::Lit(Literal::Int64(n))
    }

    #[test]
    fn leaf_variants_construct() {
        let _ = Expr::Param;
        let _ = Expr::Wildcard;
        let _ = Expr::CountAll;
        let _ = lit_i64(7);
        let _ = Expr::Ref(Path::new("users"));
    }

    #[test]
    fn binary_round_trip_through_clone_and_eq() {
        let e = Expr::Binary {
            left: Box::new(lit_i64(1)),
            op: OpDef::new_static("=", OpCategory::Comparison),
            right: Box::new(lit_i64(2)),
        };
        let c = e.clone();
        assert_eq!(e, c);
    }

    #[test]
    fn unary_wraps_expression() {
        let inner = Expr::Binary {
            left: Box::new(Expr::Ref(Path::new("a"))),
            op: OpDef::new_static("LIKE", OpCategory::Pattern),
            right: Box::new(Expr::Lit(Literal::String("%x%".into()))),
        };
        let neg = Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(inner.clone()),
        };
        let Expr::Unary { op, expr } = neg else {
            panic!("not unary")
        };
        assert_eq!(op, UnaryOp::Not);
        assert_eq!(*expr, inner);
    }

    #[test]
    fn call_carries_funcdef_and_args() {
        let f = FuncDef::new_static("LENGTH", Arity::Exact(1), FuncKind::Scalar);
        let e = Expr::Call {
            func: f.clone(),
            args: alloc::vec![Expr::Ref(Path::new("name"))],
        };
        let Expr::Call { func, args } = e else {
            panic!("not call")
        };
        assert_eq!(func, f);
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn structural_variants_construct() {
        // Cast
        let c = Expr::Cast {
            expr: Box::new(lit_i64(1)),
            target_type: DataType::Int32,
        };
        // Match (with and without fallback)
        let m_with = Expr::Match {
            arms: alloc::vec![(lit_i64(1), Expr::Lit(Literal::Bool(true)))],
            fallback: Some(Box::new(Expr::Lit(Literal::Bool(false)))),
        };
        let m_without = Expr::Match {
            arms: alloc::vec![(lit_i64(2), Expr::Lit(Literal::Bool(true)))],
            fallback: None,
        };
        // If
        let i = Expr::If {
            cond: Box::new(Expr::Lit(Literal::Bool(true))),
            then_expr: Box::new(lit_i64(10)),
            else_expr: Box::new(lit_i64(20)),
        };
        // InRange / MemberOf
        let r = Expr::InRange {
            expr: Box::new(Expr::Ref(Path::new("age"))),
            low: Box::new(lit_i64(18)),
            high: Box::new(lit_i64(99)),
        };
        let s = Expr::MemberOf {
            expr: Box::new(Expr::Ref(Path::new("status"))),
            set: alloc::vec![Expr::Lit(Literal::String("a".into()))],
        };
        // Quick assertions to use them.
        for e in [c, m_with, m_without, i, r, s] {
            // Round-trip equality via clone.
            assert_eq!(e.clone(), e);
        }
    }

    #[test]
    fn projection_decorators_construct() {
        let l = Expr::Label {
            expr: Box::new(lit_i64(1)),
            name: Name::Static("one"),
        };
        let w = Expr::Wildcard;
        let c = Expr::CountAll;
        // CountAll and Wildcard do not equal each other.
        assert_ne!(w.clone(), c.clone());
        // Label preserves payload.
        assert_eq!(l.clone(), l);
    }

    #[test]
    fn scoped_holds_context_with_frame_and_order() {
        // Build a Context: PARTITION BY user, ORDER BY ts ASC,
        // ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW.
        let ctx = Context {
            partition_by: alloc::vec![Expr::Ref(Path::new("user"))],
            order_by: alloc::vec![crate::expr::order::OrderByExpr {
                expr: Expr::Ref(Path::new("ts")),
                dir: SortDirection::Asc,
                nulls: NullsOrder::Default,
            }],
            frame: Some(Frame::rows(
                Boundary::unbounded_preceding(),
                Boundary::Current,
            )),
        };
        let scoped = Expr::Scoped {
            expr: Box::new(Expr::CountAll),
            context: ctx.clone(),
        };
        let Expr::Scoped { context, .. } = scoped else {
            panic!()
        };
        assert_eq!(context, ctx);
    }
}

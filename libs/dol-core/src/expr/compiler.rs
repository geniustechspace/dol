//! Generic expression compiler — delegates rendering to a backend [`ExprRenderer`].
//!
//! The compiler walks the expression tree and handles:
//! - Recursive tree traversal with depth limiting
//! - Arity validation for function calls
//! - Delegation to the backend for all rendering decisions
//!
//! Backends implement [`ExprRenderer`] to produce their output format.

use super::window::{FrameBound, FrameKind, WindowFrame};
use super::{Expr, FuncDef, Literal, OpDef, OrderByExpr};
use super::ops::UnaryOp;
use crate::op::BackendError;
use crate::types::DataType;

/// Maximum expression nesting depth.
pub const MAX_EXPR_DEPTH: usize = 128;

/// A backend must implement this trait to render expressions.
///
/// The compiler calls these methods for each expression variant, passing
/// already-rendered sub-expressions as strings.
pub trait ExprRenderer {
    // ── References ───────────────────────────────────────────────────────────

    /// Render a path reference: `field`, `table.field`, `schema.table.field`.
    ///
    /// `segments` is always non-empty. Default: join with `.`.
    fn render_ref(&mut self, segments: &[&str]) -> Result<String, BackendError> {
        Ok(segments.join("."))
    }

    /// Render sub-path access on an expression result: `expr.field`, `expr.a.b`.
    ///
    /// `path` is always non-empty. Default: `base.path` (dot-joined).
    ///
    /// Backends that support nested JSON/document access (PostgreSQL `->>`,
    /// SQLite `json_extract`, etc.) override this method.
    fn render_access(&mut self, base: &str, path: &[&str]) -> Result<String, BackendError> {
        Ok(format!("{}.{}", base, path.join(".")))
    }

    // ── Values ───────────────────────────────────────────────────────────────

    /// Render a bind parameter placeholder.
    ///
    /// Default: `?`.
    fn render_param(&mut self) -> Result<String, BackendError> {
        Ok("?".into())
    }

    /// Render a literal value.
    ///
    /// **Required** — escaping and formatting rules vary between backends.
    fn render_literal(&mut self, lit: &Literal<'_>) -> Result<String, BackendError>;

    /// Render an array literal: `[elem, ...]`.
    ///
    /// **Required** — representation is highly backend-specific.
    fn render_array_literal(
        &mut self,
        elements: &[String],
    ) -> Result<String, BackendError>;

    /// Render an object literal: `{ key: value, ... }`.
    ///
    /// **Required** — representation is highly backend-specific.
    fn render_object_literal(
        &mut self,
        pairs: &[(&str, String)],
    ) -> Result<String, BackendError>;

    // ── Operations ───────────────────────────────────────────────────────────

    /// Render a binary operation: `lhs op rhs`.
    ///
    /// **Required** — operator-to-token mapping is backend-specific.
    fn render_binary_op(
        &mut self,
        lhs: &str,
        op:  &OpDef,
        rhs: &str,
    ) -> Result<String, BackendError>;

    /// Render a unary operation: `op expr`.
    ///
    /// Default: SQL-standard forms for all variants.
    fn render_unary_op(
        &mut self,
        op:    UnaryOp,
        inner: &str,
    ) -> Result<String, BackendError> {
        Ok(match op {
            UnaryOp::Not      => format!("NOT ({})", inner),
            UnaryOp::Neg      => format!("-({})", inner),
            UnaryOp::BitNot   => format!("~({})", inner),
            UnaryOp::IsNull   => format!("{} IS NULL", inner),
            UnaryOp::IsNotNull => format!("{} IS NOT NULL", inner),
        })
    }

    // ── Calls ────────────────────────────────────────────────────────────────

    /// Render a function call: `name(args…)`.
    ///
    /// Default: `NAME(arg1, arg2, …)`. No-parens keywords
    /// (e.g. `CURRENT_DATE`) are emitted without parentheses.
    fn render_func(
        &mut self,
        def:           &FuncDef,
        rendered_args: &[String],
    ) -> Result<String, BackendError> {
        if def.is_no_parens_keyword() && rendered_args.is_empty() {
            Ok(def.name().to_owned())
        } else {
            Ok(format!("{}({})", def.name(), rendered_args.join(", ")))
        }
    }

    // ── Structural ───────────────────────────────────────────────────────────

    /// Render a type cast: `CAST(inner AS type)`.
    ///
    /// Default: `CAST(inner AS <DataType Display>)`.
    fn render_cast(
        &mut self,
        inner:   &str,
        as_type: &DataType,
    ) -> Result<String, BackendError> {
        Ok(format!("CAST({} AS {})", inner, as_type))
    }

    /// Render a `CASE` expression.
    ///
    /// Default: SQL-standard `CASE WHEN … THEN … ELSE … END`.
    fn render_case(
        &mut self,
        whens:     &[(String, String)],
        else_expr: Option<&str>,
    ) -> Result<String, BackendError> {
        let mut sql = String::from("CASE");
        for (cond, then) in whens {
            sql.push_str(&format!(" WHEN {} THEN {}", cond, then));
        }
        if let Some(e) = else_expr {
            sql.push_str(&format!(" ELSE {}", e));
        }
        sql.push_str(" END");
        Ok(sql)
    }

    /// Render `expr BETWEEN low AND high`.
    ///
    /// Default: `lhs BETWEEN low AND high`.
    ///
    /// For `NOT BETWEEN`, the caller wraps in `UnaryOp::Not` before rendering.
    fn render_between(
        &mut self,
        lhs:  &str,
        low:  &str,
        high: &str,
    ) -> Result<String, BackendError> {
        Ok(format!("{} BETWEEN {} AND {}", lhs, low, high))
    }

    /// Render `expr IN (list)`.
    ///
    /// Default: `lhs IN (a, b, c)`.
    fn render_in_list(
        &mut self,
        lhs:  &str,
        list: &[String],
    ) -> Result<String, BackendError> {
        Ok(format!("{} IN ({})", lhs, list.join(", ")))
    }

    /// Render `expr NOT IN (list)`.
    ///
    /// Default: `lhs NOT IN (a, b, c)`.
    fn render_not_in_list(
        &mut self,
        lhs:  &str,
        list: &[String],
    ) -> Result<String, BackendError> {
        Ok(format!("{} NOT IN ({})", lhs, list.join(", ")))
    }

    /// Render `expr NOT BETWEEN low AND high`.
    ///
    /// Default: `lhs NOT BETWEEN low AND high`.
    fn render_not_between(
        &mut self,
        lhs:  &str,
        low:  &str,
        high: &str,
    ) -> Result<String, BackendError> {
        Ok(format!("{} NOT BETWEEN {} AND {}", lhs, low, high))
    }

    // ── Decoration ───────────────────────────────────────────────────────────

    /// Render `expr AS alias`.
    ///
    /// Default: `inner AS alias`.
    fn render_alias(
        &mut self,
        inner: &str,
        alias: &str,
    ) -> Result<String, BackendError> {
        Ok(format!("{} AS {}", inner, alias))
    }

    /// Render `*` (all fields).
    ///
    /// Default: `*`.
    fn render_star(&mut self) -> Result<String, BackendError> {
        Ok("*".into())
    }

    /// Render `COUNT(*)`.
    ///
    /// Default: `COUNT(*)`.
    fn render_count_star(&mut self) -> Result<String, BackendError> {
        Ok("COUNT(*)".into())
    }

    /// Render a window function: `func OVER (…)`.
    ///
    /// Default: SQL-standard `func OVER (PARTITION BY … ORDER BY … frame)`.
    fn render_window(
        &mut self,
        func:         &str,
        partition_by: &[String],
        order_by:     &[String],
        frame:        Option<&WindowFrame>,
    ) -> Result<String, BackendError> {
        let mut parts = Vec::new();
        if !partition_by.is_empty() {
            parts.push(format!("PARTITION BY {}", partition_by.join(", ")));
        }
        if !order_by.is_empty() {
            parts.push(format!("ORDER BY {}", order_by.join(", ")));
        }
        if let Some(f) = frame {
            parts.push(render_window_frame(f));
        }
        Ok(format!("{} OVER ({})", func, parts.join(" ")))
    }

    /// Render an ORDER BY expression (used within window functions).
    ///
    /// Default: `expr ASC/DESC`.
    fn render_order_by_expr(
        &mut self,
        expr:  &OrderByExpr<'_>,
        depth: usize,
    ) -> Result<String, BackendError> {
        let rendered = compile_expr(self, &expr.expr, depth)?;
        let dir = match expr.direction {
            super::Direction::Asc  => "ASC",
            super::Direction::Desc => "DESC",
        };
        Ok(format!("{} {}", rendered, dir))
    }
}

/// Compile an expression tree into a string by delegating to the given renderer.
///
/// Returns `Err(BackendError::Render)` if expression nesting exceeds
/// [`MAX_EXPR_DEPTH`].
pub fn compile_expr<R: ExprRenderer + ?Sized>(
    renderer: &mut R,
    expr:     &Expr<'_>,
    depth:    usize,
) -> Result<String, BackendError> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(BackendError::Render(
            "expression nesting too deep (exceeded MAX_EXPR_DEPTH)".into(),
        ));
    }
    let next = depth + 1;

    match expr {
        Expr::Ref(path) => {
            let segs: Vec<&str> = path.iter().collect();
            renderer.render_ref(&segs)
        }

        Expr::Access { base, path } => {
            let base_str = compile_expr(renderer, base, next)?;
            let segs: Vec<&str> = path.iter().collect();
            renderer.render_access(&base_str, &segs)
        }

        Expr::Param => renderer.render_param(),

        Expr::Value(lit) => renderer.render_literal(lit),

        Expr::Array(elements) => {
            let items: Vec<String> = elements
                .iter()
                .map(|e| compile_expr(renderer, e, next))
                .collect::<Result<_, _>>()?;
            renderer.render_array_literal(&items)
        }

        Expr::Object(fields) => {
            let pairs: Vec<(&str, String)> = fields
                .iter()
                .map(|(k, v)| {
                    let val = compile_expr(renderer, v, next)?;
                    Ok((k.as_str(), val))
                })
                .collect::<Result<_, BackendError>>()?;
            renderer.render_object_literal(&pairs)
        }

        Expr::BinaryOp { left, op, right } => {
            let lhs = compile_expr(renderer, left,  next)?;
            let rhs = compile_expr(renderer, right, next)?;
            renderer.render_binary_op(&lhs, op, &rhs)
        }

        Expr::UnaryOp { op, expr: inner } => {
            // Special-case NOT(InList) → NOT IN and NOT(Between) → NOT BETWEEN
            // for canonical SQL form.
            if *op == UnaryOp::Not {
                match inner.as_ref() {
                    Expr::InList { expr, list } => {
                        let lhs = compile_expr(renderer, expr, next)?;
                        let items = list
                            .iter()
                            .map(|e| compile_expr(renderer, e, next))
                            .collect::<Result<Vec<_>, _>>()?;
                        return renderer.render_not_in_list(&lhs, &items);
                    }
                    Expr::Between { expr, low, high } => {
                        let lhs = compile_expr(renderer, expr, next)?;
                        let lo  = compile_expr(renderer, low,  next)?;
                        let hi  = compile_expr(renderer, high, next)?;
                        return renderer.render_not_between(&lhs, &lo, &hi);
                    }
                    _ => {}
                }
            }
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_unary_op(*op, &inner_str)
        }

        Expr::Func { name, args } => {
            if let Err(e) = name.validate_arity(args.len()) {
                return Err(BackendError::Validation(e.to_string()));
            }
            let rendered_args: Vec<String> = args
                .iter()
                .map(|a| compile_expr(renderer, a, next))
                .collect::<Result<_, _>>()?;
            renderer.render_func(name, &rendered_args)
        }

        Expr::Cast { expr: inner, as_type } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_cast(&inner_str, as_type)
        }

        Expr::Case { whens, else_expr } => {
            let rendered_whens: Vec<(String, String)> = whens
                .iter()
                .map(|(cond, then)| {
                    let c = compile_expr(renderer, cond, next)?;
                    let t = compile_expr(renderer, then, next)?;
                    Ok((c, t))
                })
                .collect::<Result<_, BackendError>>()?;
            let rendered_else = match else_expr {
                Some(e) => Some(compile_expr(renderer, e, next)?),
                None    => None,
            };
            renderer.render_case(&rendered_whens, rendered_else.as_deref())
        }

        Expr::Between { expr, low, high } => {
            let lhs  = compile_expr(renderer, expr, next)?;
            let lo   = compile_expr(renderer, low,  next)?;
            let hi   = compile_expr(renderer, high, next)?;
            renderer.render_between(&lhs, &lo, &hi)
        }

        Expr::InList { expr, list } => {
            let lhs   = compile_expr(renderer, expr, next)?;
            let items: Vec<String> = list
                .iter()
                .map(|e| compile_expr(renderer, e, next))
                .collect::<Result<_, _>>()?;
            renderer.render_in_list(&lhs, &items)
        }

        Expr::Alias { expr: inner, alias } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_alias(&inner_str, alias.as_str())
        }

        Expr::Star      => renderer.render_star(),
        Expr::CountStar => renderer.render_count_star(),

        Expr::Window {
            func,
            partition_by,
            order_by,
            frame,
        } => {
            let func_str = compile_expr(renderer, func, next)?;
            let part_strs: Vec<String> = partition_by
                .iter()
                .map(|e| compile_expr(renderer, e, next))
                .collect::<Result<_, _>>()?;
            let order_strs: Vec<String> = order_by
                .iter()
                .map(|ob| renderer.render_order_by_expr(ob, next))
                .collect::<Result<_, _>>()?;
            renderer.render_window(&func_str, &part_strs, &order_strs, frame.as_ref())
        }
    }
}

// ── Window frame helpers ─────────────────────────────────────────────────────

/// Render a SQL-standard window frame clause.
pub fn render_window_frame(frame: &WindowFrame) -> String {
    let kind = match frame.kind {
        FrameKind::Rows  => "ROWS",
        FrameKind::Range => "RANGE",
    };
    let start = render_frame_bound(&frame.start);
    match &frame.end {
        Some(end) => format!("{} BETWEEN {} AND {}", kind, start, render_frame_bound(end)),
        None      => format!("{} {}", kind, start),
    }
}

fn render_frame_bound(bound: &FrameBound) -> String {
    match bound {
        FrameBound::UnboundedPreceding => "UNBOUNDED PRECEDING".to_string(),
        FrameBound::Preceding(n)       => format!("{} PRECEDING", n),
        FrameBound::CurrentRow         => "CURRENT ROW".to_string(),
        FrameBound::Following(n)       => format!("{} FOLLOWING", n),
        FrameBound::UnboundedFollowing => "UNBOUNDED FOLLOWING".to_string(),
    }
}

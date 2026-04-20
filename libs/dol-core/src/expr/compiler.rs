//! Generic expression compiler — delegates rendering to a backend [`ExprRenderer`].
//!
//! The compiler walks the expression tree and handles:
//! - Recursive tree traversal with depth limiting
//! - Arity validation for function calls
//! - Delegation to the backend for all rendering decisions
//!
//! Backends implement [`ExprRenderer`] to produce their output format.

use super::window::{FrameBound, FrameKind, WindowFrame};
use super::{Expr, FuncDef, Literal, OpDef, OrderByExpr, Quantifier, UnaryOp};
use crate::op::BackendError;
use crate::types::DataType;

/// Maximum nesting depth for expression rendering.
pub const MAX_EXPR_DEPTH: usize = 128;

/// A backend must implement this trait to render expressions.
///
/// The compiler calls these methods for each expression variant.
/// Each method receives already-rendered sub-expressions as strings.
///
/// # Example
///
/// ```ignore
/// struct MyRenderer;
///
/// impl ExprRenderer for MyRenderer {
///     fn render_identifier(&mut self, name: &str) -> Result<String, BackendError> { ... }
///     // ... implement all methods
/// }
/// ```
pub trait ExprRenderer {
    /// Render a bare identifier (field reference).
    ///
    /// Default: returns the name unchanged.
    fn render_identifier(&mut self, name: &str) -> Result<String, BackendError> {
        Ok(name.to_owned())
    }

    /// Render a qualified identifier: `scope.name`.
    ///
    /// Default: `scope.name`.
    fn render_qualified_identifier(
        &mut self,
        scope: &str,
        name: &str,
    ) -> Result<String, BackendError> {
        Ok(format!("{}.{}", scope, name))
    }

    /// Render field access: `base.field` (e.g., JSON access).
    ///
    /// Default: `base.field`.
    fn render_field_access(&mut self, base: &str, field: &str) -> Result<String, BackendError> {
        Ok(format!("{}.{}", base, field))
    }

    /// Render a bind parameter placeholder.
    ///
    /// Default: `?`.
    fn render_param(&mut self) -> Result<String, BackendError> {
        Ok("?".into())
    }

    /// Render a literal value.
    ///
    /// **Required** — no sensible universal default; escaping and formatting
    /// rules vary between backends.
    fn render_literal(&mut self, lit: &Literal<'_>) -> Result<String, BackendError>;

    /// Render a binary operation: `lhs op rhs`, optionally negated.
    ///
    /// **Required** — operator-to-token mapping is backend-specific.
    fn render_binary_op(
        &mut self,
        lhs: &str,
        op: &OpDef,
        rhs: &str,
        negated: bool,
    ) -> Result<String, BackendError>;

    /// Render a unary operation: `op expr`.
    ///
    /// Default: SQL-standard `NOT (expr)` / `-(expr)` / `~(expr)`.
    fn render_unary_op(&mut self, op: UnaryOp, inner: &str) -> Result<String, BackendError> {
        Ok(match op {
            UnaryOp::Not => format!("NOT ({})", inner),
            UnaryOp::Neg => format!("-({})", inner),
            UnaryOp::BitNot => format!("~({})", inner),
        })
    }

    /// Render a quantified comparison: `expr op ANY/ALL (subquery)`.
    ///
    /// **Required** — operator-to-token mapping is backend-specific.
    fn render_quantified_cmp(
        &mut self,
        lhs: &str,
        op: &OpDef,
        quantifier: Quantifier,
        subquery: &str,
    ) -> Result<String, BackendError>;

    /// Render a function call: `name(args...)`.
    ///
    /// Default: `NAME(arg1, arg2, …)`. No-parens keywords
    /// (e.g. `CURRENT_DATE`) are emitted without parentheses.
    fn render_func(
        &mut self,
        def: &FuncDef,
        rendered_args: &[String],
    ) -> Result<String, BackendError> {
        if def.is_no_parens_keyword() && rendered_args.is_empty() {
            Ok(def.name().to_owned())
        } else {
            Ok(format!("{}({})", def.name(), rendered_args.join(", ")))
        }
    }

    /// Render a type cast: `CAST(inner AS type)`.
    ///
    /// Default: `CAST(inner AS <DataType Display>)`.
    fn render_cast(&mut self, inner: &str, as_type: &DataType) -> Result<String, BackendError> {
        Ok(format!("CAST({} AS {})", inner, as_type))
    }

    /// Render a CASE expression.
    ///
    /// Default: SQL-standard `CASE WHEN … THEN … ELSE … END`.
    fn render_case(
        &mut self,
        whens: &[(String, String)],
        else_expr: Option<&str>,
    ) -> Result<String, BackendError> {
        let mut sql = String::from("CASE");
        for (cond, then) in whens {
            sql.push_str(&format!(" WHEN {} THEN {}", cond, then));
        }
        if let Some(else_val) = else_expr {
            sql.push_str(&format!(" ELSE {}", else_val));
        }
        sql.push_str(" END");
        Ok(sql)
    }

    /// Render a subquery: `(SELECT ...)`.
    ///
    /// Default: wraps in parentheses.
    fn render_subquery(&mut self, sql: &str) -> Result<String, BackendError> {
        Ok(format!("({})", sql))
    }

    /// Render `expr [NOT] IN (list)`.
    ///
    /// Default: `lhs [NOT] IN (a, b, c)`.
    fn render_in_list(
        &mut self,
        lhs: &str,
        list: &[String],
        negated: bool,
    ) -> Result<String, BackendError> {
        let not = if negated { " NOT" } else { "" };
        Ok(format!("{}{} IN ({})", lhs, not, list.join(", ")))
    }

    /// Render `expr [NOT] IN (subquery)`.
    ///
    /// Default: `lhs [NOT] IN (subquery)`.
    fn render_in_subquery(
        &mut self,
        lhs: &str,
        subquery: &str,
        negated: bool,
    ) -> Result<String, BackendError> {
        let not = if negated { " NOT" } else { "" };
        Ok(format!("{}{} IN ({})", lhs, not, subquery))
    }

    /// Render `expr [NOT] BETWEEN low AND high`.
    ///
    /// Default: `lhs [NOT] BETWEEN low AND high`.
    fn render_between(
        &mut self,
        lhs: &str,
        low: &str,
        high: &str,
        negated: bool,
    ) -> Result<String, BackendError> {
        let not = if negated { " NOT" } else { "" };
        Ok(format!("{}{} BETWEEN {} AND {}", lhs, not, low, high))
    }

    /// Render `[NOT] EXISTS (subquery)`.
    ///
    /// Default: `[NOT] EXISTS (subquery)`.
    fn render_exists(&mut self, subquery: &str, negated: bool) -> Result<String, BackendError> {
        let not = if negated { "NOT " } else { "" };
        Ok(format!("{}EXISTS ({})", not, subquery))
    }

    /// Render `expr IS [NOT] NULL`.
    ///
    /// Default: `inner IS [NOT] NULL`.
    fn render_is_null(&mut self, inner: &str, negated: bool) -> Result<String, BackendError> {
        Ok(if negated {
            format!("{} IS NOT NULL", inner)
        } else {
            format!("{} IS NULL", inner)
        })
    }

    /// Render an object literal: `{ key: value, ... }`.
    ///
    /// **Required** — representation is highly backend-specific.
    fn render_object_literal(&mut self, pairs: &[(String, String)])
    -> Result<String, BackendError>;

    /// Render an array literal: `[elem, ...]`.
    ///
    /// **Required** — representation is highly backend-specific.
    fn render_array_literal(&mut self, elements: &[String]) -> Result<String, BackendError>;

    /// Render a raw expression string (escape hatch).
    ///
    /// Default: returns the string unchanged.
    fn render_raw(&mut self, sql: &str) -> Result<String, BackendError> {
        Ok(sql.to_owned())
    }

    /// Render `expr AS alias`.
    ///
    /// Default: `inner AS alias`.
    fn render_alias(&mut self, inner: &str, alias: &str) -> Result<String, BackendError> {
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

    /// Render a window function: `func OVER (...)`.
    ///
    /// Default: SQL-standard `func OVER (PARTITION BY … ORDER BY … frame)`.
    fn render_window(
        &mut self,
        func: &str,
        partition_by: &[String],
        order_by: &[String],
        frame: Option<&WindowFrame>,
    ) -> Result<String, BackendError> {
        let mut over_parts = Vec::new();

        if !partition_by.is_empty() {
            over_parts.push(format!("PARTITION BY {}", partition_by.join(", ")));
        }
        if !order_by.is_empty() {
            over_parts.push(format!("ORDER BY {}", order_by.join(", ")));
        }
        if let Some(f) = frame {
            over_parts.push(render_window_frame(f));
        }
        Ok(format!("{} OVER ({})", func, over_parts.join(" ")))
    }

    /// Render an ORDER BY expression (used within window functions).
    ///
    /// Default: `expr ASC/DESC`.
    fn render_order_by_expr(
        &mut self,
        expr: &OrderByExpr<'_>,
        depth: usize,
    ) -> Result<String, BackendError> {
        let rendered = compile_expr(self, &expr.expr, depth)?;
        let dir = match expr.direction {
            super::Direction::Asc => "ASC",
            super::Direction::Desc => "DESC",
        };
        Ok(format!("{} {}", rendered, dir))
    }
}

/// Compile an expression tree into a string by delegating to the given renderer.
///
/// This is the generic compiler that walks the `Expr` tree. The renderer handles
/// all backend-specific formatting decisions.
///
/// Returns `Err(BackendError::Render)` if expression nesting exceeds [`MAX_EXPR_DEPTH`].
pub fn compile_expr<R: ExprRenderer + ?Sized>(
    renderer: &mut R,
    expr: &Expr<'_>,
    depth: usize,
) -> Result<String, BackendError> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(BackendError::Render(
            "expression nesting too deep (exceeded MAX_EXPR_DEPTH)".into(),
        ));
    }
    let next = depth + 1;

    match expr {
        Expr::Identifier(name) => renderer.render_identifier(name),

        Expr::QualifiedIdentifier { scope, name } => {
            renderer.render_qualified_identifier(scope, name)
        }

        Expr::FieldAccess { base, field } => {
            let base_str = compile_expr(renderer, base, next)?;
            renderer.render_field_access(&base_str, field)
        }

        Expr::Param => renderer.render_param(),

        Expr::Value(lit) => renderer.render_literal(lit),

        Expr::BinaryOp {
            left,
            op,
            right,
            negated,
        } => {
            let lhs = compile_expr(renderer, left, next)?;
            let rhs = compile_expr(renderer, right, next)?;
            renderer.render_binary_op(&lhs, op, &rhs, *negated)
        }

        Expr::UnaryOp { op, expr: inner } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_unary_op(*op, &inner_str)
        }

        Expr::QuantifiedCmp {
            expr: inner,
            op,
            quantifier,
            subquery,
        } => {
            let lhs = compile_expr(renderer, inner, next)?;
            renderer.render_quantified_cmp(&lhs, op, *quantifier, subquery)
        }

        Expr::Func { name, args } => {
            // Validate arity for well-known functions
            if let Err(e) = name.validate_arity(args.len()) {
                return Err(BackendError::Validation(e.to_string()));
            }
            let rendered_args: Vec<String> = args
                .iter()
                .map(|a| compile_expr(renderer, a, next))
                .collect::<Result<Vec<_>, _>>()?;
            renderer.render_func(name, &rendered_args)
        }

        Expr::Cast {
            expr: inner,
            as_type,
        } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_cast(&inner_str, as_type)
        }

        Expr::Case { whens, else_expr } => {
            let rendered_whens: Vec<(String, String)> = whens
                .iter()
                .map(|(cond, then)| {
                    let cond_str = compile_expr(renderer, cond, next)?;
                    let then_str = compile_expr(renderer, then, next)?;
                    Ok((cond_str, then_str))
                })
                .collect::<Result<Vec<_>, BackendError>>()?;
            let rendered_else = match else_expr {
                Some(e) => Some(compile_expr(renderer, e, next)?),
                None => None,
            };
            renderer.render_case(&rendered_whens, rendered_else.as_deref())
        }

        Expr::Subquery(sql) => renderer.render_subquery(sql),

        Expr::InList {
            expr: inner,
            list,
            negated,
        } => {
            let lhs = compile_expr(renderer, inner, next)?;
            let items: Vec<String> = list
                .iter()
                .map(|e| compile_expr(renderer, e, next))
                .collect::<Result<Vec<_>, _>>()?;
            renderer.render_in_list(&lhs, &items, *negated)
        }

        Expr::InSubquery {
            expr: inner,
            subquery,
            negated,
        } => {
            let lhs = compile_expr(renderer, inner, next)?;
            renderer.render_in_subquery(&lhs, subquery, *negated)
        }

        Expr::Between {
            expr: inner,
            low,
            high,
            negated,
        } => {
            let lhs = compile_expr(renderer, inner, next)?;
            let low_str = compile_expr(renderer, low, next)?;
            let high_str = compile_expr(renderer, high, next)?;
            renderer.render_between(&lhs, &low_str, &high_str, *negated)
        }

        Expr::Exists { subquery, negated } => renderer.render_exists(subquery, *negated),

        Expr::IsNull {
            expr: inner,
            negated,
        } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_is_null(&inner_str, *negated)
        }

        Expr::ObjectLiteral(fields) => {
            let pairs: Vec<(String, String)> = fields
                .iter()
                .map(|(k, v)| {
                    let val = compile_expr(renderer, v, next)?;
                    Ok((k.clone(), val))
                })
                .collect::<Result<Vec<_>, BackendError>>()?;
            renderer.render_object_literal(&pairs)
        }

        Expr::ArrayLiteral(elements) => {
            let items: Vec<String> = elements
                .iter()
                .map(|e| compile_expr(renderer, e, next))
                .collect::<Result<Vec<_>, _>>()?;
            renderer.render_array_literal(&items)
        }

        Expr::Raw(sql) => renderer.render_raw(sql),

        Expr::Alias { expr: inner, alias } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_alias(&inner_str, alias)
        }

        Expr::Star => renderer.render_star(),

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
                .collect::<Result<Vec<_>, _>>()?;
            let order_strs: Vec<String> = order_by
                .iter()
                .map(|ob| renderer.render_order_by_expr(ob, next))
                .collect::<Result<Vec<_>, _>>()?;
            renderer.render_window(&func_str, &part_strs, &order_strs, frame.as_ref())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Window frame helpers (used by the default `render_window` implementation)
// ═══════════════════════════════════════════════════════════════════════════

/// Render a SQL-standard window frame clause.
pub fn render_window_frame(frame: &WindowFrame) -> String {
    let kind = match frame.kind {
        FrameKind::Rows => "ROWS",
        FrameKind::Range => "RANGE",
    };
    let start = render_frame_bound(&frame.start);
    if let Some(ref end) = frame.end {
        format!("{} BETWEEN {} AND {}", kind, start, render_frame_bound(end))
    } else {
        format!("{} {}", kind, start)
    }
}

fn render_frame_bound(bound: &FrameBound) -> String {
    match bound {
        FrameBound::UnboundedPreceding => "UNBOUNDED PRECEDING".to_string(),
        FrameBound::Preceding(n) => format!("{} PRECEDING", n),
        FrameBound::CurrentRow => "CURRENT ROW".to_string(),
        FrameBound::Following(n) => format!("{} FOLLOWING", n),
        FrameBound::UnboundedFollowing => "UNBOUNDED FOLLOWING".to_string(),
    }
}

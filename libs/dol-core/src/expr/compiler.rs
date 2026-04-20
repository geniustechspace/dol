//! Generic expression compiler — delegates rendering to a backend [`ExprRenderer`].
//!
//! The compiler walks the expression tree and handles:
//! - Recursive tree traversal with depth limiting
//! - Arity validation for function calls
//! - Delegation to the backend for all rendering decisions
//!
//! Backends implement [`ExprRenderer`] to produce their output format.

use super::{
    Expr, FuncDef, Literal, OpDef, OrderByExpr, Quantifier, UnaryOp,
};
use super::window::WindowFrame;
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
    fn render_identifier(&mut self, name: &str) -> Result<String, BackendError>;

    /// Render a qualified identifier: `scope.name`.
    fn render_qualified_identifier(&mut self, scope: &str, name: &str) -> Result<String, BackendError>;

    /// Render field access: `base.field` (e.g., JSON access).
    fn render_field_access(&mut self, base: &str, field: &str) -> Result<String, BackendError>;

    /// Render a bind parameter placeholder.
    fn render_param(&mut self) -> Result<String, BackendError>;

    /// Render a literal value.
    fn render_literal(&mut self, lit: &Literal<'_>) -> Result<String, BackendError>;

    /// Render a binary operation: `lhs op rhs`, optionally negated.
    fn render_binary_op(
        &mut self,
        lhs: &str,
        op: &OpDef,
        rhs: &str,
        negated: bool,
    ) -> Result<String, BackendError>;

    /// Render a unary operation: `op expr`.
    fn render_unary_op(&mut self, op: UnaryOp, inner: &str) -> Result<String, BackendError>;

    /// Render a quantified comparison: `expr op ANY/ALL (subquery)`.
    fn render_quantified_cmp(
        &mut self,
        lhs: &str,
        op: &OpDef,
        quantifier: Quantifier,
        subquery: &str,
    ) -> Result<String, BackendError>;

    /// Render a function call: `name(args...)`.
    fn render_func(&mut self, def: &FuncDef, rendered_args: &[String]) -> Result<String, BackendError>;

    /// Render a type cast: `CAST(inner AS type)`.
    fn render_cast(&mut self, inner: &str, as_type: &DataType) -> Result<String, BackendError>;

    /// Render a CASE expression.
    fn render_case(
        &mut self,
        whens: &[(String, String)],
        else_expr: Option<&str>,
    ) -> Result<String, BackendError>;

    /// Render a subquery: `(SELECT ...)`.
    fn render_subquery(&mut self, sql: &str) -> Result<String, BackendError>;

    /// Render `expr [NOT] IN (list)`.
    fn render_in_list(
        &mut self,
        lhs: &str,
        list: &[String],
        negated: bool,
    ) -> Result<String, BackendError>;

    /// Render `expr [NOT] IN (subquery)`.
    fn render_in_subquery(
        &mut self,
        lhs: &str,
        subquery: &str,
        negated: bool,
    ) -> Result<String, BackendError>;

    /// Render `expr [NOT] BETWEEN low AND high`.
    fn render_between(
        &mut self,
        lhs: &str,
        low: &str,
        high: &str,
        negated: bool,
    ) -> Result<String, BackendError>;

    /// Render `[NOT] EXISTS (subquery)`.
    fn render_exists(&mut self, subquery: &str, negated: bool) -> Result<String, BackendError>;

    /// Render `expr IS [NOT] NULL`.
    fn render_is_null(&mut self, inner: &str, negated: bool) -> Result<String, BackendError>;

    /// Render an object literal: `{ key: value, ... }`.
    fn render_object_literal(&mut self, pairs: &[(String, String)]) -> Result<String, BackendError>;

    /// Render an array literal: `[elem, ...]`.
    fn render_array_literal(&mut self, elements: &[String]) -> Result<String, BackendError>;

    /// Render a raw expression string (escape hatch).
    fn render_raw(&mut self, sql: &str) -> Result<String, BackendError>;

    /// Render `expr AS alias`.
    fn render_alias(&mut self, inner: &str, alias: &str) -> Result<String, BackendError>;

    /// Render `*` (all fields).
    fn render_star(&mut self) -> Result<String, BackendError>;

    /// Render `COUNT(*)`.
    fn render_count_star(&mut self) -> Result<String, BackendError>;

    /// Render a window function: `func OVER (...)`.
    fn render_window(
        &mut self,
        func: &str,
        partition_by: &[String],
        order_by: &[String],
        frame: Option<&WindowFrame>,
    ) -> Result<String, BackendError>;

    /// Render an ORDER BY expression (used within window functions).
    fn render_order_by_expr(&mut self, expr: &OrderByExpr<'_>, depth: usize) -> Result<String, BackendError> {
        let rendered = compile_expr(self, &expr.expr, depth)?;
        let dir = match expr.direction {
            super::Direction::Asc => "ASC",
            super::Direction::Desc => "DESC",
        };
        // Default implementation — backends can override for nulls ordering etc.
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

        Expr::BinaryOp { left, op, right, negated } => {
            let lhs = compile_expr(renderer, left, next)?;
            let rhs = compile_expr(renderer, right, next)?;
            renderer.render_binary_op(&lhs, op, &rhs, *negated)
        }

        Expr::UnaryOp { op, expr: inner } => {
            let inner_str = compile_expr(renderer, inner, next)?;
            renderer.render_unary_op(*op, &inner_str)
        }

        Expr::QuantifiedCmp { expr: inner, op, quantifier, subquery } => {
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

        Expr::Cast { expr: inner, as_type } => {
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

        Expr::InList { expr: inner, list, negated } => {
            let lhs = compile_expr(renderer, inner, next)?;
            let items: Vec<String> = list
                .iter()
                .map(|e| compile_expr(renderer, e, next))
                .collect::<Result<Vec<_>, _>>()?;
            renderer.render_in_list(&lhs, &items, *negated)
        }

        Expr::InSubquery { expr: inner, subquery, negated } => {
            let lhs = compile_expr(renderer, inner, next)?;
            renderer.render_in_subquery(&lhs, subquery, *negated)
        }

        Expr::Between { expr: inner, low, high, negated } => {
            let lhs = compile_expr(renderer, inner, next)?;
            let low_str = compile_expr(renderer, low, next)?;
            let high_str = compile_expr(renderer, high, next)?;
            renderer.render_between(&lhs, &low_str, &high_str, *negated)
        }

        Expr::Exists { subquery, negated } => {
            renderer.render_exists(subquery, *negated)
        }

        Expr::IsNull { expr: inner, negated } => {
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

        Expr::Window { func, partition_by, order_by, frame } => {
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

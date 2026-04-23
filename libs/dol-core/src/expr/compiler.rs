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

// ── Iterative arena renderer ──────────────────────────────────────────────────

/// Render a lowered expression tree using an iterative post-order scan.
///
/// Because [`ExprArena::lower`] inserts nodes in post-order (children at
/// smaller indices than their parents), a single forward pass over nodes
/// `0..=root.0` is sufficient: when processing node `i`, all children have
/// already been rendered and their strings are available in `rendered[child.0]`.
///
/// This replaces the recursive `compile_expr` path for arena-lowered
/// expressions.  The [`ExprRenderer`] trait interface is identical — backends
/// work without modification.
///
/// [`ExprArena::lower`]: super::arena::ExprArena::lower
pub fn render_arena<R: ExprRenderer + ?Sized>(
    renderer: &mut R,
    arena:    &crate::expr::arena::ExprArena,
    interner: &crate::expr::interner::Interner,
    root:     crate::expr::node::NodeId,
) -> Result<String, BackendError> {
    use crate::expr::node::NodeId;

    let limit = root.0 as usize + 1;
    let mut rendered: Vec<Option<String>> = vec![None; limit];

    for idx in 0..limit {
        let id  = NodeId(idx as u32);
        let s = render_arena_node(renderer, arena, interner, id, &mut rendered)?;
        rendered[idx] = Some(s);
    }

    rendered[root.0 as usize]
        .take()
        .ok_or_else(|| BackendError::Render("root node not rendered".into()))
}

/// Render a single arena node whose children are already in `rendered`.
fn render_arena_node<R: ExprRenderer + ?Sized>(
    renderer: &mut R,
    arena:    &crate::expr::arena::ExprArena,
    interner: &crate::expr::interner::Interner,
    id:       crate::expr::node::NodeId,
    rendered: &mut Vec<Option<String>>,
) -> Result<String, BackendError> {
    use crate::expr::node::{ExprNode, NodeId};
    use super::order::Direction;

    #[inline(always)]
    fn get<'r>(rendered: &'r [Option<String>], child: NodeId) -> &'r str {
        rendered[child.0 as usize]
            .as_deref()
            .expect("arena child rendered before parent (post-order guarantee)")
    }

    let node = arena.get(id);
    match node {
        ExprNode::Ref(path_ids) => {
            let segs: Vec<&str> = path_ids.iter().map(|&s| interner.get(s)).collect();
            renderer.render_ref(&segs)
        }

        ExprNode::Access { base, path } => {
            let (base_id, path_ids) = (*base, path.clone());
            let base_str = get(rendered, base_id).to_owned();
            let segs: Vec<&str> = path_ids.iter().map(|&s| interner.get(s)).collect();
            renderer.render_access(&base_str, &segs)
        }

        ExprNode::Param => renderer.render_param(),

        ExprNode::Value(lit) => renderer.render_literal(lit),

        ExprNode::Array(child_ids) => {
            let ids = child_ids.clone();
            let items: Vec<String> = ids.iter().map(|&c| get(rendered, c).to_owned()).collect();
            renderer.render_array_literal(&items)
        }

        ExprNode::Object(pairs) => {
            let pairs = pairs.clone();
            let kv: Vec<(&str, String)> = pairs
                .iter()
                .map(|(kid, vid)| (interner.get(*kid), get(rendered, *vid).to_owned()))
                .collect();
            renderer.render_object_literal(&kv)
        }

        ExprNode::BinaryOp { left, op, right } => {
            let (l, o, r_id) = (*left, *op, *right);
            let lhs = get(rendered, l).to_owned();
            let rhs = get(rendered, r_id).to_owned();
            renderer.render_binary_op(&lhs, arena.op(o), &rhs)
        }

        ExprNode::UnaryOp { op, expr } => {
            let (op, inner_id) = (*op, *expr);
            let inner = get(rendered, inner_id).to_owned();
            renderer.render_unary_op(op, &inner)
        }

        ExprNode::Func(func_node) => {
            let (func_id, arg_ids) = (func_node.id, func_node.args.clone());
            let def = arena.func(func_id);
            if let Err(e) = def.validate_arity(arg_ids.len()) {
                return Err(BackendError::Validation(e.to_string()));
            }
            let args: Vec<String> = arg_ids.iter().map(|&a| get(rendered, a).to_owned()).collect();
            renderer.render_func(def, &args)
        }

        ExprNode::Cast(cast_node) => {
            let (expr_id, as_type) = (cast_node.expr, cast_node.as_type.clone());
            let inner = get(rendered, expr_id).to_owned();
            renderer.render_cast(&inner, &as_type)
        }

        ExprNode::Case(case_node) => {
            let (when_ids, else_id) = (case_node.whens.clone(), case_node.else_expr);
            let whens: Vec<(String, String)> = when_ids
                .iter()
                .map(|(c, t)| (get(rendered, *c).to_owned(), get(rendered, *t).to_owned()))
                .collect();
            let else_str = else_id.map(|e| get(rendered, e).to_owned());
            renderer.render_case(&whens, else_str.as_deref())
        }

        ExprNode::Between { expr, low, high } => {
            let (e, lo, hi) = (*expr, *low, *high);
            let lhs  = get(rendered, e).to_owned();
            let lo_s = get(rendered, lo).to_owned();
            let hi_s = get(rendered, hi).to_owned();
            renderer.render_between(&lhs, &lo_s, &hi_s)
        }

        ExprNode::NotBetween { expr, low, high } => {
            let (e, lo, hi) = (*expr, *low, *high);
            let lhs  = get(rendered, e).to_owned();
            let lo_s = get(rendered, lo).to_owned();
            let hi_s = get(rendered, hi).to_owned();
            renderer.render_not_between(&lhs, &lo_s, &hi_s)
        }

        ExprNode::InList { expr, list } => {
            let (e, list_ids) = (*expr, list.clone());
            let lhs   = get(rendered, e).to_owned();
            let items: Vec<String> = list_ids.iter().map(|&c| get(rendered, c).to_owned()).collect();
            renderer.render_in_list(&lhs, &items)
        }

        ExprNode::NotInList { expr, list } => {
            let (e, list_ids) = (*expr, list.clone());
            let lhs   = get(rendered, e).to_owned();
            let items: Vec<String> = list_ids.iter().map(|&c| get(rendered, c).to_owned()).collect();
            renderer.render_not_in_list(&lhs, &items)
        }

        ExprNode::Alias { expr, alias } => {
            let (expr_id, alias_id) = (*expr, *alias);
            let inner = get(rendered, expr_id).to_owned();
            let alias_str = interner.get(alias_id);
            renderer.render_alias(&inner, alias_str)
        }

        ExprNode::Star      => renderer.render_star(),
        ExprNode::CountStar => renderer.render_count_star(),

        ExprNode::Window(win) => {
            let win = win.clone();
            let func_str = get(rendered, win.func).to_owned();
            let part: Vec<String> = win
                .partition_by
                .iter()
                .map(|&e| get(rendered, e).to_owned())
                .collect();
            let ord: Vec<String> = win
                .order_by
                .iter()
                .map(|ob| {
                    let expr_s = get(rendered, ob.expr);
                    let dir = match ob.direction {
                        Direction::Asc  => "ASC",
                        Direction::Desc => "DESC",
                    };
                    format!("{} {}", expr_s, dir)
                })
                .collect();
            renderer.render_window(&func_str, &part, &ord, win.frame.as_ref())
        }
    }
}

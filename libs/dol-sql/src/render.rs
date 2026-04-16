//! SQL rendering engine — turns DOL IR and Expr trees into SQL strings.
//!
//! This module ports and extends the rendering logic from core-db.

use super::dialect::{
    ArrayLiteralStyle, ConcatStyle, Dialect, JsonAccessStyle, PaginationStyle, ParamCounter,
    ReturningStyle,
};
use dol_core::expr::window::{FrameBound, FrameKind, WindowFrame};
use dol_core::expr::{
    BinOp, Direction, Expr, Literal, NullsPosition, OrderByExpr, Quantifier, TernaryOp, UnaryOp,
};
use dol_core::ir::BackendError;
use dol_core::ir::SqlOutput;
use dol_core::ir::definition::OwnedEntityConstraint;
use dol_core::ir::*;
use dol_core::model::Field;
use dol_core::model::FieldType;
use dol_core::model::constraint::{FkAction, GeneratedKind};

// ===========================================================================
// Expr rendering (the core recursive renderer)
// ===========================================================================

/// Renders an [`Expr`] tree into a SQL string.
pub fn render_expr(expr: &Expr, counter: &mut ParamCounter, dialect: &Dialect) -> String {
    match expr {
        Expr::Identifier(name) => name.clone(),

        Expr::QualifiedIdentifier { scope, name } => {
            format!("{}.{}", scope, name)
        }

        Expr::FieldAccess { base, field } => {
            let base_sql = render_expr(base, counter, dialect);
            match &dialect.json_access {
                JsonAccessStyle::ArrowOperator => format!("{}->>'{}'", base_sql, field),
                JsonAccessStyle::JsonExtractFunction => {
                    format!("json_extract({}, '$.{}')", base_sql, field)
                }
                JsonAccessStyle::JsonValueFunction => {
                    format!("JSON_VALUE({}, '$.{}')", base_sql, field)
                }
                JsonAccessStyle::Unsupported => format!("{}.{}", base_sql, field),
            }
        }

        Expr::Param => counter.next(),

        Expr::Literal(lit) => render_literal(lit, dialect),

        Expr::BinaryOp {
            left,
            op,
            right,
            negated,
        } => render_binary_op(left, *op, right, *negated, counter, dialect),

        Expr::UnaryOp { op, expr } => {
            let inner = render_expr(expr, counter, dialect);
            match op {
                UnaryOp::Not => format!("NOT ({})", inner),
                UnaryOp::Neg => format!("-({})", inner),
                UnaryOp::IsNull => format!("{} IS NULL", inner),
                UnaryOp::IsNotNull => format!("{} IS NOT NULL", inner),
                UnaryOp::IsTrue => format!("{} IS TRUE", inner),
                UnaryOp::IsNotTrue => format!("{} IS NOT TRUE", inner),
                UnaryOp::IsFalse => format!("{} IS FALSE", inner),
                UnaryOp::IsNotFalse => format!("{} IS NOT FALSE", inner),
                UnaryOp::IsUnknown => format!("{} IS UNKNOWN", inner),
                UnaryOp::IsNotUnknown => format!("{} IS NOT UNKNOWN", inner),
                UnaryOp::BitNot => format!("~({})", inner),
                UnaryOp::Sqrt => format!("|/({})", inner),
                UnaryOp::CubeRoot => format!("||/({})", inner),
                UnaryOp::Abs => format!("@({})", inner),
                UnaryOp::Factorial => format!("({}!)", inner),
            }
        }

        Expr::Func { name, args } => {
            let rendered_args: Vec<_> = args
                .iter()
                .map(|a| render_expr(a, counter, dialect))
                .collect();
            format!("{}({})", name, rendered_args.join(", "))
        }

        Expr::Cast { expr, as_type } => {
            let inner = render_expr(expr, counter, dialect);
            format!("CAST({} AS {})", inner, as_type)
        }

        Expr::Case { whens, else_expr } => {
            let mut sql = String::from("CASE");
            for (cond, then) in whens {
                let cond_sql = render_expr(cond, counter, dialect);
                let then_sql = render_expr(then, counter, dialect);
                sql.push_str(&format!(" WHEN {} THEN {}", cond_sql, then_sql));
            }
            if let Some(else_val) = else_expr {
                let else_sql = render_expr(else_val, counter, dialect);
                sql.push_str(&format!(" ELSE {}", else_sql));
            }
            sql.push_str(" END");
            sql
        }

        Expr::Subquery(sql) => format!("({})", sql),

        Expr::InList {
            expr,
            list,
            negated,
        } => {
            let lhs = render_expr(expr, counter, dialect);
            let items: Vec<_> = list
                .iter()
                .map(|e| render_expr(e, counter, dialect))
                .collect();
            let not = if *negated { " NOT" } else { "" };
            format!("{}{} IN ({})", lhs, not, items.join(", "))
        }

        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let lhs = render_expr(expr, counter, dialect);
            let low_sql = render_expr(low, counter, dialect);
            let high_sql = render_expr(high, counter, dialect);
            let not = if *negated { " NOT" } else { "" };
            format!("{}{} BETWEEN {} AND {}", lhs, not, low_sql, high_sql)
        }

        Expr::Exists { subquery, negated } => {
            let not = if *negated { "NOT " } else { "" };
            format!("{}EXISTS ({})", not, subquery)
        }

        Expr::IsNull { expr, negated } => {
            let inner = render_expr(expr, counter, dialect);
            if *negated {
                format!("{} IS NOT NULL", inner)
            } else {
                format!("{} IS NULL", inner)
            }
        }

        Expr::InSubquery {
            expr,
            subquery,
            negated,
        } => {
            let lhs = render_expr(expr, counter, dialect);
            let not = if *negated { " NOT" } else { "" };
            format!("{}{} IN ({})", lhs, not, subquery)
        }

        Expr::ObjectLiteral(fields) => {
            let pairs: Vec<_> = fields
                .iter()
                .map(|(k, v)| {
                    let val = render_expr(v, counter, dialect);
                    format!("'{}', {}", k, val)
                })
                .collect();
            match &dialect.json_access {
                JsonAccessStyle::ArrowOperator => {
                    format!("jsonb_build_object({})", pairs.join(", "))
                }
                _ => format!("json_object({})", pairs.join(", ")),
            }
        }

        Expr::ArrayLiteral(elements) => {
            let items: Vec<_> = elements
                .iter()
                .map(|e| render_expr(e, counter, dialect))
                .collect();
            match &dialect.array_literal_style {
                ArrayLiteralStyle::ArrayKeyword => format!("ARRAY[{}]", items.join(", ")),
                ArrayLiteralStyle::JsonArrayFunction => {
                    format!("JSON_ARRAY({})", items.join(", "))
                }
                ArrayLiteralStyle::Unsupported => format!("({})", items.join(", ")),
            }
        }

        Expr::Raw(sql) => sql.clone(),

        Expr::Alias { expr, alias } => {
            let inner = render_expr(expr, counter, dialect);
            format!("{} AS {}", inner, alias)
        }

        Expr::Star => "*".to_string(),

        Expr::CountStar => "COUNT(*)".to_string(),

        Expr::Window {
            func,
            partition_by,
            order_by,
            frame,
        } => {
            let func_sql = render_expr(func, counter, dialect);
            let mut over_parts = Vec::new();

            if !partition_by.is_empty() {
                let parts: Vec<_> = partition_by
                    .iter()
                    .map(|e| render_expr(e, counter, dialect))
                    .collect();
                over_parts.push(format!("PARTITION BY {}", parts.join(", ")));
            }

            if !order_by.is_empty() {
                let parts: Vec<_> = order_by
                    .iter()
                    .map(|ob| render_order_by_expr(ob, counter, dialect))
                    .collect();
                over_parts.push(format!("ORDER BY {}", parts.join(", ")));
            }

            if let Some(f) = frame {
                over_parts.push(render_window_frame(f));
            }

            format!("{} OVER ({})", func_sql, over_parts.join(" "))
        }

        Expr::TernaryOp {
            expr,
            op,
            first,
            second,
            negated,
        } => {
            let expr_sql = render_expr(expr, counter, dialect);
            let first_sql = render_expr(first, counter, dialect);
            let second_sql = render_expr(second, counter, dialect);
            let not = if *negated { " NOT" } else { "" };
            match op {
                TernaryOp::Between => {
                    format!(
                        "{}{} BETWEEN {} AND {}",
                        expr_sql, not, first_sql, second_sql
                    )
                }
            }
        }

        Expr::QuantifiedCmp {
            expr,
            op,
            quantifier,
            subquery,
        } => {
            let lhs = render_expr(expr, counter, dialect);
            let op_str = render_binop_token(*op);
            let quant = match quantifier {
                Quantifier::Any => "ANY",
                Quantifier::All => "ALL",
            };
            format!("{} {} {} ({})", lhs, op_str, quant, subquery)
        }
    }
}

fn render_literal(lit: &Literal, dialect: &Dialect) -> String {
    match lit {
        Literal::String(s) => format!("'{}'", s),
        Literal::Int(n) => n.to_string(),
        Literal::Float(f) => f.to_string(),
        Literal::Bool(true) => dialect.bool_true.clone(),
        Literal::Bool(false) => dialect.bool_false.clone(),
        Literal::Null => "NULL".to_string(),
        Literal::Uuid(u) => format!("'{}'", u),
        Literal::Bytes(_) => "?bytes?".to_string(), // placeholder
        Literal::Decimal(d) => d.clone(),
    }
}

fn render_binary_op(
    left: &Expr,
    op: BinOp,
    right: &Expr,
    negated: bool,
    counter: &mut ParamCounter,
    dialect: &Dialect,
) -> String {
    // Special case: ILike on dialects without native ILIKE support
    if op == BinOp::ILike && !dialect.features.ilike {
        let lhs = render_expr(left, counter, dialect);
        let rhs = render_expr(right, counter, dialect);
        let not = if negated { "NOT " } else { "" };
        return format!("{}LOWER({}) LIKE LOWER({})", not, lhs, rhs);
    }

    // Special case: Concat dispatches on dialect.concat_style
    if op == BinOp::Concat {
        let lhs = render_expr(left, counter, dialect);
        let rhs = render_expr(right, counter, dialect);
        return match &dialect.concat_style {
            ConcatStyle::PipeOperator => format!("{} || {}", lhs, rhs),
            ConcatStyle::ConcatFunction => format!("CONCAT({}, {})", lhs, rhs),
            ConcatStyle::PlusOperator => format!("{} + {}", lhs, rhs),
        };
    }

    let lhs = render_expr(left, counter, dialect);
    let rhs = render_expr(right, counter, dialect);
    let op_str = render_binop_token(op);

    let base = match op {
        BinOp::And | BinOp::Or => format!("({} {} {})", lhs, op_str, rhs),
        _ => format!("{} {} {}", lhs, op_str, rhs),
    };

    if negated {
        format!("NOT ({})", base)
    } else {
        base
    }
}

/// Map a [`BinOp`] to its SQL token string.
fn render_binop_token(op: BinOp) -> &'static str {
    match op {
        // Comparison
        BinOp::Eq => "=",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        // Null-safe comparison
        BinOp::IsDistinctFrom => "IS DISTINCT FROM",
        BinOp::IsNotDistinctFrom => "IS NOT DISTINCT FROM",
        // Arithmetic
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        // Logical
        BinOp::And => "AND",
        BinOp::Or => "OR",
        // Pattern
        BinOp::Like => "LIKE",
        BinOp::ILike => "ILIKE",
        BinOp::SimilarTo => "SIMILAR TO",
        BinOp::RegexMatch => "~",
        BinOp::RegexMatchInsensitive => "~*",
        BinOp::Glob => "GLOB",
        // String
        BinOp::Concat => "||",
        BinOp::StartsWith => "^@",
        BinOp::Contains => "LIKE", // lowered by backend
        // Bitwise
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "#",
        BinOp::ShiftLeft => "<<",
        BinOp::ShiftRight => ">>",
        // Array / Collection
        BinOp::ArrayContains => "@>",
        BinOp::ArrayContainedBy => "<@",
        BinOp::ArrayOverlap => "&&",
        // JSON / Document
        BinOp::JsonGet => "->",
        BinOp::JsonGetText => "->>",
        BinOp::JsonPath => "#>",
        BinOp::JsonPathText => "#>>",
        BinOp::JsonHasKey => "?",
        BinOp::JsonHasAnyKey => "?|",
        BinOp::JsonHasAllKeys => "?&",
        // Range
        BinOp::RangeContains => "@>",
        BinOp::RangeContainedBy => "<@",
        BinOp::RangeOverlap => "&&",
        // Collection ops (value-level)
        BinOp::Merge => "MERGE",
        BinOp::Append => "APPEND",
        BinOp::Prepend => "PREPEND",
        BinOp::RemoveKey => "REMOVE",
        // Spatial
        BinOp::Distance => "<->",
    }
}

/// Renders an [`OrderByExpr`] into SQL.
pub fn render_order_by_expr(
    ob: &OrderByExpr,
    counter: &mut ParamCounter,
    dialect: &Dialect,
) -> String {
    let expr_sql = render_expr(&ob.expr, counter, dialect);
    let dir = match ob.direction {
        Direction::Asc => "ASC",
        Direction::Desc => "DESC",
    };
    match (&ob.nulls, dialect.features.nulls_ordering) {
        (Some(NullsPosition::First), true) => format!("{} {} NULLS FIRST", expr_sql, dir),
        (Some(NullsPosition::Last), true) => format!("{} {} NULLS LAST", expr_sql, dir),
        (Some(_), false) | (None, _) => format!("{} {}", expr_sql, dir),
    }
}

fn render_window_frame(frame: &WindowFrame) -> String {
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

// ===========================================================================
// Helper renderers
// ===========================================================================

/// Renders a `Vec<Expr>` as a WHERE clause (AND-joined).
pub fn render_filters(filters: &[Expr], counter: &mut ParamCounter, dialect: &Dialect) -> String {
    if filters.is_empty() {
        return String::new();
    }
    let parts: Vec<_> = filters
        .iter()
        .map(|f| render_expr(f, counter, dialect))
        .collect();
    format!(" WHERE {}", parts.join(" AND "))
}

/// Renders a `Vec<OrderByExpr>` as an ORDER BY clause.
pub fn render_order_by_exprs(
    order_by: &[OrderByExpr],
    counter: &mut ParamCounter,
    dialect: &Dialect,
) -> String {
    if order_by.is_empty() {
        return String::new();
    }
    let parts: Vec<_> = order_by
        .iter()
        .map(|ob| render_order_by_expr(ob, counter, dialect))
        .collect();
    format!(" ORDER BY {}", parts.join(", "))
}

/// Renders a RETURNING clause per dialect style.
pub fn render_returning(columns: &[String], dialect: &Dialect) -> String {
    if columns.is_empty() {
        return String::new();
    }
    match &dialect.returning_style {
        ReturningStyle::Returning => {
            format!(" RETURNING {}", columns.join(", "))
        }
        ReturningStyle::OutputInserted => {
            let cols: Vec<_> = columns.iter().map(|c| format!("INSERTED.{}", c)).collect();
            format!(" OUTPUT {}", cols.join(", "))
        }
        ReturningStyle::ReturningInto => {
            format!(" RETURNING {}", columns.join(", "))
        }
        ReturningStyle::Unsupported => String::new(),
    }
}

/// Renders pagination clauses per dialect style.
pub fn render_pagination(
    offset: &Option<OffsetLimit>,
    limit: &Option<OffsetLimit>,
    counter: &mut ParamCounter,
    dialect: &Dialect,
) -> String {
    if offset.is_none() && limit.is_none() {
        return String::new();
    }
    match &dialect.pagination {
        PaginationStyle::LimitOffset => {
            let mut sql = String::new();
            if let Some(ol) = offset {
                let val = render_offset_limit(ol, counter);
                sql.push_str(&format!(" OFFSET {}", val));
            }
            if let Some(ol) = limit {
                let val = render_offset_limit(ol, counter);
                sql.push_str(&format!(" LIMIT {}", val));
            }
            sql
        }
        PaginationStyle::OffsetFetch => {
            let mut sql = String::new();
            if let Some(ol) = offset {
                let val = render_offset_limit(ol, counter);
                sql.push_str(&format!(" OFFSET {} ROWS", val));
            } else {
                sql.push_str(" OFFSET 0 ROWS");
            }
            if let Some(ol) = limit {
                let val = render_offset_limit(ol, counter);
                sql.push_str(&format!(" FETCH NEXT {} ROWS ONLY", val));
            }
            sql
        }
        PaginationStyle::Rownum => String::new(),
    }
}

fn render_offset_limit(ol: &OffsetLimit, counter: &mut ParamCounter) -> String {
    match ol {
        OffsetLimit::Param => counter.next(),
        OffsetLimit::Value(v) => v.to_string(),
    }
}

/// Renders a row-level locking clause per dialect.
pub fn render_lock_mode(mode: &LockMode, dialect: &Dialect) -> String {
    if dialect.locking.use_table_hint {
        return String::new();
    }
    match mode {
        LockMode::ForUpdate if dialect.locking.for_update => " FOR UPDATE".into(),
        LockMode::ForShare if dialect.locking.for_share => " FOR SHARE".into(),
        LockMode::ForUpdateNoWait if dialect.locking.for_update && dialect.locking.nowait => {
            " FOR UPDATE NOWAIT".into()
        }
        LockMode::ForUpdateSkipLocked
            if dialect.locking.for_update && dialect.locking.skip_locked =>
        {
            " FOR UPDATE SKIP LOCKED".into()
        }
        LockMode::ForShareNoWait if dialect.locking.for_share && dialect.locking.nowait => {
            " FOR SHARE NOWAIT".into()
        }
        LockMode::ForShareSkipLocked
            if dialect.locking.for_share && dialect.locking.skip_locked =>
        {
            " FOR SHARE SKIP LOCKED".into()
        }
        _ => String::new(),
    }
}

/// Renders a field definition using the dialect's type map.
pub fn render_field_def(field: &Field, dialect: &Dialect) -> String {
    let type_str = dialect.type_map.resolve(&field.field_type);
    let mut def = format!("{} {}", field.name, type_str);

    if let Some(collation) = field.collation {
        def.push_str(&format!(" COLLATE {}", collation));
    }

    if !field.nullable {
        def.push_str(" NOT NULL");
    }

    if let Some(expr) = field.default_expr {
        def.push_str(&format!(" DEFAULT {}", expr));
    }

    if field.unique {
        def.push_str(" UNIQUE");
    }

    if let Some(ref fk) = field.references {
        def.push_str(&format!(" REFERENCES {}({})", fk.table, fk.column));
        if fk.on_delete != FkAction::NoAction {
            def.push_str(&format!(" ON DELETE {}", fk.on_delete));
        }
        if fk.on_update != FkAction::NoAction {
            def.push_str(&format!(" ON UPDATE {}", fk.on_update));
        }
    }

    if let Some(check_expr) = field.check {
        def.push_str(&format!(" CHECK ({})", check_expr));
    }

    if let Some((kind, expr)) = &field.generated {
        let stored = match kind {
            GeneratedKind::Stored => "STORED",
            GeneratedKind::Virtual => "VIRTUAL",
        };
        def.push_str(&format!(" GENERATED ALWAYS AS ({}) {}", expr, stored));
    }

    def
}

/// Renders a field definition from an owned FieldDef (IR).
pub fn render_field_def_ir(fd: &dol_core::ir::definition::FieldDef, dialect: &Dialect) -> String {
    let type_str = dialect.type_map.resolve(&fd.field_type);
    let mut def = format!("{} {}", fd.name, type_str);

    if let Some(ref collation) = fd.collation {
        def.push_str(&format!(" COLLATE {}", collation));
    }

    if !fd.nullable {
        def.push_str(" NOT NULL");
    }

    if let Some(ref expr) = fd.default_expr {
        def.push_str(&format!(" DEFAULT {}", expr));
    }

    if fd.unique {
        def.push_str(" UNIQUE");
    }

    if let Some(ref fk) = fd.references {
        def.push_str(&format!(" REFERENCES {}({})", fk.table, fk.column));
        if fk.on_delete != FkAction::NoAction {
            def.push_str(&format!(" ON DELETE {}", fk.on_delete));
        }
        if fk.on_update != FkAction::NoAction {
            def.push_str(&format!(" ON UPDATE {}", fk.on_update));
        }
    }

    if let Some(ref check_expr) = fd.check {
        def.push_str(&format!(" CHECK ({})", check_expr));
    }

    if let Some((kind, ref expr)) = fd.generated {
        let stored = match kind {
            GeneratedKind::Stored => "STORED",
            GeneratedKind::Virtual => "VIRTUAL",
        };
        def.push_str(&format!(" GENERATED ALWAYS AS ({}) {}", expr, stored));
    }

    def
}

/// Renders a FieldType using the dialect's type map.
pub fn render_type(field_type: &FieldType, dialect: &Dialect) -> String {
    dialect.type_map.resolve(field_type)
}

/// Renders a model-level constraint.
pub fn render_model_constraint(constraint: &OwnedEntityConstraint) -> String {
    match constraint {
        OwnedEntityConstraint::Unique(cols) => {
            format!("UNIQUE ({})", cols.join(", "))
        }
        OwnedEntityConstraint::ForeignKey {
            columns,
            ref_table,
            ref_columns,
            on_delete,
        } => {
            let mut sql = format!(
                "FOREIGN KEY ({}) REFERENCES {} ({})",
                columns.join(", "),
                ref_table,
                ref_columns.join(", ")
            );
            if *on_delete != FkAction::NoAction {
                sql.push_str(&format!(" ON DELETE {}", on_delete));
            }
            sql
        }
        OwnedEntityConstraint::Check(expr) => {
            format!("CHECK ({})", expr)
        }
        OwnedEntityConstraint::PrimaryKey(cols) => {
            format!("PRIMARY KEY ({})", cols.join(", "))
        }
    }
}

// ===========================================================================
// IR renderers — each turns an IR struct into SqlOutput
// ===========================================================================

/// Render a QueryIR to SQL.
pub fn render_query_ir(ir: &QueryIR, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = ParamCounter::new(&dialect.param_style);
    render_query_ir_with_counter(ir, dialect, &mut counter)
}

/// Render a QueryIR to SQL using an existing `ParamCounter`.
///
/// This allows compound queries to share a single counter so parameter
/// numbers are globally unique across all sub-queries.
pub(crate) fn render_query_ir_with_counter(
    ir: &QueryIR,
    dialect: &Dialect,
    counter: &mut ParamCounter,
) -> Result<SqlOutput, BackendError> {
    let mut sql = String::new();

    // SELECT
    sql.push_str("SELECT ");
    if ir.distinct {
        if !ir.distinct_on.is_empty() && dialect.features.distinct_on {
            sql.push_str(&format!("DISTINCT ON ({}) ", ir.distinct_on.join(", ")));
        } else {
            sql.push_str("DISTINCT ");
        }
    }

    // Projections
    if ir.projections.is_empty() {
        sql.push('*');
    } else {
        let projs: Vec<_> = ir
            .projections
            .iter()
            .map(|e| render_expr(e, counter, dialect))
            .collect();
        sql.push_str(&projs.join(", "));
    }

    // FROM
    let table_name = entity_ref_to_sql(&ir.source, dialect);
    sql.push_str(&format!(" FROM {}", table_name));

    // JOINs
    for join in &ir.joins {
        let join_type = match join.join_type {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL OUTER JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        let target = entity_ref_to_sql(&join.target, dialect);
        sql.push_str(&format!(" {} {}", join_type, target));
        if !join.on_conditions.is_empty() {
            let conds: Vec<_> = join
                .on_conditions
                .iter()
                .map(|(l, r)| format!("{} = {}", l, r))
                .collect();
            sql.push_str(&format!(" ON {}", conds.join(" AND ")));
        }
    }

    // WHERE
    sql.push_str(&render_filters(&ir.filters, counter, dialect));

    // GROUP BY
    if !ir.group_by.is_empty() {
        let groups: Vec<_> = ir
            .group_by
            .iter()
            .map(|e| render_expr(e, counter, dialect))
            .collect();
        sql.push_str(&format!(" GROUP BY {}", groups.join(", ")));
    }

    // HAVING
    if !ir.having.is_empty() {
        let havings: Vec<_> = ir
            .having
            .iter()
            .map(|e| render_expr(e, counter, dialect))
            .collect();
        sql.push_str(&format!(" HAVING {}", havings.join(" AND ")));
    }

    // ORDER BY
    sql.push_str(&render_order_by_exprs(&ir.order_by, counter, dialect));

    // OFFSET / LIMIT
    sql.push_str(&render_pagination(&ir.offset, &ir.limit, counter, dialect));

    // Locking
    if let Some(ref lock) = ir.lock_mode {
        sql.push_str(&render_lock_mode(lock, dialect));
    }

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render an InsertIR to SQL.
pub fn render_insert_ir(ir: &InsertIR, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = ParamCounter::new(&dialect.param_style);
    let table_name = entity_ref_to_sql(&ir.target, dialect);

    let cols = ir.fields.join(", ");
    let mut all_values = Vec::new();
    for _ in 0..ir.row_count {
        let row_params: Vec<_> = ir.fields.iter().map(|_| counter.next()).collect();
        all_values.push(format!("({})", row_params.join(", ")));
    }

    let mut sql = format!(
        "INSERT INTO {} ({}) VALUES {}",
        table_name,
        cols,
        all_values.join(", ")
    );

    sql.push_str(&render_returning(&ir.returning, dialect));

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render an InsertSelectIR to SQL.
pub fn render_insert_select_ir(
    ir: &InsertSelectIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let table_name = entity_ref_to_sql(&ir.target, dialect);
    let cols = ir.fields.join(", ");

    let mut sql = format!("INSERT INTO {} ({}) {}", table_name, cols, ir.source_query,);

    // InsertSelect doesn't consume params itself (the source_query has its own)
    let counter = ParamCounter::new(&dialect.param_style);
    sql.push_str(&render_returning(&ir.returning, dialect));

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render an UpdateIR to SQL.
pub fn render_update_ir(ir: &UpdateIR, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = ParamCounter::new(&dialect.param_style);
    let table_name = entity_ref_to_sql(&ir.target, dialect);

    let sets: Vec<_> = ir
        .assignments
        .iter()
        .map(|(col, expr)| {
            let val = render_expr(expr, &mut counter, dialect);
            format!("{} = {}", col, val)
        })
        .collect();

    let mut sql = format!("UPDATE {} SET {}", table_name, sets.join(", "));
    sql.push_str(&render_filters(&ir.filters, &mut counter, dialect));
    sql.push_str(&render_returning(&ir.returning, dialect));

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render a RemoveIR to SQL.
pub fn render_remove_ir(ir: &RemoveIR, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = ParamCounter::new(&dialect.param_style);
    let table_name = entity_ref_to_sql(&ir.target, dialect);

    let mut sql = format!("DELETE FROM {}", table_name);
    sql.push_str(&render_filters(&ir.filters, &mut counter, dialect));
    sql.push_str(&render_returning(&ir.returning, dialect));

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render an UpsertIR to SQL.
pub fn render_upsert_ir(ir: &UpsertIR, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = ParamCounter::new(&dialect.param_style);
    let table_name = entity_ref_to_sql(&ir.target, dialect);

    let cols = ir.fields.join(", ");
    let params: Vec<_> = ir.fields.iter().map(|_| counter.next()).collect();

    let mut sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table_name,
        cols,
        params.join(", ")
    );

    // ON CONFLICT
    if let Some(ref constraint) = ir.conflict_constraint {
        sql.push_str(&format!(" ON CONFLICT ON CONSTRAINT {}", constraint));
    } else if !ir.conflict_fields.is_empty() {
        sql.push_str(&format!(" ON CONFLICT ({})", ir.conflict_fields.join(", ")));
    }

    if ir.do_nothing {
        sql.push_str(" DO NOTHING");
    } else if !ir.update_fields.is_empty() {
        let sets: Vec<_> = ir
            .update_fields
            .iter()
            .map(|f| format!("{} = EXCLUDED.{}", f, f))
            .collect();
        sql.push_str(&format!(" DO UPDATE SET {}", sets.join(", ")));

        // Conflict filters
        if !ir.conflict_filters.is_empty() {
            let filters: Vec<_> = ir
                .conflict_filters
                .iter()
                .map(|f| render_expr(f, &mut counter, dialect))
                .collect();
            sql.push_str(&format!(" WHERE {}", filters.join(" AND ")));
        }
    }

    sql.push_str(&render_returning(&ir.returning, dialect));

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render a DefineEntityIR to SQL (CREATE TABLE).
pub fn render_define_entity_ir(
    ir: &DefineEntityIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("CREATE TABLE ");
    if ir.if_not_exists {
        sql.push_str("IF NOT EXISTS ");
    }

    let name = if let Some(ref ns) = ir.namespace {
        format!("{}.{}", ns, ir.name)
    } else {
        ir.name.clone()
    };
    sql.push_str(&name);
    sql.push_str(" (\n");

    let mut parts = Vec::new();
    for fd in &ir.fields {
        parts.push(format!("  {}", render_field_def_ir(fd, dialect)));
    }
    for constraint in &ir.constraints {
        parts.push(format!("  {}", render_model_constraint(constraint)));
    }

    sql.push_str(&parts.join(",\n"));
    sql.push_str("\n)");

    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

/// Render an AlterEntityIR to SQL (ALTER TABLE).
pub fn render_alter_entity_ir(
    ir: &AlterEntityIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let table_name = entity_ref_to_sql(&ir.target, dialect);
    let mut parts = Vec::new();

    for action in &ir.actions {
        let part = match action {
            AlterAction::AddField(fd) => {
                format!(
                    "ALTER TABLE {} ADD COLUMN {}",
                    table_name,
                    render_field_def_ir(fd, dialect)
                )
            }
            AlterAction::DropField(name) => {
                format!("ALTER TABLE {} DROP COLUMN {}", table_name, name)
            }
            AlterAction::RenameField { from, to } => {
                format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {}",
                    table_name, from, to
                )
            }
            AlterAction::AlterFieldType { name, new_type } => {
                let type_str = render_type(new_type, dialect);
                format!(
                    "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
                    table_name, name, type_str
                )
            }
            AlterAction::SetFieldDefault { name, expr } => {
                format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {}",
                    table_name, name, expr
                )
            }
            AlterAction::DropFieldDefault(name) => {
                format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT",
                    table_name, name
                )
            }
            AlterAction::SetFieldNotNull(name) => {
                format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL",
                    table_name, name
                )
            }
            AlterAction::DropFieldNotNull(name) => {
                format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL",
                    table_name, name
                )
            }
            AlterAction::AddConstraint(c) => {
                format!(
                    "ALTER TABLE {} ADD {}",
                    table_name,
                    render_model_constraint(c)
                )
            }
            AlterAction::DropConstraint(name) => {
                format!("ALTER TABLE {} DROP CONSTRAINT {}", table_name, name)
            }
            AlterAction::RenameEntity(new_name) => {
                format!("ALTER TABLE {} RENAME TO {}", table_name, new_name)
            }
        };
        parts.push(part);
    }

    Ok(SqlOutput {
        sql: parts.join(";\n"),
        param_count: 0,
    })
}

/// Render a DropEntityIR to SQL (DROP TABLE).
pub fn render_drop_entity_ir(
    ir: &DropEntityIR,
    _dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("DROP TABLE ");
    if ir.if_exists {
        sql.push_str("IF EXISTS ");
    }
    let name = if let Some(ref ns) = ir.target.namespace {
        format!("{}.{}", ns, ir.target.name)
    } else {
        ir.target.name.clone()
    };
    sql.push_str(&name);
    if ir.cascade {
        sql.push_str(" CASCADE");
    }

    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

/// Render a DefineIndexIR to SQL (CREATE INDEX).
pub fn render_define_index_ir(
    ir: &DefineIndexIR,
    _dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("CREATE ");
    if ir.unique {
        sql.push_str("UNIQUE ");
    }
    sql.push_str("INDEX ");
    if ir.concurrently {
        sql.push_str("CONCURRENTLY ");
    }
    if ir.if_not_exists {
        sql.push_str("IF NOT EXISTS ");
    }
    sql.push_str(&ir.name);
    sql.push_str(&format!(" ON {}", ir.target.name));

    if let Some(ref method) = ir.method {
        let m = match method {
            IndexMethod::BTree => "btree",
            IndexMethod::Hash => "hash",
            IndexMethod::Gin => "gin",
            IndexMethod::Gist => "gist",
            IndexMethod::SpGist => "spgist",
            IndexMethod::Brin => "brin",
        };
        sql.push_str(&format!(" USING {}", m));
    }

    sql.push_str(&format!(" ({})", ir.columns.join(", ")));

    if let Some(ref wc) = ir.where_clause {
        sql.push_str(&format!(" WHERE {}", wc));
    }

    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

/// Render a DropIndexIR to SQL.
pub fn render_drop_index_ir(
    ir: &DropIndexIR,
    _dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("DROP INDEX ");
    if ir.concurrently {
        sql.push_str("CONCURRENTLY ");
    }
    if ir.if_exists {
        sql.push_str("IF EXISTS ");
    }
    sql.push_str(&ir.name);
    if ir.cascade {
        sql.push_str(" CASCADE");
    }

    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

/// Render a GrantIR to SQL.
pub fn render_grant_ir(ir: &GrantIR) -> Result<SqlOutput, BackendError> {
    let priv_str = render_privilege(&ir.privilege);
    let sql = format!("GRANT {} ON {} TO {}", priv_str, ir.on_target, ir.to_role);
    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

/// Render a RevokeIR to SQL.
pub fn render_revoke_ir(ir: &RevokeIR) -> Result<SqlOutput, BackendError> {
    let priv_str = render_privilege(&ir.privilege);
    let sql = format!(
        "REVOKE {} ON {} FROM {}",
        priv_str, ir.on_target, ir.from_role
    );
    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

fn render_privilege(p: &Privilege) -> String {
    match p {
        Privilege::Select => "SELECT".into(),
        Privilege::Insert => "INSERT".into(),
        Privilege::Update => "UPDATE".into(),
        Privilege::Delete => "DELETE".into(),
        Privilege::All => "ALL".into(),
        Privilege::Usage => "USAGE".into(),
        Privilege::Create => "CREATE".into(),
        Privilege::Connect => "CONNECT".into(),
        Privilege::Custom(s) => s.clone(),
    }
}

/// Render a TransactionIR to SQL.
pub fn render_transaction_ir(
    ir: &TransactionIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    match ir {
        TransactionIR::Begin => Ok(SqlOutput {
            sql: "BEGIN".to_string(),
            param_count: 0,
        }),
        TransactionIR::Commit => Ok(SqlOutput {
            sql: "COMMIT".to_string(),
            param_count: 0,
        }),
        TransactionIR::Rollback => Ok(SqlOutput {
            sql: "ROLLBACK".to_string(),
            param_count: 0,
        }),
        TransactionIR::Savepoint(name) => Ok(SqlOutput {
            sql: format!("SAVEPOINT {}", name),
            param_count: 0,
        }),
        TransactionIR::ReleaseSavepoint(name) => Ok(SqlOutput {
            sql: format!("RELEASE SAVEPOINT {}", name),
            param_count: 0,
        }),
        TransactionIR::RollbackToSavepoint(name) => Ok(SqlOutput {
            sql: format!("ROLLBACK TO SAVEPOINT {}", name),
            param_count: 0,
        }),
        TransactionIR::Block(stmts) => render_transaction_block(stmts, dialect),
    }
}

/// Render a transaction block: `BEGIN; stmt1; stmt2; ...; COMMIT`.
///
/// Each inner statement is rendered using the full SQL backend, and the block
/// is wrapped in `BEGIN` / `COMMIT`.
fn render_transaction_block(
    stmts: &[Statement],
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let backend = crate::SqlBackend::new(dialect.clone());
    let mut parts = Vec::with_capacity(stmts.len() + 2);
    let mut total_params = 0;

    parts.push("BEGIN".to_string());

    for stmt in stmts {
        let output = backend.render(stmt)?;
        match output {
            RenderedOutput::Sql(sql_out) => {
                total_params += sql_out.param_count;
                parts.push(sql_out.sql);
            }
            _ => {
                return Err(BackendError::RenderError(
                    "transaction block contains non-SQL statement".into(),
                ));
            }
        }
    }

    parts.push("COMMIT".to_string());

    Ok(SqlOutput {
        sql: parts.join(";\n"),
        param_count: total_params,
    })
}

// ===========================================================================
// DefineType rendering (CREATE TYPE ... AS ENUM)
// ===========================================================================

/// Render a [`DefineTypeIR`] to SQL.
///
/// Dialect-aware:
/// - **PostgreSQL / CockroachDB** (`EnumStyle::CreateType`):
///   `CREATE TYPE name AS ENUM ('a', 'b', 'c')`
/// - **MySQL / MariaDB** (`EnumStyle::InlineEnum`):
///   Not a standalone statement — returns an informational comment.
///   (Inline ENUMs are rendered at the column level in CREATE TABLE.)
/// - **SQLite / MSSQL / Oracle** (`EnumStyle::CheckConstraint`):
///   Not a standalone statement — returns an informational comment.
///   (CHECK constraints are rendered at the column level in CREATE TABLE.)
pub fn render_define_type_ir(
    ir: &DefineTypeIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    use super::dialect::ddl::EnumStyle;

    let qualified_name = if let Some(ref ns) = ir.namespace {
        format!("{}.{}", ns, ir.name)
    } else {
        ir.name.clone()
    };

    let sql = match dialect.ddl.enum_style {
        EnumStyle::CreateType => {
            let variants: Vec<String> = ir.variants.iter().map(|v| format!("'{}'", v)).collect();
            format!(
                "CREATE TYPE {} AS ENUM ({})",
                qualified_name,
                variants.join(", ")
            )
        }
        EnumStyle::InlineEnum => {
            // MySQL inline ENUMs are part of column definitions, not standalone.
            // Return a comment so callers know this is intentionally a no-op.
            format!(
                "-- type {} is rendered inline as ENUM({}) in column definitions",
                qualified_name,
                ir.variants
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        EnumStyle::CheckConstraint => {
            // SQLite/MSSQL/Oracle use CHECK constraints instead of types.
            format!(
                "-- type {} is enforced via CHECK (col IN ({})) in column definitions",
                qualified_name,
                ir.variants
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    };

    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

// ===========================================================================
// DropType rendering (DROP TYPE)
// ===========================================================================

/// Render a [`DropTypeIR`] to SQL.
///
/// Only meaningful for dialects with `EnumStyle::CreateType` (PostgreSQL,
/// CockroachDB). For other dialects, returns a comment.
pub fn render_drop_type_ir(ir: &DropTypeIR, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    use super::dialect::ddl::EnumStyle;

    let sql = match dialect.ddl.enum_style {
        EnumStyle::CreateType => {
            let mut s = String::from("DROP TYPE ");
            if ir.if_exists {
                s.push_str("IF EXISTS ");
            }
            s.push_str(&ir.name);
            s
        }
        _ => {
            // Inline enums / check constraints don't have a separate type to drop.
            format!(
                "-- type {} does not exist as a standalone object in this dialect",
                ir.name
            )
        }
    };

    Ok(SqlOutput {
        sql,
        param_count: 0,
    })
}

// ===========================================================================
// DefinePolicy rendering (CREATE POLICY ... ON ... FOR ... USING ... WITH CHECK)
// ===========================================================================

/// Render a [`DefinePolicyIR`] to SQL.
///
/// Produces PostgreSQL-style row-level security policy:
/// ```sql
/// CREATE POLICY name ON table
///   FOR ALL
///   USING (tenant_id = $1)
///   WITH CHECK (tenant_id = $1)
/// ```
pub fn render_define_policy_ir(
    ir: &DefinePolicyIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();

    let action_str = match ir.action {
        dol_core::ir::control::PolicyAction::Read => "SELECT",
        dol_core::ir::control::PolicyAction::Write => "ALL",
        dol_core::ir::control::PolicyAction::All => "ALL",
    };

    let mut sql = format!(
        "CREATE POLICY {} ON {} FOR {}",
        ir.name, ir.on_model, action_str
    );

    if let Some(ref expr) = ir.using_expr {
        let rendered = render_expr(expr, &mut counter, dialect);
        sql.push_str(&format!(" USING ({})", rendered));
    }

    if let Some(ref expr) = ir.check_expr {
        let rendered = render_expr(expr, &mut counter, dialect);
        sql.push_str(&format!(" WITH CHECK ({})", rendered));
    }

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

/// Render a compound query (UNION, INTERSECT, EXCEPT) to SQL.
///
/// All sub-queries are rendered with a single shared `ParamCounter` so that
/// bind-parameter numbers are globally unique across the entire compound
/// statement (e.g. Postgres `$1, $2, …`).
pub fn render_compound_query_ir(
    ir: &CompoundQueryIR,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    let base = render_query_ir_with_counter(&ir.base, dialect, &mut counter)?;

    let mut sql = base.sql;

    for (kind, query_ir) in &ir.operations {
        let kind_str = match kind {
            SetOpKind::Union => "UNION",
            SetOpKind::UnionAll => "UNION ALL",
            SetOpKind::Intersect => "INTERSECT",
            SetOpKind::IntersectAll => "INTERSECT ALL",
            SetOpKind::Except => "EXCEPT",
            SetOpKind::ExceptAll => "EXCEPT ALL",
        };
        let part = render_query_ir_with_counter(query_ir, dialect, &mut counter)?;
        sql.push_str(&format!(" {} {}", kind_str, part.sql));
    }

    if !ir.order_by.is_empty() {
        sql.push_str(&render_order_by_exprs(&ir.order_by, &mut counter, dialect));
    }

    if ir.offset.is_some() || ir.limit.is_some() {
        sql.push_str(&render_pagination(
            &ir.offset,
            &ir.limit,
            &mut counter,
            dialect,
        ));
    }

    Ok(SqlOutput {
        sql,
        param_count: counter.count(),
    })
}

// ===========================================================================
// Helpers
// ===========================================================================

fn entity_ref_to_sql(mref: &EntityRef, _dialect: &Dialect) -> String {
    let name = if let Some(ref ns) = mref.namespace {
        format!("{}.{}", ns, mref.name)
    } else {
        mref.name.clone()
    };
    if let Some(ref alias) = mref.alias {
        format!("{} AS {}", name, alias)
    } else {
        name
    }
}

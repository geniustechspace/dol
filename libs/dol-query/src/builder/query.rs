//! GET (SELECT) query builder — the primary read path for DOL.
//!
//! `GetBuilder` borrows a `&Model` and provides chainable methods to compose
//! a SELECT query. Call `.build()` to produce a [`Query`].
//!
//! For SQL rendering, import the `Render` extension trait from `dol-sql`.

use dol_entity::Entity;
use dol_core::expr::{Direction, Expr, NullsPosition, OrderByExpr, field_dyn};
use dol_core::op::{EntityRef, Join, JoinKind, LockMode, OffsetLimit, Query};

// ---------------------------------------------------------------------------
// Private join helper
// ---------------------------------------------------------------------------

/// An internal join clause before it is lowered to [`Join`].
#[derive(Debug, Clone)]
struct JoinClause {
    join_type: JoinKind,
    model_name: String,
    model_namespace: Option<String>,
    alias: Option<String>,
    on_conditions: Vec<(String, String)>,
}

// ===========================================================================
// GetBuilder
// ===========================================================================

/// A composable SELECT query builder that borrows a `&Model`.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct GetBuilder<'a> {
    model: &'a Entity,
    table_alias: Option<String>,
    projections: Vec<Expr<'static>>,
    joins: Vec<JoinClause>,
    filters: Vec<Expr<'static>>,
    group_by: Vec<Expr<'static>>,
    having: Vec<Expr<'static>>,
    order_by: Vec<OrderByExpr<'static>>,
    has_offset: bool,
    has_limit: bool,
    distinct: bool,
    distinct_on: Vec<String>,
    lock_mode: Option<LockMode>,
}

impl<'a> GetBuilder<'a> {
    /// Create a new `GetBuilder` for the given model.
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            table_alias: None,
            projections: Vec::new(),
            joins: Vec::new(),
            filters: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            order_by: Vec::new(),
            has_offset: false,
            has_limit: false,
            distinct: false,
            distinct_on: Vec::new(),
            lock_mode: None,
        }
    }

    // ── Table alias ─────────────────────────────────────────────────────

    /// Set an alias for the source table (`FROM table AS alias`).
    pub fn alias(mut self, alias: &str) -> Self {
        self.table_alias = Some(alias.to_string());
        self
    }

    // ── Projection methods ──────────────────────────────────────────────

    /// Add named columns to the projection list.
    pub fn fields(mut self, names: &[&str]) -> Self {
        for name in names {
            self.projections.push(field_dyn(name));
        }
        self
    }

    /// Add a single expression to the projection list.
    ///
    /// Use the expression language to build any projection:
    ///
    /// ```ignore
    /// .field(field("email").alias("user_email"))
    /// .field(Expr::CountStar.alias("total"))
    /// .field(func::sum(field("amount")))
    /// .field(raw_expr("COALESCE(name, email)"))
    /// ```
    pub fn field(mut self, expr: Expr<'static>) -> Self {
        self.projections.push(expr);
        self
    }

    // ── JOIN methods ────────────────────────────────────────────────────

    /// Add a JOIN of the specified type on another model.
    ///
    /// `on_conditions` is a slice of `(left_col, right_col)` pairs that will
    /// be rendered as `ON left_col = right_col AND ...`.
    pub fn join(
        mut self,
        join_type: JoinKind,
        target: &Entity,
        on_conditions: &[(&str, &str)],
    ) -> Self {
        self.joins.push(JoinClause {
            join_type,
            model_name: target.name.to_string(),
            model_namespace: target.namespace.map(|s| s.to_string()),
            alias: None,
            on_conditions: on_conditions
                .iter()
                .map(|(l, r)| (l.to_string(), r.to_string()))
                .collect(),
        });
        self
    }

    /// Add a JOIN with an explicit alias for the target table.
    pub fn join_aliased(
        mut self,
        join_type: JoinKind,
        target: &Entity,
        alias: &str,
        on_conditions: &[(&str, &str)],
    ) -> Self {
        self.joins.push(JoinClause {
            join_type,
            model_name: target.name.to_string(),
            model_namespace: target.namespace.map(|s| s.to_string()),
            alias: Some(alias.to_string()),
            on_conditions: on_conditions
                .iter()
                .map(|(l, r)| (l.to_string(), r.to_string()))
                .collect(),
        });
        self
    }

    /// Shorthand for `INNER JOIN`.
    pub fn inner_join(self, target: &Entity, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Inner, target, on_conditions)
    }

    /// Shorthand for `LEFT JOIN`.
    pub fn left_join(self, target: &Entity, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Left, target, on_conditions)
    }

    /// Shorthand for `RIGHT JOIN`.
    pub fn right_join(self, target: &Entity, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Right, target, on_conditions)
    }

    /// Shorthand for `FULL OUTER JOIN`.
    pub fn full_join(self, target: &Entity, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Full, target, on_conditions)
    }

    // ── Filter methods ──────────────────────────────────────────────────

    /// Add an arbitrary filter expression to the WHERE clause.
    ///
    /// Multiple filters are AND-joined. Use the expression language to
    /// build conditions:
    ///
    /// ```ignore
    /// .filter(field("id").eq(param()))
    /// .filter(field("email").ilike(param()))
    /// .filter(field("status").eq(raw_expr("'active'")))
    /// .filter(raw_expr("created_at > NOW() - INTERVAL '30 days'"))
    /// ```
    pub fn filter(mut self, expr: Expr<'static>) -> Self {
        self.filters.push(expr);
        self
    }

    // ── GROUP BY / HAVING ───────────────────────────────────────────────

    /// Set the GROUP BY columns.
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|c| field_dyn(c)).collect();
        self
    }

    /// Add a HAVING filter expression.
    pub fn having(mut self, expr: Expr<'static>) -> Self {
        self.having.push(expr);
        self
    }

    // ── ORDER BY ────────────────────────────────────────────────────────

    /// Add `column DESC` to the ORDER BY clause.
    pub fn order_by_desc(mut self, column: &str) -> Self {
        self.order_by.push(OrderByExpr {
            expr: field_dyn(column),
            direction: Direction::Desc,
            nulls: None,
        });
        self
    }

    /// Add `column ASC` to the ORDER BY clause.
    pub fn order_by_asc(mut self, column: &str) -> Self {
        self.order_by.push(OrderByExpr {
            expr: field_dyn(column),
            direction: Direction::Asc,
            nulls: None,
        });
        self
    }

    /// Add an ORDER BY clause with explicit direction and optional NULLS position.
    pub fn order_by(
        mut self,
        column: &str,
        direction: Direction,
        nulls: Option<NullsPosition>,
    ) -> Self {
        self.order_by.push(OrderByExpr {
            expr: field_dyn(column),
            direction,
            nulls,
        });
        self
    }

    /// Add a fully-constructed [`OrderByExpr`] to the ORDER BY clause.
    pub fn order_by_expr(mut self, expr: OrderByExpr<'static>) -> Self {
        self.order_by.push(expr);
        self
    }

    // ── Pagination ──────────────────────────────────────────────────────

    /// Add an OFFSET bind-parameter placeholder.
    pub fn offset(mut self) -> Self {
        self.has_offset = true;
        self
    }

    /// Add a LIMIT bind-parameter placeholder.
    pub fn limit(mut self) -> Self {
        self.has_limit = true;
        self
    }

    // ── DISTINCT ────────────────────────────────────────────────────────

    /// Enable `SELECT DISTINCT`.
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Enable `SELECT DISTINCT ON (columns)` (PostgreSQL-specific).
    pub fn distinct_on(mut self, columns: &[&str]) -> Self {
        self.distinct = true;
        self.distinct_on = columns.iter().map(|c| c.to_string()).collect();
        self
    }

    // ── Row-level locking ───────────────────────────────────────────────

    /// Add `FOR UPDATE` locking.
    pub fn for_update(mut self) -> Self {
        self.lock_mode = Some(LockMode::ForUpdate);
        self
    }

    /// Add `FOR SHARE` locking.
    pub fn for_share(mut self) -> Self {
        self.lock_mode = Some(LockMode::ForShare);
        self
    }

    /// Set an arbitrary [`LockMode`].
    pub fn lock(mut self, mode: LockMode) -> Self {
        self.lock_mode = Some(mode);
        self
    }

    // ── Param introspection ─────────────────────────────────────────────

    /// Returns the number of bind parameters this query will consume.
    pub fn param_count(&self) -> usize {
        let proj_params = count_expr_params(&self.projections);
        let filter_params = count_expr_params(&self.filters);
        let group_params = count_expr_params(&self.group_by);
        let having_params = count_expr_params(&self.having);
        let order_params: usize = self
            .order_by
            .iter()
            .map(|ob| count_single_expr_params(&ob.expr))
            .sum();
        let offset_params = if self.has_offset { 1 } else { 0 };
        let limit_params = if self.has_limit { 1 } else { 0 };

        proj_params
            + filter_params
            + group_params
            + having_params
            + order_params
            + offset_params
            + limit_params
    }

    // ── Build to IR ─────────────────────────────────────────────────────

    /// Consume the builder and produce a [`Query`].
    ///
    /// When no projections have been set (via `.fields()`, `.field()`,
    /// etc.), all entity fields are selected by default.
    pub fn build(self) -> Query<'a> {
        let source = EntityRef {
            name: self.model.name.to_string(),
            namespace: self.model.namespace.map(|s| s.to_string()),
            alias: self.table_alias,
        };

        let joins = self
            .joins
            .into_iter()
            .map(|jc| Join {
                join_type: jc.join_type,
                target: EntityRef {
                    name: jc.model_name,
                    namespace: jc.model_namespace,
                    alias: jc.alias,
                },
                on_conditions: jc.on_conditions,
            })
            .collect();

        let offset = if self.has_offset {
            Some(OffsetLimit::Param)
        } else {
            None
        };

        let limit = if self.has_limit {
            Some(OffsetLimit::Param)
        } else {
            None
        };

        // Default: select all entity fields when no projections were specified.
        let projections = if self.projections.is_empty() {
            self.model
                .fields
                .iter()
                .map(|f| field_dyn(f.name))
                .collect()
        } else {
            self.projections
        };

        Query {
            source,
            projections,
            joins,
            filters: self.filters,
            group_by: self.group_by,
            having: self.having,
            order_by: self.order_by,
            offset,
            limit,
            distinct: self.distinct,
            distinct_on: self.distinct_on,
            lock_mode: self.lock_mode,
        }
    }
}

// ===========================================================================
// Param counting helpers
// ===========================================================================

/// Recursively count the number of bind parameters in a single expression.
pub(crate) fn count_single_expr_params(expr: &Expr<'static>) -> usize {
    match expr {
        Expr::Param => 1,

        Expr::BinaryOp { left, right, .. } => {
            count_single_expr_params(left) + count_single_expr_params(right)
        }

        Expr::UnaryOp { expr, .. } => count_single_expr_params(expr),

        Expr::Func { args, .. } => args.iter().map(count_single_expr_params).sum(),

        Expr::Cast { expr, .. } => count_single_expr_params(expr),

        Expr::Case { whens, else_expr } => {
            let when_params: usize = whens
                .iter()
                .map(|(cond, then)| count_single_expr_params(cond) + count_single_expr_params(then))
                .sum();
            let else_params = else_expr
                .as_ref()
                .map(|e| count_single_expr_params(e))
                .unwrap_or(0);
            when_params + else_params
        }

        Expr::InList { expr, list, .. } => {
            count_single_expr_params(expr)
                + list.iter().map(count_single_expr_params).sum::<usize>()
        }

        Expr::Between {
            expr, low, high, ..
        } => {
            count_single_expr_params(expr)
                + count_single_expr_params(low)
                + count_single_expr_params(high)
        }

        Expr::Window {
            func,
            partition_by,
            order_by,
            ..
        } => {
            count_single_expr_params(func)
                + partition_by
                    .iter()
                    .map(count_single_expr_params)
                    .sum::<usize>()
                + order_by
                    .iter()
                    .map(|ob| count_single_expr_params(&ob.expr))
                    .sum::<usize>()
        }

        Expr::Alias { expr, .. } => count_single_expr_params(expr),

        Expr::Access { base, .. } => count_single_expr_params(base),

        Expr::Object(fields) => fields
            .iter()
            .map(|(_, v)| count_single_expr_params(v))
            .sum(),

        Expr::Array(elements) => elements.iter().map(count_single_expr_params).sum(),

        _ => 0,
    }
}

/// Count the total number of bind parameters across a slice of expressions.
fn count_expr_params(exprs: &[Expr<'static>]) -> usize {
    exprs.iter().map(count_single_expr_params).sum()
}

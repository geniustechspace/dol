//! GET (SELECT) query builder — the primary read path for DOL.
//!
//! `GetBuilder` borrows a `&Model` and provides chainable methods to compose
//! a SELECT query. Call `.build()` to produce a [`dol_ir::Statement`].
//!
//! For SQL rendering, import the `Render` extension trait from `dol-sql`.

use dol_schema::Entity;
use dol_expr::tree::{Direction, Expr, NullsPosition, OrderByExpr, field_dyn};

use crate::{JoinKind, LockMode};

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

    // ── Aggregation ─────────────────────────────────────────────────────

    /// Set the fields to group by in aggregation queries.
    ///
    /// In SQL-backed stores this maps to GROUP BY; in document stores it
    /// drives aggregation pipeline grouping.
    pub fn aggregate_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|c| field_dyn(c)).collect();
        self
    }

    #[deprecated(note = "use `aggregate_by()`")]
    #[inline]
    pub fn group_by(self, columns: &[&str]) -> Self {
        self.aggregate_by(columns)
    }

    /// Add a post-aggregation filter expression.
    ///
    /// In SQL-backed stores this maps to HAVING; in document stores it
    /// applies after the grouping stage.
    pub fn aggregate_filter(mut self, expr: Expr<'static>) -> Self {
        self.having.push(expr);
        self
    }

    #[deprecated(note = "use `aggregate_filter()`")]
    #[inline]
    pub fn having(self, expr: Expr<'static>) -> Self {
        self.aggregate_filter(expr)
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

    // ── Deduplication ───────────────────────────────────────────────────

    /// Eliminate duplicate result rows.
    ///
    /// In SQL-backed stores this maps to SELECT DISTINCT.
    pub fn deduplicate(mut self) -> Self {
        self.distinct = true;
        self
    }

    #[deprecated(note = "use `deduplicate()`")]
    #[inline]
    pub fn distinct(self) -> Self {
        self.deduplicate()
    }

    /// Eliminate duplicates based on specified columns (backend-specific).
    ///
    /// In PostgreSQL this maps to SELECT DISTINCT ON (columns).
    pub fn deduplicate_on(mut self, columns: &[&str]) -> Self {
        self.distinct = true;
        self.distinct_on = columns.iter().map(|c| c.to_string()).collect();
        self
    }

    #[deprecated(note = "use `deduplicate_on()`")]
    #[inline]
    pub fn distinct_on(self, columns: &[&str]) -> Self {
        self.deduplicate_on(columns)
    }

    // ── Row-level locking ───────────────────────────────────────────────

    /// Acquire an exclusive row-level lock on matched rows.
    ///
    /// In SQL-backed stores this maps to FOR UPDATE.
    pub fn lock_exclusive(mut self) -> Self {
        self.lock_mode = Some(LockMode::ForUpdate);
        self
    }

    #[deprecated(note = "use `lock_exclusive()`")]
    #[inline]
    pub fn for_update(self) -> Self {
        self.lock_exclusive()
    }

    /// Acquire a shared row-level lock on matched rows.
    ///
    /// In SQL-backed stores this maps to FOR SHARE.
    pub fn lock_shared(mut self) -> Self {
        self.lock_mode = Some(LockMode::ForShare);
        self
    }

    #[deprecated(note = "use `lock_shared()`")]
    #[inline]
    pub fn for_share(self) -> Self {
        self.lock_shared()
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

    /// Consume the builder and produce a [`dol_ir::Statement`].
    ///
    /// When no projections have been set (via `.fields()`, `.field()`,
    /// etc.), all entity fields are selected by default.
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        use crate::lower::{lower_expr, lower_filters, lower_order_by};
        use dol_expr::expr::{
            BinOp, ExprNode, JoinNode, JoinType as ArenaJoinType, LockHint, QueryNode,
        };
        use dol_expr::ids::NULL_NODE;
        use smallvec::SmallVec;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();

        let from = interner.intern(&crate::lower::qualified_name(
            &self.model.name.to_string(),
            &self.model.namespace.map(|s| s.to_string()),
        ));
        let alias = self.table_alias.as_deref().map(|a| interner.intern(a));

        // Default projections.
        let proj_exprs: Vec<Expr<'static>> = if self.projections.is_empty() {
            self.model.fields.iter().map(|f| field_dyn(f.name)).collect()
        } else {
            self.projections
        };
        let columns: SmallVec<[u32; 8]> = proj_exprs
            .iter()
            .map(|e| lower_expr(e, &mut arena, &mut interner))
            .collect();

        // Joins.
        let joins: SmallVec<[JoinNode; 2]> = self
            .joins
            .into_iter()
            .map(|jc| {
                let source = interner.intern(&jc.model_name);
                let jalias = jc.alias.as_deref().map(|a| interner.intern(a));
                let join_type = match jc.join_type {
                    JoinKind::Inner => ArenaJoinType::Inner,
                    JoinKind::Left => ArenaJoinType::Left,
                    JoinKind::Right => ArenaJoinType::Right,
                    JoinKind::Full => ArenaJoinType::Full,
                    JoinKind::Cross => ArenaJoinType::Cross,
                };
                let on = if jc.on_conditions.is_empty() {
                    NULL_NODE
                } else {
                    let mut cond_ids: Vec<u32> = Vec::new();
                    for (l, r) in &jc.on_conditions {
                        let lid = {
                            let col = interner.intern(l);
                            let fid = arena.alloc_field(dol_expr::FieldNode {
                                namespace: None,
                                column: col,
                                steps: SmallVec::new(),
                            });
                            arena.alloc(ExprNode::Field(fid))
                        };
                        let rid = {
                            let col = interner.intern(r);
                            let fid = arena.alloc_field(dol_expr::FieldNode {
                                namespace: None,
                                column: col,
                                steps: SmallVec::new(),
                            });
                            arena.alloc(ExprNode::Field(fid))
                        };
                        cond_ids.push(arena.alloc(ExprNode::BinOp {
                            op: BinOp::Eq,
                            lhs: lid,
                            rhs: rid,
                        }));
                    }
                    let mut result = cond_ids[0];
                    for id in &cond_ids[1..] {
                        result = arena.alloc(ExprNode::BinOp {
                            op: BinOp::And,
                            lhs: result,
                            rhs: *id,
                        });
                    }
                    result
                };
                JoinNode {
                    source,
                    alias: jalias,
                    join_type,
                    on,
                }
            })
            .collect();

        let filter = lower_filters(&self.filters, &mut arena, &mut interner);

        let group_by: SmallVec<[u32; 4]> = self
            .group_by
            .iter()
            .map(|e| lower_expr(e, &mut arena, &mut interner))
            .collect();

        let having = if self.having.is_empty() {
            NULL_NODE
        } else {
            lower_filters(&self.having, &mut arena, &mut interner)
        };

        let order_by: SmallVec<[(u32, dol_expr::expr::Order); 4]> = self
            .order_by
            .iter()
            .map(|ob| lower_order_by(ob, &mut arena, &mut interner))
            .collect();

        // Note: LockHint has fewer variants than LockMode — ForShare+NoWait
        // and ForShare+SkipLocked are approximated as NoWait/SkipLocked
        // (losing the ForShare distinction). This is a dol-expr limitation.
        let lock = self.lock_mode.map(|m| match m {
            LockMode::ForUpdate => LockHint::ForUpdate,
            LockMode::ForShare => LockHint::ForShare,
            LockMode::ForUpdateNoWait | LockMode::ForShareNoWait => LockHint::NoWait,
            LockMode::ForUpdateSkipLocked | LockMode::ForShareSkipLocked => LockHint::SkipLocked,
        });

        let node = QueryNode {
            from,
            alias,
            joins,
            filter,
            columns,
            group_by,
            having,
            order_by,
            limit: None,
            offset: None,
            lock,
        };

        (dol_ir::Statement::Query(node), arena, interner)
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

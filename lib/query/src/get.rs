//! GET (SELECT) query builder for `dol-query`.
//!
//! Mirrors `dol-builder::GetBuilder` but works with owned name/namespace
//! instead of requiring a static `&Entity` reference.

use crate::JoinKind;
#[cfg(feature = "sql")]
use dol_expr::expr::LockHint;
use dol_expr::tree::{Direction, Expr, NullsPosition, OrderByExpr, field_dyn};

// ---------------------------------------------------------------------------
// Private join helper
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct JoinClause {
    join_type: JoinKind,
    target_name: String,
    target_namespace: Option<String>,
    alias: Option<String>,
    on_conditions: Vec<(String, String)>,
}

// ===========================================================================
// GetQuery
// ===========================================================================

/// A composable SELECT query builder that works with any entity source.
///
/// Construct via [`Query::from(...).get()`](crate::Query::get).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .try_build() is called"]
pub struct GetQuery {
    name: String,
    namespace: Option<String>,
    field_names: Option<Vec<String>>,
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
    /// Row-level lock hint. SQL-only — settable via [`for_update`], [`for_share`],
    /// or [`lock`] when the `sql` feature is enabled.
    ///
    /// [`for_update`]: Self::for_update
    /// [`for_share`]:  Self::for_share
    /// [`lock`]:       Self::lock
    #[cfg(feature = "sql")]
    lock_mode: Option<LockHint>,
}

impl GetQuery {
    pub(crate) fn new(
        name: String,
        namespace: Option<String>,
        field_names: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            namespace,
            field_names,
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
            #[cfg(feature = "sql")]
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

    /// Add a JOIN of the specified type on another entity by name.
    pub fn join(
        mut self,
        join_type: JoinKind,
        target: &str,
        on_conditions: &[(&str, &str)],
    ) -> Self {
        let (ns, n) = parse_entity_name(target);
        self.joins.push(JoinClause {
            join_type,
            target_name: n,
            target_namespace: ns,
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
        target: &str,
        alias: &str,
        on_conditions: &[(&str, &str)],
    ) -> Self {
        let (ns, n) = parse_entity_name(target);
        self.joins.push(JoinClause {
            join_type,
            target_name: n,
            target_namespace: ns,
            alias: Some(alias.to_string()),
            on_conditions: on_conditions
                .iter()
                .map(|(l, r)| (l.to_string(), r.to_string()))
                .collect(),
        });
        self
    }

    /// Shorthand for `INNER JOIN`.
    pub fn inner_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Inner, target, on_conditions)
    }

    /// Shorthand for `LEFT JOIN`.
    pub fn left_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Left, target, on_conditions)
    }

    /// Shorthand for `RIGHT JOIN`.
    pub fn right_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Right, target, on_conditions)
    }

    /// Shorthand for `FULL OUTER JOIN`.
    pub fn full_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinKind::Full, target, on_conditions)
    }

    // ── Filter methods ──────────────────────────────────────────────────

    /// Add an arbitrary filter expression to the WHERE clause.
    ///
    /// Multiple filters are AND-joined. Use the expression language:
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

    // ── Row-level locking (SQL-only) ────────────────────────────────────

    /// Add `FOR UPDATE` locking. SQL-specific.
    #[cfg(feature = "sql")]
    pub fn for_update(mut self) -> Self {
        self.lock_mode = Some(LockHint::ForUpdate);
        self
    }

    /// Add `FOR SHARE` locking. SQL-specific.
    #[cfg(feature = "sql")]
    pub fn for_share(mut self) -> Self {
        self.lock_mode = Some(LockHint::ForShare);
        self
    }

    /// Set an arbitrary [`LockHint`]. SQL-specific.
    #[cfg(feature = "sql")]
    pub fn lock(mut self, mode: LockHint) -> Self {
        self.lock_mode = Some(mode);
        self
    }

    // ── Build to IR ─────────────────────────────────────────────────────

    /// Consume the builder and produce a [`dol_ir::Program`] holding a
    /// single [`dol_ir::Operation::Query`] that references an arena
    /// [`ExprNode::Query`](dol_expr::expr::ExprNode::Query) carrying the
    /// SELECT body.
    ///
    /// When no projections have been set and Entity field metadata is
    /// available, all entity fields are selected by default.
    ///
    /// Fallible: returns the matching [`BuildError`] variant when lowering
    /// any of the projection / WHERE / GROUP BY / HAVING / ORDER BY
    /// expressions exhausts the default budget.
    pub fn try_build(self) -> Result<dol_ir::Program, crate::BuildError> {
        use dol_core::policy::{Budget, Limits};
        use dol_expr::expr::{ExprNode, JoinNode, JoinType as ArenaJoinType, QueryNode};
        use dol_expr::ids::NodeId;
        use dol_expr::lower::{
            lower_exprs, lower_expr_with_budget, lower_filters, lower_order_by,
        };
        use dol_ir::TargetKind;
        use dol_ir::operation::Query as OpQuery;
        use smallvec::SmallVec;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();
        let mut budget = Budget::new(Limits::host());

        let from = interner.intern(&dol_expr::lower::qualified_name(
            &self.name,
            &self.namespace,
        ));
        let alias = self.table_alias.as_deref().map(|a| interner.intern(a));

        // Default projections.
        let proj_exprs: Vec<Expr<'static>> = if self.projections.is_empty() {
            if let Some(ref names) = self.field_names {
                names.iter().map(|n| field_dyn(n)).collect()
            } else {
                Vec::new()
            }
        } else {
            self.projections
        };
        let columns: SmallVec<[NodeId; 8]> =
            lower_exprs(&proj_exprs, &mut arena, &mut interner, &mut budget)
                .map_err(crate::BuildError::Projection)?;

        // Joins.
        let mut joins: SmallVec<[JoinNode; 2]> = SmallVec::new();
        for jc in self.joins {
            // Honor the parsed `"namespace.name"` form by re-joining the
            // dotted source. The interner deduplicates so this is cheap;
            // backends parse the dotted form when they need the parts.
            let source = match &jc.target_namespace {
                Some(ns) => interner.intern(&format!("{ns}.{}", jc.target_name)),
                None => interner.intern(&jc.target_name),
            };
            let alias = jc.alias.as_deref().map(|a| interner.intern(a));
            let join_type = match jc.join_type {
                JoinKind::Inner => ArenaJoinType::Inner,
                JoinKind::Left => ArenaJoinType::Left,
                JoinKind::Right => ArenaJoinType::Right,
                JoinKind::Full => ArenaJoinType::Full,
                JoinKind::Cross => ArenaJoinType::Cross,
            };
            // Build ON condition from pairs (`None` for absent, e.g.
            // `CROSS JOIN`).
            let on: Option<NodeId> = if jc.on_conditions.is_empty() {
                None
            } else {
                // ON-conditions are simple `col = col` pairs and don't
                // need budget-aware lowering — they're constructed
                // structurally from interner ids. We still charge the
                // budget at allocation so a 4 G `ON ... AND ...` chain
                // can't run unbounded.
                let mut cond_ids: Vec<NodeId> = Vec::new();
                for (l, r) in &jc.on_conditions {
                    // Charge two ticks: one for each `Field` allocation
                    // (left and right column references) emitted below.
                    // Keeps the worst-case `ON a=b AND c=d AND ...`
                    // chain bounded by the configured fuel cap.
                    budget
                        .tick(2)
                        .map_err(|e| crate::BuildError::JoinOn(dol_expr::lower::LowerError::from(e)))?;
                    let lid = {
                        let col = interner.intern(l);
                        let fid = arena.alloc_field(dol_expr::FieldNode {
                            namespace: None,
                            name: col,
                            steps: SmallVec::new(),
                        });
                        arena.alloc(ExprNode::Field(fid))
                    };
                    let rid = {
                        let col = interner.intern(r);
                        let fid = arena.alloc_field(dol_expr::FieldNode {
                            namespace: None,
                            name: col,
                            steps: SmallVec::new(),
                        });
                        arena.alloc(ExprNode::Field(fid))
                    };
                    cond_ids.push(arena.alloc(ExprNode::BinOp {
                        op: dol_expr::expr::BinOp::Eq,
                        lhs: lid,
                        rhs: rid,
                    }));
                }
                let mut result = cond_ids[0];
                for id in &cond_ids[1..] {
                    result = arena.alloc(ExprNode::BinOp {
                        op: dol_expr::expr::BinOp::And,
                        lhs: result,
                        rhs: *id,
                    });
                }
                Some(result)
            };
            joins.push(JoinNode {
                source,
                alias,
                join_type,
                on,
            });
        }

        let filter = lower_filters(&self.filters, &mut arena, &mut interner, &mut budget)
            .map_err(crate::BuildError::Filter)?;

        let mut group_by: SmallVec<[NodeId; 4]> = SmallVec::new();
        for e in &self.group_by {
            let nid = lower_expr_with_budget(e, &mut arena, &mut interner, &mut budget)
                .map_err(crate::BuildError::GroupBy)?;
            group_by.push(nid);
        }

        let having: Option<NodeId> = if self.having.is_empty() {
            None
        } else {
            lower_filters(&self.having, &mut arena, &mut interner, &mut budget)
                .map_err(crate::BuildError::Having)?
        };

        let mut order_by: SmallVec<[(NodeId, dol_expr::expr::Order); 4]> = SmallVec::new();
        for ob in &self.order_by {
            let pair = lower_order_by(ob, &mut arena, &mut interner, &mut budget)
                .map_err(crate::BuildError::OrderBy)?;
            order_by.push(pair);
        }

        #[cfg(feature = "sql")]
        let lock = self.lock_mode;
        #[cfg(not(feature = "sql"))]
        let lock: Option<dol_expr::expr::LockHint> = None;

        let qnode = QueryNode {
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

        // Lower the QueryNode into the arena and reference it from
        // Operation::Query.
        let qid = arena.alloc_query(qnode);
        let body = arena.alloc(ExprNode::Query(qid));

        let target = crate::target::target_from_parts(
            &mut interner,
            TargetKind::Relation,
            &self.name,
            self.namespace.as_deref(),
        );
        let op: dol_ir::Operation = OpQuery {
            target,
            node: Some(body),
        }
        .into();
        Ok(dol_ir::Program::new(op, arena, interner))
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Parse `"namespace.name"` into `(Some(namespace), name)` or `(None, name)`.
fn parse_entity_name(s: &str) -> (Option<String>, String) {
    match s.rsplit_once('.') {
        Some((ns, n)) => (Some(ns.to_string()), n.to_string()),
        None => (None, s.to_string()),
    }
}

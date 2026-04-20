//! GET (SELECT) query builder for `dol-query`.
//!
//! Mirrors `dol-builder::GetBuilder` but works with owned name/namespace
//! instead of requiring a static `&Entity` reference.

use dol_core::expr::{Direction, Expr, NullsPosition, OrderByExpr, field};
use dol_core::op::{EntityRef, Join, JoinKind, LockMode, OffsetLimit, Query};

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
#[must_use = "builders do nothing until .build() is called"]
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
    lock_mode: Option<LockMode>,
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
            self.projections.push(field(name));
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
        self.group_by = columns.iter().map(|c| field(c)).collect();
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
            expr: field(column),
            direction: Direction::Desc,
            nulls: None,
        });
        self
    }

    /// Add `column ASC` to the ORDER BY clause.
    pub fn order_by_asc(mut self, column: &str) -> Self {
        self.order_by.push(OrderByExpr {
            expr: field(column),
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
            expr: field(column),
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

    // ── Build to IR ─────────────────────────────────────────────────────

    /// Consume the builder and produce a [`Query`].
    ///
    /// When no projections have been set and Entity field metadata is
    /// available, all entity fields are selected by default.
    pub fn build(self) -> Query<'static> {
        let source = EntityRef {
            name: self.name,
            namespace: self.namespace,
            alias: self.table_alias,
        };

        let joins = self
            .joins
            .into_iter()
            .map(|jc| Join {
                join_type: jc.join_type,
                target: EntityRef {
                    name: jc.target_name,
                    namespace: jc.target_namespace,
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

        // Default: select all entity fields when no projections were specified
        // and field metadata is available.
        let projections = if self.projections.is_empty() {
            if let Some(ref names) = self.field_names {
                names.iter().map(|n| Expr::Identifier(n.clone())).collect()
            } else {
                self.projections
            }
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
// Helpers
// ===========================================================================

/// Parse `"namespace.name"` into `(Some(namespace), name)` or `(None, name)`.
fn parse_entity_name(s: &str) -> (Option<String>, String) {
    match s.rsplit_once('.') {
        Some((ns, n)) => (Some(ns.to_string()), n.to_string()),
        None => (None, s.to_string()),
    }
}

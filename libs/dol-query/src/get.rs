//! GET (SELECT) query builder for `dol-query`.
//!
//! Mirrors `dol-builder::GetBuilder` but works with owned name/namespace
//! instead of requiring a static `&Entity` reference.

use dol_expr::{Direction, Expr, NullsPosition, OrderByExpr, col, param, raw_expr};
use dol_ir::{EntityRef, JoinIR, JoinType, LockMode, OffsetLimit, QueryIR};

// ---------------------------------------------------------------------------
// Private join helper
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct JoinClause {
    join_type: JoinType,
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
    projections: Vec<Expr>,
    joins: Vec<JoinClause>,
    filters: Vec<Expr>,
    group_by: Vec<Expr>,
    having: Vec<Expr>,
    order_by: Vec<OrderByExpr>,
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

    /// Select all columns known from the entity metadata.
    ///
    /// # Panics
    ///
    /// Panics if this query was constructed from a plain string without
    /// field metadata. Use `.columns()` instead for string-sourced queries.
    pub fn all_columns(mut self) -> Self {
        let fields = self
            .field_names
            .as_ref()
            .expect("all_columns() requires an Entity source with field metadata");
        for name in fields {
            self.projections.push(Expr::Identifier(name.clone()));
        }
        self
    }

    /// Select a list of named columns.
    pub fn columns(mut self, names: &[&str]) -> Self {
        for name in names {
            self.projections.push(col(name));
        }
        self
    }

    /// Add a single column/expression to the projection list.
    pub fn select_item(mut self, expr: Expr) -> Self {
        self.projections.push(expr);
        self
    }

    /// Select a column aliased to a different name (`col AS alias`).
    pub fn column_as(mut self, name: &str, alias: &str) -> Self {
        self.projections.push(col(name).alias(alias));
        self
    }

    /// Add `COUNT(*)` to the projection list.
    pub fn count_all(mut self) -> Self {
        self.projections.push(Expr::CountStar);
        self
    }

    /// Add `COUNT(*) AS alias` to the projection list.
    pub fn count_all_as(mut self, alias: &str) -> Self {
        self.projections.push(Expr::CountStar.alias(alias));
        self
    }

    /// Add a raw SQL expression to the projection list.
    pub fn raw_column(mut self, sql: &str) -> Self {
        self.projections.push(raw_expr(sql));
        self
    }

    // ── JOIN methods ────────────────────────────────────────────────────

    /// Add a JOIN of the specified type on another entity by name.
    pub fn join(
        mut self,
        join_type: JoinType,
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
        join_type: JoinType,
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
        self.join(JoinType::Inner, target, on_conditions)
    }

    /// Shorthand for `LEFT JOIN`.
    pub fn left_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinType::Left, target, on_conditions)
    }

    /// Shorthand for `RIGHT JOIN`.
    pub fn right_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinType::Right, target, on_conditions)
    }

    /// Shorthand for `FULL OUTER JOIN`.
    pub fn full_join(self, target: &str, on_conditions: &[(&str, &str)]) -> Self {
        self.join(JoinType::Full, target, on_conditions)
    }

    // ── Filter methods ──────────────────────────────────────────────────

    /// Add an arbitrary filter expression to the WHERE clause.
    ///
    /// Multiple filters are AND-joined.
    pub fn filter(mut self, expr: Expr) -> Self {
        self.filters.push(expr);
        self
    }

    /// Add `column = $param` to the WHERE clause.
    pub fn where_eq(mut self, column: &str) -> Self {
        self.filters.push(col(column).eq(param()));
        self
    }

    /// Add `column = <literal>` to the WHERE clause (inline literal, no bind param).
    pub fn where_eq_literal(mut self, column: &str, value: Expr) -> Self {
        self.filters.push(col(column).eq(value));
        self
    }

    /// Add `column ILIKE $param` to the WHERE clause (dialect-aware).
    pub fn where_ilike(mut self, column: &str) -> Self {
        self.filters.push(col(column).ilike(param()));
        self
    }

    /// Add `EXISTS (subquery)` to the WHERE clause.
    pub fn where_exists(mut self, subquery: &str) -> Self {
        self.filters.push(Expr::Exists {
            subquery: subquery.to_string(),
            negated: false,
        });
        self
    }

    /// Add `NOT EXISTS (subquery)` to the WHERE clause.
    pub fn where_not_exists(mut self, subquery: &str) -> Self {
        self.filters.push(Expr::Exists {
            subquery: subquery.to_string(),
            negated: true,
        });
        self
    }

    /// Add `column IN (subquery)` to the WHERE clause.
    pub fn where_in_subquery(mut self, column: &str, subquery: &str) -> Self {
        self.filters.push(Expr::InSubquery {
            expr: Box::new(col(column)),
            subquery: subquery.to_string(),
            negated: false,
        });
        self
    }

    /// Add `column NOT IN (subquery)` to the WHERE clause.
    pub fn where_not_in_subquery(mut self, column: &str, subquery: &str) -> Self {
        self.filters.push(Expr::InSubquery {
            expr: Box::new(col(column)),
            subquery: subquery.to_string(),
            negated: true,
        });
        self
    }

    /// Add a raw SQL expression to the WHERE clause.
    pub fn where_raw(mut self, sql: &str) -> Self {
        self.filters.push(raw_expr(sql));
        self
    }

    // ── GROUP BY / HAVING ───────────────────────────────────────────────

    /// Set the GROUP BY columns.
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|c| col(c)).collect();
        self
    }

    /// Add a HAVING filter expression.
    pub fn having(mut self, expr: Expr) -> Self {
        self.having.push(expr);
        self
    }

    // ── ORDER BY ────────────────────────────────────────────────────────

    /// Add `column DESC` to the ORDER BY clause.
    pub fn order_by_desc(mut self, column: &str) -> Self {
        self.order_by.push(OrderByExpr {
            expr: col(column),
            direction: Direction::Desc,
            nulls: None,
        });
        self
    }

    /// Add `column ASC` to the ORDER BY clause.
    pub fn order_by_asc(mut self, column: &str) -> Self {
        self.order_by.push(OrderByExpr {
            expr: col(column),
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
            expr: col(column),
            direction,
            nulls,
        });
        self
    }

    /// Add a fully-constructed [`OrderByExpr`] to the ORDER BY clause.
    pub fn order_by_expr(mut self, expr: OrderByExpr) -> Self {
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

    /// Consume the builder and produce a [`QueryIR`].
    pub fn build(self) -> QueryIR {
        let source = EntityRef {
            name: self.name,
            namespace: self.namespace,
            alias: self.table_alias,
        };

        let joins = self
            .joins
            .into_iter()
            .map(|jc| JoinIR {
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

        QueryIR {
            source,
            projections: self.projections,
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

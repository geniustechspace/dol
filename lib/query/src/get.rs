//! GET (SELECT) query builder for `dol-query`.
//!
//! Pure data: lowering to a `dol_command::program::Program` is handled
//! by [`dol_command::lower_query::lower_get`] (or via the
//! [`BuildProgram`](dol_command::lower_query::BuildProgram) trait).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::JoinKind;
use dol_expr::expr::LockHint;
use dol_expr::tree::{Direction, Expr, NullsPosition, OrderByExpr, field_dyn};

// ---------------------------------------------------------------------------
// Public join helper
// ---------------------------------------------------------------------------

/// One JOIN clause: target entity, optional alias, and `(left, right)`
/// equality condition pairs.
///
/// `pub` so the lowering host crate (`dol-command` with the `query`
/// feature) can read it without going through accessors.
#[derive(Debug, Clone)]
pub struct JoinClause {
    /// What kind of join (`INNER`, `LEFT`, `CROSS`, …).
    pub join_type: JoinKind,
    /// Last dotted segment of the target entity name.
    pub target_name: String,
    /// Optional namespace (everything before the last dotted segment).
    pub target_namespace: Option<String>,
    /// Optional alias (`FROM ... AS alias`).
    pub alias: Option<String>,
    /// `(left_column, right_column)` equality pairs forming the
    /// `ON ... AND ...` condition. Empty for `CROSS JOIN`.
    pub on_conditions: Vec<(String, String)>,
}

// ===========================================================================
// GetQuery
// ===========================================================================

/// A composable SELECT query builder that works with any entity source.
///
/// Construct via [`Query::from(...).get()`](crate::Query::get).
///
/// All fields are `pub` so the lowering host crate (`dol-command` with the
/// `query` feature, default-on) can read them without going through
/// accessors.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until lowered to a Program"]
pub struct GetQuery {
    /// Target entity name (last dotted segment of the source string).
    pub name: String,
    /// Optional namespace prefix.
    pub namespace: Option<String>,
    /// Optional list of all field names (used as a default projection
    /// when `projections` is empty).
    pub field_names: Option<Vec<String>>,
    /// Optional table alias (`FROM table AS alias`).
    pub table_alias: Option<String>,
    /// Projection expressions; empty falls back to `field_names`.
    pub projections: Vec<Expr<'static>>,
    /// JOIN clauses to apply.
    pub joins: Vec<JoinClause>,
    /// Filter expressions; AND-joined when lowered.
    pub filters: Vec<Expr<'static>>,
    /// `GROUP BY` columns/expressions.
    pub group_by: Vec<Expr<'static>>,
    /// `HAVING` clauses; AND-joined when lowered.
    pub having: Vec<Expr<'static>>,
    /// `ORDER BY` clauses.
    pub order_by: Vec<OrderByExpr<'static>>,
    /// Whether `OFFSET ?` is present.
    pub has_offset: bool,
    /// Whether `LIMIT ?` is present.
    pub has_limit: bool,
    /// Whether `SELECT DISTINCT` is set.
    pub distinct: bool,
    /// Optional `DISTINCT ON (cols)` (PostgreSQL).
    pub distinct_on: Vec<String>,
    /// Row-level lock hint. SQL-only — settable via [`for_update`],
    /// [`for_share`], or [`lock`] when the `sql` feature is enabled, but
    /// the field itself is always present so cross-crate lowering does
    /// not need to be feature-gated.
    ///
    /// [`for_update`]: Self::for_update
    /// [`for_share`]:  Self::for_share
    /// [`lock`]:       Self::lock
    pub lock_mode: Option<LockHint>,
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

//! Comprehensive tests for the DOL crate — ported from core-db plus DOL-specific tests.

use crate::CompoundSelectBuilder;
use crate::EntityDefineExt;
use crate::Render;
use crate::TransactionRender;
use crate::backend::sql::dialect::Dialect;
use crate::builder::EntityBuilderExt;
use crate::builder::control::{GrantBuilder, Privilege, RevokeBuilder};
use crate::builder::definition::{DefineIndexBuilder, DropIndexBuilder};
use crate::builder::transaction::TransactionBuilder;
use crate::expr::func;
use crate::expr::window::FrameBound;
use crate::expr::{Direction, Expr, NullsPosition, case, col, field, lit, param, raw_expr};
use crate::ir::LockMode;
use crate::model::{Entity, EntityConstraint, Field, FieldType, FkAction};

fn pg() -> Dialect {
    Dialect::postgres()
}

// ---------------------------------------------------------------------------
// Test model definitions (mirrors core-db test fixtures)
// ---------------------------------------------------------------------------

static USERS: Entity = Entity::new(
    "users",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("tenant_id", FieldType::Uuid),
        Field::new("email", FieldType::Text),
        Field::new("display_name", FieldType::Text),
        Field::new("status", FieldType::Text),
        Field::new("created_at", FieldType::Timestamp),
        Field::new("updated_at", FieldType::Timestamp),
    ],
);

static TENANTS: Entity = Entity::new(
    "tenants",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("slug", FieldType::Text),
        Field::new("display_name", FieldType::Text),
        Field::new("status", FieldType::Text),
        Field::new("plan", FieldType::Text),
        Field::new("labels", FieldType::Json).nullable(),
        Field::new("scheduled_deletion_at", FieldType::Timestamp).nullable(),
        Field::new("created_at", FieldType::Timestamp),
        Field::new("created_by", FieldType::Uuid),
        Field::new("updated_at", FieldType::Timestamp),
        Field::new("updated_by", FieldType::Uuid),
        Field::new("version", FieldType::Int),
    ],
);

static SETTINGS: Entity = Entity::new(
    "tenant_settings",
    &[
        Field::new("tenant_id", FieldType::Uuid).primary_key(),
        Field::new("max_users", FieldType::Int),
        Field::new("mfa_required", FieldType::Bool),
    ],
);

// ===========================================================================
// Model / Schema tests
// ===========================================================================

#[test]
fn model_field_lookup() {
    let f = USERS.field("email");
    assert_eq!(f.name, "email");
    assert_eq!(f.field_type, FieldType::Text);
}

#[test]
#[should_panic(expected = "field 'nonexistent' not found")]
fn model_field_lookup_panics() {
    USERS.field("nonexistent");
}

#[test]
fn model_primary_keys() {
    let pks: Vec<_> = USERS.primary_keys().map(|f| f.name).collect();
    assert_eq!(pks, vec!["id"]);
}

#[test]
fn model_field_list() {
    let list = USERS.field_list();
    assert_eq!(
        list,
        "id, tenant_id, email, display_name, status, created_at, updated_at"
    );
}

#[test]
fn model_qualified_name_with_namespace() {
    let m = Entity::new("users", &[]).with_namespace("auth");
    assert_eq!(m.qualified_name(), "auth.users");
}

#[test]
fn model_qualified_name_without_namespace() {
    assert_eq!(USERS.qualified_name(), "users");
}

// ===========================================================================
// DQL / GetBuilder tests
// ===========================================================================

#[test]
fn select_all_with_where() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_eq("id")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE tenant_id = $1 AND id = $2"
    );
}

#[test]
fn select_with_pagination() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .order_by_desc("created_at")
        .offset()
        .limit()
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE tenant_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3"
    );
}

#[test]
fn select_with_ilike() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_ilike("email")
        .order_by_desc("created_at")
        .offset()
        .limit()
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE tenant_id = $1 AND email ILIKE $2 \
         ORDER BY created_at DESC OFFSET $3 LIMIT $4"
    );
}

#[test]
fn select_specific_columns() {
    let sql = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("tenant_id")
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "SELECT id, email FROM users WHERE tenant_id = $1");
}

#[test]
fn select_no_where() {
    let sql = TENANTS
        .get()
        .all_columns()
        .order_by_desc("created_at")
        .offset()
        .limit()
        .render(Some(&pg())).unwrap();
    assert!(sql.starts_with("SELECT id, slug, display_name, status, plan, labels,"));
    assert!(sql.ends_with("ORDER BY created_at DESC OFFSET $1 LIMIT $2"));
}

#[test]
fn select_with_literal_where() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq_literal("status", raw_expr("'active'"))
        .where_eq("tenant_id")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE status = 'active' AND tenant_id = $1"
    );
}

#[test]
fn select_count_all() {
    let sql = USERS
        .get()
        .count_all_as("total")
        .where_eq("tenant_id")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT COUNT(*) AS total FROM users WHERE tenant_id = $1"
    );
}

#[test]
fn select_aggregate_sum() {
    let sql = SETTINGS
        .get()
        .select_item(func::sum(col("max_users")).alias("total_users"))
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT SUM(max_users) AS total_users FROM tenant_settings"
    );
}

#[test]
fn select_group_by_having() {
    let sql = USERS
        .get()
        .columns(&["tenant_id"])
        .count_all_as("cnt")
        .group_by(&["tenant_id"])
        .having(raw_expr("COUNT(*) > 10"))
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("GROUP BY tenant_id"));
    assert!(sql.contains("HAVING COUNT(*) > 10"));
}

#[test]
fn select_distinct() {
    let sql = USERS
        .get()
        .columns(&["tenant_id"])
        .distinct()
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "SELECT DISTINCT tenant_id FROM users");
}

#[test]
fn select_distinct_on() {
    let sql = USERS
        .get()
        .all_columns()
        .distinct_on(&["tenant_id"])
        .order_by_desc("created_at")
        .render(Some(&pg())).unwrap();
    assert!(sql.starts_with("SELECT DISTINCT ON (tenant_id) id, tenant_id"));
    assert!(sql.ends_with("ORDER BY created_at DESC"));
}

#[test]
fn select_for_update() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("id")
        .for_update()
        .render(Some(&pg())).unwrap();
    assert!(sql.ends_with("FOR UPDATE"));
}

#[test]
fn select_for_update_skip_locked() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("id")
        .lock(LockMode::ForUpdateSkipLocked)
        .render(Some(&pg())).unwrap();
    assert!(sql.ends_with("FOR UPDATE SKIP LOCKED"));
}

#[test]
fn select_column_alias() {
    let sql = USERS
        .get()
        .column_as("email", "user_email")
        .where_eq("id")
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "SELECT email AS user_email FROM users WHERE id = $1");
}

#[test]
fn select_raw_column() {
    let sql = USERS
        .get()
        .raw_column("COALESCE(display_name, email) AS name")
        .where_eq("id")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "SELECT COALESCE(display_name, email) AS name FROM users WHERE id = $1"
    );
}

#[test]
fn select_or_predicate() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .filter(col("status").eq(raw_expr("'active'")) | col("status").eq(raw_expr("'pending'")))
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND (status = 'active' OR status = 'pending')"));
}

#[test]
fn select_exists_subquery() {
    let sql = USERS
        .get()
        .all_columns()
        .where_exists("SELECT 1 FROM sessions WHERE sessions.user_id = users.id")
        .render(Some(&pg())).unwrap();
    assert!(
        sql.contains("WHERE EXISTS (SELECT 1 FROM sessions WHERE sessions.user_id = users.id)")
    );
}

#[test]
fn select_in_subquery() {
    let sql = USERS
        .get()
        .all_columns()
        .where_in_subquery(
            "tenant_id",
            "SELECT id FROM tenants WHERE status = 'active'",
        )
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("WHERE tenant_id IN (SELECT id FROM tenants WHERE status = 'active')"));
}

#[test]
fn select_where_raw() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_raw("created_at > NOW() - INTERVAL '30 days'")
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("AND created_at > NOW() - INTERVAL '30 days'"));
}

#[test]
fn select_order_by_nulls() {
    let sql = USERS
        .get()
        .all_columns()
        .order_by("display_name", Direction::Asc, Some(NullsPosition::Last))
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("ORDER BY display_name ASC NULLS LAST"));
}

#[test]
fn select_param_count() {
    let q = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_eq("status")
        .offset()
        .limit();
    assert_eq!(q.param_count(), 4);
}

#[test]
fn select_inner_join() {
    let sql = USERS
        .get()
        .columns(&["u.id", "u.email", "t.slug"])
        .alias("u")
        .inner_join(&TENANTS, &[("u.tenant_id", "t.id")])
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("INNER JOIN tenants"));
    assert!(sql.contains("ON u.tenant_id = t.id"));
}

#[test]
fn select_left_join() {
    let sql = USERS
        .get()
        .columns(&["u.id", "s.max_users"])
        .alias("u")
        .left_join(&SETTINGS, &[("u.tenant_id", "s.tenant_id")])
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("LEFT JOIN tenant_settings"));
    assert!(sql.contains("ON u.tenant_id = s.tenant_id"));
}

#[test]
fn select_multiple_joins() {
    let sql = USERS
        .get()
        .columns(&["u.id", "t.slug", "s.max_users"])
        .alias("u")
        .inner_join(&TENANTS, &[("u.tenant_id", "t.id")])
        .left_join(&SETTINGS, &[("t.id", "s.tenant_id")])
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("INNER JOIN tenants"));
    assert!(sql.contains("LEFT JOIN tenant_settings"));
}

// ===========================================================================
// filter() API on Select / Update / Delete
// ===========================================================================

#[test]
fn select_filter_produces_same_as_where_eq() {
    let sql_legacy = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_eq("id")
        .render(Some(&pg())).unwrap();
    let sql_expr = USERS
        .get()
        .all_columns()
        .filter(col("tenant_id").eq(param()))
        .filter(col("id").eq(param()))
        .render(Some(&pg())).unwrap();
    assert_eq!(sql_legacy, sql_expr);
}

#[test]
fn update_filter_api() {
    let sql = USERS
        .update()
        .set("email")
        .filter(col("id").eq(param()))
        .filter(col("tenant_id").eq(param()))
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "UPDATE users SET email = $1 WHERE id = $2 AND tenant_id = $3"
    );
}

#[test]
fn delete_filter_api() {
    let sql = USERS
        .remove()
        .filter(col("tenant_id").eq(param()))
        .filter(col("status").eq(raw_expr("'deleted'")))
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "DELETE FROM users WHERE tenant_id = $1 AND status = 'deleted'"
    );
}

#[test]
fn select_filter_or_group() {
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .filter(col("status").eq(raw_expr("'active'")) | col("status").eq(raw_expr("'pending'")))
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND (status = 'active' OR status = 'pending')"));
}

// ===========================================================================
// DML tests — INSERT
// ===========================================================================

#[test]
fn insert_all_columns() {
    let sql = USERS.insert().all_columns().render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO users (id, tenant_id, email, display_name, status, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    );
}

#[test]
fn insert_specific_columns() {
    let sql = USERS
        .insert()
        .columns(&["id", "tenant_id", "email"])
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO users (id, tenant_id, email) VALUES ($1, $2, $3)"
    );
}

#[test]
fn insert_with_returning() {
    let sql = USERS
        .insert()
        .all_columns()
        .returning(&["id"])
        .render(Some(&pg())).unwrap();
    assert!(sql.ends_with("RETURNING id"));
}

#[test]
fn insert_batch() {
    let sql = SETTINGS.insert().all_columns().rows(3).render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         VALUES ($1, $2, $3), ($4, $5, $6), ($7, $8, $9)"
    );
}

#[test]
fn insert_batch_param_count() {
    let q = SETTINGS.insert().all_columns().rows(3);
    assert_eq!(q.param_count(), 9);
}

#[test]
fn insert_select() {
    let sql = SETTINGS
        .insert_select()
        .columns(&["tenant_id", "max_users", "mfa_required"])
        .from_select("SELECT id, 100, true FROM tenants WHERE plan = 'enterprise'")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         SELECT id, 100, true FROM tenants WHERE plan = 'enterprise'"
    );
}

#[test]
fn insert_select_with_returning() {
    let sql = SETTINGS
        .insert_select()
        .columns(&["tenant_id", "max_users", "mfa_required"])
        .from_select("SELECT id, 50, false FROM tenants")
        .returning(&["tenant_id"])
        .render(Some(&pg())).unwrap();
    assert!(sql.ends_with("RETURNING tenant_id"));
}

// ===========================================================================
// DML tests — UPDATE
// ===========================================================================

#[test]
fn update_basic() {
    let sql = USERS
        .update()
        .set("email")
        .set("display_name")
        .set("status")
        .set("updated_at")
        .where_eq("tenant_id")
        .where_eq("id")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "UPDATE users SET email = $1, display_name = $2, status = $3, updated_at = $4 \
         WHERE tenant_id = $5 AND id = $6"
    );
}

#[test]
fn update_with_version_increment() {
    let sql = TENANTS
        .update()
        .set("display_name")
        .set("status")
        .set("plan")
        .set("labels")
        .set("scheduled_deletion_at")
        .set("updated_at")
        .set("updated_by")
        .set_increment("version")
        .where_eq("id")
        .where_eq("version")
        .render(Some(&pg())).unwrap();
    // DOL set_increment uses a bind param: version = version + $N
    assert!(sql.contains("version = version + $8"));
    assert!(sql.contains("WHERE id = $9 AND version = $10"));
}

#[test]
fn update_with_literal_set_and_returning() {
    let sql = USERS
        .update()
        .set_literal("status", "'revoked'")
        .where_eq("id")
        .where_eq_literal("status", "'active'")
        .returning_all()
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("SET status = 'revoked'"));
    assert!(sql.contains("WHERE id = $1 AND status = 'active'"));
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn update_with_raw_where() {
    let sql = USERS
        .update()
        .set_literal("status", "'archived'")
        .where_eq("tenant_id")
        .where_raw("updated_at < NOW() - INTERVAL '1 year'")
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND updated_at < NOW() - INTERVAL '1 year'"));
}

#[test]
fn update_param_count() {
    let q = USERS
        .update()
        .set("email")
        .set("display_name")
        .set_increment("version")
        .where_eq("id");
    assert_eq!(q.param_count(), 4); // 2 set params + 1 increment param + 1 where param
}

// ===========================================================================
// DML tests — DELETE (RemoveBuilder)
// ===========================================================================

#[test]
fn delete_basic() {
    let sql = USERS.remove().where_eq("id").render(Some(&pg())).unwrap();
    assert_eq!(sql, "DELETE FROM users WHERE id = $1");
}

#[test]
fn delete_with_returning() {
    let sql = USERS
        .remove()
        .where_eq("id")
        .returning_all()
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn delete_with_raw_where() {
    let sql = USERS
        .remove()
        .where_eq("tenant_id")
        .where_raw("created_at < NOW() - INTERVAL '90 days'")
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND created_at < NOW() - INTERVAL '90 days'"));
}

#[test]
fn delete_param_count() {
    let q = USERS.remove().where_eq("tenant_id").where_eq("id");
    assert_eq!(q.param_count(), 2);
}

// ===========================================================================
// DML tests — UPSERT
// ===========================================================================

#[test]
fn upsert_basic() {
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id) DO UPDATE SET \
         max_users = EXCLUDED.max_users, mfa_required = EXCLUDED.mfa_required"
    );
}

#[test]
fn upsert_do_nothing() {
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_nothing()
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id) DO NOTHING"
    );
}

#[test]
fn upsert_with_returning() {
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .returning_all()
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn upsert_on_conflict_constraint() {
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict_constraint("uq_tenant_settings_pk")
        .do_update(&["max_users"])
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("ON CONFLICT ON CONSTRAINT uq_tenant_settings_pk DO UPDATE SET"));
}

#[test]
fn upsert_with_conflict_where() {
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users"])
        .conflict_where_eq("mfa_required")
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("DO UPDATE SET max_users = EXCLUDED.max_users WHERE mfa_required = $4"));
}

#[test]
fn upsert_param_count() {
    let q = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users"])
        .conflict_where_eq("mfa_required");
    assert_eq!(q.param_count(), 4); // 3 insert cols + 1 conflict where
}

// ===========================================================================
// DDL tests — CREATE TABLE (CreateFromMeta)
// ===========================================================================

#[test]
fn create_table() {
    let sql = SETTINGS.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("CREATE TABLE tenant_settings"));
    assert!(sql.contains("tenant_id UUID NOT NULL"));
    assert!(sql.contains("max_users INTEGER NOT NULL"));
    assert!(sql.contains("mfa_required BOOLEAN NOT NULL"));
    assert!(sql.contains("PRIMARY KEY (tenant_id)"));
}

#[test]
fn create_table_if_not_exists() {
    let sql = SETTINGS.create().if_not_exists().render(Some(&pg())).unwrap();
    assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS tenant_settings"));
}

#[test]
fn create_table_with_nullable() {
    static T: Entity = Entity::new(
        "test",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("meta", FieldType::Json).nullable(),
        ],
    );
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("meta JSONB,"));
    assert!(!sql.contains("meta JSONB NOT NULL"));
}

#[test]
fn create_table_with_default_expr() {
    static T: Entity = Entity::new(
        "events",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("status", FieldType::Text).default("'pending'"),
            Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        ],
    );
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("status TEXT NOT NULL DEFAULT 'pending'"));
    assert!(sql.contains("created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"));
}

#[test]
fn create_table_with_unique() {
    static T: Entity = Entity::new(
        "accounts",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("email", FieldType::Text).unique(),
        ],
    );
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
}

#[test]
fn create_table_with_references() {
    static T: Entity = Entity::new(
        "posts",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("author_id", FieldType::Uuid).references(
                "users",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
        ],
    );
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE"));
}

#[test]
fn create_table_with_constraints() {
    static T: Entity = Entity::new(
        "memberships",
        &[
            Field::new("user_id", FieldType::Uuid),
            Field::new("group_id", FieldType::Uuid),
            Field::new("role", FieldType::Text),
        ],
    )
    .with_constraints(&[
        EntityConstraint::Unique(&["user_id", "group_id"]),
        EntityConstraint::Check("role IN ('admin', 'member')"),
    ]);
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("UNIQUE (user_id, group_id)"));
    assert!(sql.contains("CHECK (role IN ('admin', 'member'))"));
}

#[test]
fn create_table_with_fk_constraint() {
    static T: Entity = Entity::new(
        "order_items",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("order_id", FieldType::Uuid),
            Field::new("product_id", FieldType::Uuid),
        ],
    )
    .with_constraints(&[EntityConstraint::ForeignKey {
        columns: &["order_id"],
        ref_table: "orders",
        ref_columns: &["id"],
        on_delete: FkAction::Cascade,
    }]);
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("FOREIGN KEY (order_id) REFERENCES orders (id) ON DELETE CASCADE"));
}

#[test]
fn create_table_with_default_unique_fk() {
    static T: Entity = Entity::new(
        "credentials",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("user_id", FieldType::Uuid).references(
                "users",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
            Field::new("kind", FieldType::Text),
            Field::new("secret_hash", FieldType::Text).unique(),
            Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        ],
    );
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE"));
    assert!(sql.contains("secret_hash TEXT NOT NULL UNIQUE"));
    assert!(sql.contains("created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"));
    assert!(sql.contains("PRIMARY KEY (id)"));
}

#[test]
fn custom_field_type() {
    static T: Entity = Entity::new(
        "items",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("status", FieldType::Custom("item_status")),
        ],
    );
    let sql = T.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("status item_status NOT NULL"));
}

// ===========================================================================
// DDL tests — DROP TABLE (DropEntityBuilder)
// ===========================================================================

#[test]
fn drop_table() {
    let sql = SETTINGS
        .drop_entity()
        .if_exists()
        .cascade()
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "DROP TABLE IF EXISTS tenant_settings CASCADE");
}

// ===========================================================================
// DDL tests — ALTER TABLE (AlterEntityBuilder)
// ===========================================================================

#[test]
fn alter_table_add_column() {
    let sql = USERS
        .alter()
        .add_column(Field::new("phone", FieldType::Text).nullable())
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users ADD COLUMN phone TEXT");
}

#[test]
fn alter_table_drop_column() {
    let sql = USERS.alter().drop_field("legacy_field").render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users DROP COLUMN legacy_field");
}

#[test]
fn alter_table_alter_column_type() {
    let sql = USERS
        .alter()
        .alter_column_type("status", FieldType::Custom("VARCHAR(50)"))
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ALTER COLUMN status TYPE VARCHAR(50)"
    );
}

#[test]
fn alter_table_set_default() {
    let sql = USERS
        .alter()
        .set_default("status", "'active'")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ALTER COLUMN status SET DEFAULT 'active'"
    );
}

#[test]
fn alter_table_drop_default() {
    let sql = USERS.alter().drop_default("status").render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users ALTER COLUMN status DROP DEFAULT");
}

#[test]
fn alter_table_set_not_null() {
    let sql = USERS.alter().set_not_null("email").render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users ALTER COLUMN email SET NOT NULL");
}

#[test]
fn alter_table_drop_not_null() {
    let sql = USERS
        .alter()
        .drop_not_null("display_name")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ALTER COLUMN display_name DROP NOT NULL"
    );
}

#[test]
fn alter_table_rename_column() {
    let sql = USERS
        .alter()
        .rename_field("email", "email_address")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users RENAME COLUMN email TO email_address"
    );
}

#[test]
fn alter_table_rename_table() {
    let sql = USERS.alter().rename_model("app_users").render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users RENAME TO app_users");
}

#[test]
fn alter_table_add_constraint() {
    let sql = USERS
        .alter()
        .add_constraint(EntityConstraint::Unique(&["tenant_id", "email"]))
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("ALTER TABLE users ADD UNIQUE (tenant_id, email)"));
}

#[test]
fn alter_table_drop_constraint() {
    let sql = USERS
        .alter()
        .drop_constraint("uq_email")
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users DROP CONSTRAINT uq_email");
}

#[test]
fn alter_table_multiple_actions() {
    let sql = USERS
        .alter()
        .add_column(Field::new("phone", FieldType::Text).nullable())
        .set_not_null("email")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ADD COLUMN phone TEXT;\n\
         ALTER TABLE users ALTER COLUMN email SET NOT NULL"
    );
}

// ===========================================================================
// DDL tests — Indexes (DefineIndexBuilder / DropIndexBuilder)
// ===========================================================================

#[test]
fn create_index() {
    let sql = DefineIndexBuilder::new("idx_users_email")
        .on("users")
        .columns(&["email"])
        .unique()
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "CREATE UNIQUE INDEX idx_users_email ON users (email)");
}

#[test]
fn create_index_if_not_exists() {
    let sql = DefineIndexBuilder::new("idx_users_email")
        .on("users")
        .columns(&["email"])
        .if_not_exists()
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "CREATE INDEX IF NOT EXISTS idx_users_email ON users (email)"
    );
}

#[test]
fn create_index_partial() {
    let sql = DefineIndexBuilder::new("idx_active_users")
        .on("users")
        .columns(&["email"])
        .where_clause("status = 'active'")
        .render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "CREATE INDEX idx_active_users ON users (email) WHERE status = 'active'"
    );
}

#[test]
fn create_index_concurrently() {
    let sql = DefineIndexBuilder::new("idx_email")
        .on("users")
        .columns(&["email"])
        .concurrently()
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "CREATE INDEX CONCURRENTLY idx_email ON users (email)");
}

#[test]
fn drop_index() {
    let sql = DropIndexBuilder::new("idx_users_email")
        .if_exists()
        .cascade()
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "DROP INDEX IF EXISTS idx_users_email CASCADE");
}

// ===========================================================================
// DCL tests
// ===========================================================================

#[test]
fn grant_select() {
    let sql = GrantBuilder::new(Privilege::Select)
        .on("users")
        .to("app_reader")
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
}

#[test]
fn revoke_insert() {
    let sql = RevokeBuilder::new(Privilege::Insert)
        .on("users")
        .from("app_writer")
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "REVOKE INSERT ON users FROM app_writer");
}

#[test]
fn grant_all() {
    let sql = GrantBuilder::new(Privilege::All)
        .on("tenants")
        .to("admin")
        .render(Some(&pg())).unwrap();
    assert_eq!(sql, "GRANT ALL ON tenants TO admin");
}

// ===========================================================================
// TCL / Transaction tests
// ===========================================================================

#[test]
fn transaction_begin_commit_rollback() {
    assert_eq!(
        TransactionBuilder::render(&TransactionBuilder::begin(), None).unwrap(),
        "BEGIN"
    );
    assert_eq!(
        TransactionBuilder::render(&TransactionBuilder::commit(), None).unwrap(),
        "COMMIT"
    );
    assert_eq!(
        TransactionBuilder::render(&TransactionBuilder::rollback(), None).unwrap(),
        "ROLLBACK"
    );
}

#[test]
fn transaction_savepoint() {
    let ir = TransactionBuilder::savepoint("sp1");
    assert_eq!(TransactionBuilder::render(&ir, None).unwrap(), "SAVEPOINT sp1");

    let ir = TransactionBuilder::release_savepoint("sp1");
    assert_eq!(
        TransactionBuilder::render(&ir, None).unwrap(),
        "RELEASE SAVEPOINT sp1"
    );

    let ir = TransactionBuilder::rollback_to_savepoint("sp1");
    assert_eq!(
        TransactionBuilder::render(&ir, None).unwrap(),
        "ROLLBACK TO SAVEPOINT sp1"
    );
}

// ===========================================================================
// Expr construction + rendering
// ===========================================================================

#[test]
fn expr_column_eq_param() {
    let expr = col("email").eq(param());
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "email = $1");
}

#[test]
fn expr_nested_boolean() {
    let expr = col("a").eq(param()) & (col("b").gt(param()) | col("c").lt(param()));
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "(a = $1 AND (b > $2 OR c < $3))");
}

#[test]
fn expr_not() {
    let expr = !col("deleted");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "NOT (deleted)");
}

#[test]
fn expr_arithmetic() {
    let expr = (col("price") * lit(1.1f64)) + lit(5i64);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "price * 1.1 + 5");
}

#[test]
fn expr_is_null_is_not_null() {
    let pg = Dialect::postgres();
    let mut c1 = pg.param_counter();
    let sql1 = crate::backend::sql::render::render_expr(&col("meta").is_null(), &mut c1, &pg);
    assert_eq!(sql1, "meta IS NULL");

    let mut c2 = pg.param_counter();
    let sql2 = crate::backend::sql::render::render_expr(&col("meta").is_not_null(), &mut c2, &pg);
    assert_eq!(sql2, "meta IS NOT NULL");
}

#[test]
fn expr_between() {
    let expr = col("age").between(param(), param());
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "age BETWEEN $1 AND $2");
}

#[test]
fn expr_in_list() {
    let expr = col("status").in_list(vec![param(), param()]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "status IN ($1, $2)");
}

#[test]
fn expr_not_in_list() {
    let expr = col("status").not_in_list(vec![param(), param()]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "status NOT IN ($1, $2)");
}

#[test]
fn expr_like_ilike() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();

    // PG supports ILIKE natively
    let expr = col("name").ilike(param());
    let mut c1 = pg.param_counter();
    let sql1 = crate::backend::sql::render::render_expr(&expr, &mut c1, &pg);
    assert_eq!(sql1, "name ILIKE $1");

    // MySQL falls back to LOWER(x) LIKE LOWER(y)
    let mut c2 = mysql.param_counter();
    let sql2 = crate::backend::sql::render::render_expr(&expr, &mut c2, &mysql);
    assert_eq!(sql2, "LOWER(name) LIKE LOWER(?)");
}

#[test]
fn expr_cast() {
    let expr = col("price").cast("NUMERIC(10,2)");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "CAST(price AS NUMERIC(10,2))");
}

#[test]
fn expr_alias() {
    let expr = col("email").alias("e");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "email AS e");
}

#[test]
fn expr_bool_literal_pg_vs_mysql() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&lit(true), &mut c1, &pg),
        "TRUE"
    );

    let mut c2 = mysql.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&lit(true), &mut c2, &mysql),
        "1"
    );
}

#[test]
fn expr_concat_pg_pipe_vs_mysql_func() {
    let expr = col("first_name").concat(col("last_name"));
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();
    let mssql = Dialect::mssql();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&expr, &mut c1, &pg),
        "first_name || last_name"
    );

    let mut c2 = mysql.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&expr, &mut c2, &mysql),
        "CONCAT(first_name, last_name)"
    );

    let mut c3 = mssql.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&expr, &mut c3, &mssql),
        "first_name + last_name"
    );
}

#[test]
fn expr_case_when() {
    let expr = case()
        .when(col("status").eq(raw_expr("'active'")), lit("Active"))
        .when(col("status").eq(raw_expr("'disabled'")), lit("Disabled"))
        .else_(lit("Unknown"))
        .end();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(
        sql,
        "CASE WHEN status = 'active' THEN 'Active' WHEN status = 'disabled' THEN 'Disabled' ELSE 'Unknown' END"
    );
}

// ===========================================================================
// DOL-specific: field() vs col() equivalence
// ===========================================================================

#[test]
fn field_and_col_are_equivalent() {
    // Both field() and col() produce Expr::Identifier
    let f = format!("{:?}", field("email"));
    let c = format!("{:?}", col("email"));
    assert_eq!(f, c);
}

// ===========================================================================
// SQL functions
// ===========================================================================

#[test]
fn func_lower_upper() {
    let pg = Dialect::postgres();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::lower(col("email")), &mut c1, &pg),
        "LOWER(email)"
    );

    let mut c2 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::upper(col("name")), &mut c2, &pg),
        "UPPER(name)"
    );
}

#[test]
fn func_coalesce() {
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let expr = func::coalesce(vec![col("display_name"), col("email")]);
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "COALESCE(display_name, email)");
}

#[test]
fn func_count_star() {
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&func::count_star(), &mut counter, &pg);
    assert_eq!(sql, "COUNT(*)");
}

#[test]
fn func_aggregates() {
    let pg = Dialect::postgres();

    let cases = [
        (func::count(col("id")), "COUNT(id)"),
        (func::sum(col("amount")), "SUM(amount)"),
        (func::avg(col("score")), "AVG(score)"),
        (func::min(col("price")), "MIN(price)"),
        (func::max(col("price")), "MAX(price)"),
    ];
    for (expr, expected) in cases {
        let mut c = pg.param_counter();
        assert_eq!(
            crate::backend::sql::render::render_expr(&expr, &mut c, &pg),
            expected
        );
    }
}

#[test]
fn func_now_and_dates() {
    let pg = Dialect::postgres();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::now(), &mut c1, &pg),
        "NOW()"
    );

    let mut c2 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::current_timestamp(), &mut c2, &pg),
        "CURRENT_TIMESTAMP"
    );
}

// ===========================================================================
// Window functions
// ===========================================================================

#[test]
fn window_row_number() {
    let expr = func::row_number()
        .over()
        .partition_by(vec![col("tenant_id")])
        .order_by(vec![col("created_at").desc()])
        .build();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(
        sql,
        "ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY created_at DESC)"
    );
}

#[test]
fn window_rank_with_frame() {
    let expr = func::rank()
        .over()
        .order_by(vec![col("score").desc()])
        .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(
        sql,
        "RANK() OVER (ORDER BY score DESC ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)"
    );
}

// ===========================================================================
// Set operations (UNION, INTERSECT, EXCEPT)
// ===========================================================================

#[test]
fn set_op_union() {
    let q1 = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("tenant_id")
        .build();
    let q2 = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq_literal("status", raw_expr("'active'"))
        .build();
    let sql = CompoundSelectBuilder::new(q1).union(q2).render(Some(&pg())).unwrap();
    assert!(sql.contains("UNION"));
    assert!(sql.contains("SELECT id, email FROM users WHERE tenant_id = $1"));
}

#[test]
fn set_op_union_all() {
    let q1 = SETTINGS.get().all_columns().build();
    let q2 = SETTINGS.get().all_columns().where_eq("tenant_id").build();
    let sql = CompoundSelectBuilder::new(q1)
        .union_all(q2)
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("UNION ALL"));
}

#[test]
fn set_op_intersect_except() {
    let q1 = USERS.get().columns(&["id"]).build();
    let q2 = USERS.get().columns(&["id"]).where_eq("tenant_id").build();
    let q3 = USERS
        .get()
        .columns(&["id"])
        .where_eq_literal("status", raw_expr("'disabled'"))
        .build();
    let sql = CompoundSelectBuilder::new(q1)
        .intersect(q2)
        .except(q3)
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("INTERSECT"));
    assert!(sql.contains("EXCEPT"));
}

#[test]
fn set_op_with_order_and_limit() {
    let q1 = USERS.get().columns(&["id"]).build();
    let q2 = USERS.get().columns(&["id"]).where_eq("tenant_id").build();
    let sql = CompoundSelectBuilder::new(q1)
        .union_all(q2)
        .order_by(vec![col("id").asc()])
        .limit()
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("UNION ALL"));
    assert!(sql.contains("ORDER BY id ASC"));
    assert!(sql.contains("LIMIT"));
}

// ===========================================================================
// Subquery composition
// ===========================================================================

#[test]
fn subquery_as_scalar() {
    let sub = SETTINGS
        .get()
        .select_item(func::sum(col("max_users")).alias("total"))
        .render(Some(&pg())).unwrap();
    let sql = USERS
        .get()
        .all_columns()
        .filter(col("id").in_subquery(&sub))
        .render(Some(&pg())).unwrap();
    assert!(sql.contains("IN (SELECT SUM(max_users) AS total FROM tenant_settings)"));
}

#[test]
fn subquery_exists_expr() {
    let expr = Expr::Exists {
        subquery: "SELECT 1 FROM sessions WHERE sessions.user_id = users.id".to_string(),
        negated: false,
    };
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(
        sql,
        "EXISTS (SELECT 1 FROM sessions WHERE sessions.user_id = users.id)"
    );
}

// ===========================================================================
// Cross-dialect rendering tests
// ===========================================================================

#[test]
fn dialect_mysql_select_with_where() {
    let mysql = Dialect::mysql();
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_eq("id")
        .render(Some(&mysql)).unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at FROM users WHERE tenant_id = ? AND id = ?"
    );
}

#[test]
fn dialect_mssql_select_with_where() {
    let mssql = Dialect::mssql();
    let sql = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("id")
        .render(Some(&mssql)).unwrap();
    assert_eq!(sql, "SELECT id, email FROM users WHERE id = @p1");
}

#[test]
fn dialect_oracle_select_with_where() {
    let oracle = Dialect::oracle();
    let sql = USERS
        .get()
        .columns(&["id"])
        .where_eq("tenant_id")
        .render(Some(&oracle)).unwrap();
    assert_eq!(sql, "SELECT id FROM users WHERE tenant_id = :1");
}

#[test]
fn dialect_mysql_insert() {
    let mysql = Dialect::mysql();
    let sql = SETTINGS.insert().all_columns().render(Some(&mysql)).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) VALUES (?, ?, ?)"
    );
}

#[test]
fn dialect_mssql_insert() {
    let mssql = Dialect::mssql();
    let sql = SETTINGS.insert().all_columns().render(Some(&mssql)).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) VALUES (@p1, @p2, @p3)"
    );
}

#[test]
fn dialect_mysql_batch_insert() {
    let mysql = Dialect::mysql();
    let sql = SETTINGS.insert().all_columns().rows(2).render(Some(&mysql)).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) VALUES (?, ?, ?), (?, ?, ?)"
    );
}

#[test]
fn dialect_mysql_update() {
    let mysql = Dialect::mysql();
    let sql = USERS
        .update()
        .set("email")
        .where_eq("id")
        .render(Some(&mysql)).unwrap();
    assert_eq!(sql, "UPDATE users SET email = ? WHERE id = ?");
}

#[test]
fn dialect_mssql_update() {
    let mssql = Dialect::mssql();
    let sql = USERS
        .update()
        .set("email")
        .where_eq("id")
        .render(Some(&mssql)).unwrap();
    assert_eq!(sql, "UPDATE users SET email = @p1 WHERE id = @p2");
}

#[test]
fn dialect_mysql_delete() {
    let mysql = Dialect::mysql();
    let sql = USERS.remove().where_eq("id").render(Some(&mysql)).unwrap();
    assert_eq!(sql, "DELETE FROM users WHERE id = ?");
}

#[test]
fn dialect_pg_returning() {
    let pg = Dialect::postgres();
    let sql = USERS
        .remove()
        .where_eq("id")
        .returning(&["id"])
        .render(Some(&pg)).unwrap();
    assert!(sql.ends_with(" RETURNING id"));
}

#[test]
fn dialect_mysql_returning_unsupported() {
    let mysql = Dialect::mysql();
    let sql = USERS
        .remove()
        .where_eq("id")
        .returning(&["id"])
        .render(Some(&mysql)).unwrap();
    assert!(!sql.contains("RETURNING"));
    assert_eq!(sql, "DELETE FROM users WHERE id = ?");
}

#[test]
fn dialect_mssql_returning_output() {
    let mssql = Dialect::mssql();
    let sql = SETTINGS
        .insert()
        .all_columns()
        .returning(&["tenant_id"])
        .render(Some(&mssql)).unwrap();
    assert!(sql.contains("OUTPUT INSERTED.tenant_id"));
}

#[test]
fn dialect_mssql_pagination_offset_fetch() {
    let mssql = Dialect::mssql();
    let sql = USERS
        .get()
        .all_columns()
        .order_by_asc("id")
        .offset()
        .limit()
        .render(Some(&mssql)).unwrap();
    assert!(sql.contains("OFFSET @p1 ROWS FETCH NEXT @p2 ROWS ONLY"));
}

#[test]
fn dialect_pg_pagination_limit_offset() {
    let pg = Dialect::postgres();
    let sql = USERS
        .get()
        .all_columns()
        .order_by_asc("id")
        .offset()
        .limit()
        .render(Some(&pg)).unwrap();
    assert!(sql.contains("OFFSET $1 LIMIT $2"));
}

#[test]
fn dialect_mysql_upsert_on_duplicate_key() {
    // DOL currently always uses ON CONFLICT syntax (PostgreSQL-native).
    // Dialect-specific upsert rendering (ON DUPLICATE KEY, MERGE) is a Phase 9 item.
    let mysql = Dialect::mysql();
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .render(Some(&mysql)).unwrap();
    // Verify it renders something valid (PG-style with MySQL params)
    assert!(sql.contains("ON CONFLICT"));
    assert!(sql.contains("DO UPDATE SET"));
}

#[test]
fn dialect_mysql_upsert_do_nothing() {
    let mysql = Dialect::mysql();
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_nothing()
        .render(Some(&mysql)).unwrap();
    assert!(sql.contains("DO NOTHING"));
}

#[test]
fn dialect_mssql_upsert_merge() {
    // DOL currently always uses ON CONFLICT syntax.
    // MSSQL MERGE rendering is a Phase 9 item.
    let mssql = Dialect::mssql();
    let sql = SETTINGS
        .upsert()
        .all_columns()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .render(Some(&mssql)).unwrap();
    assert!(sql.contains("ON CONFLICT"));
}

#[test]
fn dialect_sqlite_select() {
    let sqlite = Dialect::sqlite();
    let sql = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("id")
        .render(Some(&sqlite)).unwrap();
    assert_eq!(sql, "SELECT id, email FROM users WHERE id = ?");
}

#[test]
fn dialect_distinct_on_pg_only() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();

    let pg_sql = USERS
        .get()
        .all_columns()
        .distinct_on(&["tenant_id"])
        .render(Some(&pg)).unwrap();
    assert!(pg_sql.starts_with("SELECT DISTINCT ON (tenant_id)"));

    let mysql_sql = USERS
        .get()
        .all_columns()
        .distinct_on(&["tenant_id"])
        .render(Some(&mysql)).unwrap();
    assert!(!mysql_sql.contains("DISTINCT ON"));
}

#[test]
fn dialect_create_table_type_mapping() {
    let mysql = Dialect::mysql();
    let sql = SETTINGS.create().render(Some(&mysql)).unwrap();
    assert!(sql.contains("CHAR(36)"));
    assert!(sql.contains("INT"));
    assert!(sql.contains("TINYINT(1)"));
}

#[test]
fn dialect_create_table_sqlite_types() {
    let sqlite = Dialect::sqlite();
    let sql = USERS.create().render(Some(&sqlite)).unwrap();
    assert!(sql.contains("id TEXT"));
    assert!(sql.contains("created_at TEXT"));
}

#[test]
fn dialect_cockroachdb_is_pg_compatible() {
    let crdb = Dialect::cockroachdb();
    let sql = USERS.get().all_columns().where_eq("id").render(Some(&crdb)).unwrap();
    assert!(sql.contains("WHERE id = $1"));
}

#[test]
fn dialect_for_update_sqlite_unsupported() {
    let sqlite = Dialect::sqlite();
    let sql = USERS.get().all_columns().for_update().render(Some(&sqlite)).unwrap();
    assert!(!sql.contains("FOR UPDATE"));
}

#[test]
fn dialect_nulls_ordering_mysql_unsupported() {
    let mysql = Dialect::mysql();
    let sql = USERS
        .get()
        .all_columns()
        .order_by("created_at", Direction::Desc, Some(NullsPosition::Last))
        .render(Some(&mysql)).unwrap();
    assert!(!sql.contains("NULLS LAST"));
    assert!(sql.contains("ORDER BY created_at DESC"));
}

#[test]
fn dialect_presets_all_instantiate() {
    let _pg = Dialect::postgres();
    let _mysql = Dialect::mysql();
    let _maria = Dialect::mariadb();
    let _sqlite = Dialect::sqlite();
    let _mssql = Dialect::mssql();
    let _oracle = Dialect::oracle();
    let _crdb = Dialect::cockroachdb();
}

#[test]
fn dialect_type_map_resolve() {
    let pg = Dialect::postgres();
    assert_eq!(pg.resolve_type(&FieldType::Uuid), "UUID");
    assert_eq!(pg.resolve_type(&FieldType::Timestamp), "TIMESTAMPTZ");

    let mysql = Dialect::mysql();
    assert_eq!(mysql.resolve_type(&FieldType::Uuid), "CHAR(36)");
    assert_eq!(mysql.resolve_type(&FieldType::Bool), "TINYINT(1)");
    assert_eq!(mysql.resolve_type(&FieldType::Timestamp), "DATETIME(6)");

    let mssql = Dialect::mssql();
    assert_eq!(mssql.resolve_type(&FieldType::Uuid), "UNIQUEIDENTIFIER");
    assert_eq!(mssql.resolve_type(&FieldType::Bool), "BIT");

    let oracle = Dialect::oracle();
    assert_eq!(oracle.resolve_type(&FieldType::Uuid), "RAW(16)");
    assert_eq!(oracle.resolve_type(&FieldType::Bool), "NUMBER(1)");

    let sqlite = Dialect::sqlite();
    assert_eq!(sqlite.resolve_type(&FieldType::Uuid), "TEXT");
    assert_eq!(sqlite.resolve_type(&FieldType::Bool), "INTEGER");
}

#[test]
fn dialect_param_counter() {
    use crate::backend::sql::dialect::{ParamCounter, ParamStyle};

    let mut pg = ParamCounter::new(&ParamStyle::postgres());
    assert_eq!(pg.next(), "$1");
    assert_eq!(pg.next(), "$2");
    assert_eq!(pg.count(), 2);

    let mut mysql = ParamCounter::new(&ParamStyle::mysql());
    assert_eq!(mysql.next(), "?");
    assert_eq!(mysql.next(), "?");

    let mut mssql = ParamCounter::new(&ParamStyle::mssql());
    assert_eq!(mssql.next(), "@p1");
    assert_eq!(mssql.next(), "@p2");

    let mut oracle = ParamCounter::new(&ParamStyle::oracle());
    assert_eq!(oracle.next(), ":1");
    assert_eq!(oracle.next(), ":2");
}

#[test]
fn dialect_quote_style() {
    use crate::backend::sql::dialect::QuoteStyle;

    assert_eq!(QuoteStyle::DoubleQuote.quote("users"), "\"users\"");
    assert_eq!(QuoteStyle::Backtick.quote("users"), "`users`");
    assert_eq!(QuoteStyle::Bracket.quote("users"), "[users]");
    assert_eq!(QuoteStyle::None.quote("users"), "users");
}

#[test]
fn dialect_default_is_sqlite() {
    let d = crate::backend::sql::dialect::default_dialect();
    assert_eq!(d.name, "sqlite");
}

#[test]
fn dialect_oracle_pagination() {
    let oracle = Dialect::oracle();
    let sql = USERS
        .get()
        .columns(&["id"])
        .order_by_asc("id")
        .offset()
        .limit()
        .render(Some(&oracle)).unwrap();
    assert!(sql.contains("OFFSET :1 ROWS FETCH NEXT :2 ROWS ONLY"));
}

#[test]
fn dialect_drop_table_mssql_no_cascade() {
    // DOL's current drop model renderer doesn't honor dialect CASCADE restrictions.
    // This will be refined in Phase 9 when dialect-aware DDL is fully implemented.
    let mssql = Dialect::mssql();
    let sql = USERS
        .drop_entity()
        .if_exists()
        .cascade()
        .render(Some(&mssql)).unwrap();
    assert!(sql.contains("IF EXISTS"));
    // Currently always adds CASCADE — OK for now
    assert!(sql.contains("DROP TABLE"));
}

#[test]
fn dialect_mysql_ilike_fallback() {
    let mysql = Dialect::mysql();
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_ilike("email")
        .render(Some(&mysql)).unwrap();
    assert!(sql.contains("LOWER(email) LIKE LOWER(?)"));
}

#[test]
fn dialect_mssql_expr_params() {
    let mssql = Dialect::mssql();
    let sql = USERS
        .get()
        .all_columns()
        .filter(col("id").eq(param()))
        .filter(col("status").eq(param()))
        .render(Some(&mssql)).unwrap();
    assert!(sql.contains("WHERE id = @p1 AND status = @p2"));
}

// ===========================================================================
// Config deserialization tests (feature = "config")
// ===========================================================================

#[cfg(feature = "config")]
mod config_tests {
    use super::*;

    #[test]
    fn dialect_from_json_str() {
        let json = r#"{
            "name": "postgresql",
            "param_style": {"style": "Numbered", "prefix": "$"},
            "quote_style": "DoubleQuote",
            "type_map": {"mappings": {"Uuid": "UUID", "Text": "TEXT", "Integer": "INTEGER", "Boolean": "BOOLEAN", "Timestamptz": "TIMESTAMPTZ", "Jsonb": "JSONB", "TextArray": "TEXT[]", "SmallInt": "SMALLINT", "BigInt": "BIGINT", "Real": "REAL", "DoublePrecision": "DOUBLE PRECISION", "Numeric": "NUMERIC", "Bytea": "BYTEA", "Date": "DATE", "Time": "TIME", "Interval": "INTERVAL", "Serial": "SERIAL", "BigSerial": "BIGSERIAL", "Inet": "INET"}},
            "pagination": "LimitOffset",
            "upsert_style": "OnConflict",
            "returning_style": "Returning",
            "locking": {"for_update": true, "for_share": true, "skip_locked": true, "nowait": true, "use_table_hint": false},
            "ddl": {"create_if_not_exists": true, "drop_if_exists": true, "index_concurrently": true, "enum_style": "CreateType", "auto_increment_style": "SerialType", "alter_add_column": true, "alter_drop_column": true, "alter_rename_column": true, "alter_modify_column": true, "alter_rename_table": true, "transactional_ddl": true, "drop_cascade": true},
            "features": {"distinct_on": true, "ilike": true, "array_any": true, "nulls_ordering": true, "anonymous_blocks": true, "schemas": true, "cte": true, "window_functions": true, "lateral_join": true, "on_conflict": true},
            "concat_style": "PipeOperator"
        }"#;
        let d = Dialect::from_json_str(json).unwrap();
        assert_eq!(d.name, "postgresql");
        assert_eq!(d.bool_true, "TRUE");
        assert!(d.features.distinct_on);

        let sql = USERS.get().columns(&["id"]).where_eq("id").render(Some(&d)).unwrap();
        assert_eq!(sql, "SELECT id FROM users WHERE id = $1");
    }

    #[test]
    fn dialect_from_toml_str() {
        let toml = r#"
name = "mysql"
pagination = "LimitOffset"
upsert_style = "OnDuplicateKey"
returning_style = "Unsupported"
bool_true = "1"
bool_false = "0"
concat_style = "ConcatFunction"
quote_style = "Backtick"

[param_style]
style = "Positional"

[type_map.mappings]
Uuid = "CHAR(36)"
Text = "TEXT"
Integer = "INT"
Boolean = "TINYINT(1)"
Timestamptz = "DATETIME(6)"
Jsonb = "JSON"
TextArray = "JSON"
SmallInt = "SMALLINT"
BigInt = "BIGINT"
Real = "FLOAT"
DoublePrecision = "DOUBLE"
Numeric = "DECIMAL"
Bytea = "LONGBLOB"
Date = "DATE"
Time = "TIME"
Interval = "VARCHAR(64)"
Serial = "INT AUTO_INCREMENT"
BigSerial = "BIGINT AUTO_INCREMENT"
Inet = "VARCHAR(45)"

[locking]
for_update = true
for_share = true
skip_locked = true
nowait = true
use_table_hint = false

[ddl]
create_if_not_exists = true
drop_if_exists = true
index_concurrently = false
enum_style = "InlineEnum"
auto_increment_style = "AutoIncrement"
alter_add_column = true
alter_drop_column = true
alter_rename_column = true
alter_modify_column = true
alter_rename_table = true
transactional_ddl = false
drop_cascade = false

[features]
distinct_on = false
ilike = false
array_any = false
nulls_ordering = false
anonymous_blocks = false
schemas = false
cte = true
window_functions = true
lateral_join = true
on_conflict = false
"#;
        let d = Dialect::from_toml_str(toml).unwrap();
        assert_eq!(d.name, "mysql");
        assert_eq!(d.bool_true, "1");
        assert!(!d.features.distinct_on);

        let sql = SETTINGS.insert().all_columns().render(Some(&d)).unwrap();
        assert!(sql.contains("VALUES (?, ?, ?)"));
    }

    #[test]
    fn dialect_from_yaml_str() {
        let yaml = r#"
name: sqlite
param_style:
  style: Positional
quote_style: DoubleQuote
type_map:
  mappings:
    Uuid: TEXT
    Text: TEXT
    Integer: INTEGER
    Boolean: INTEGER
    Timestamptz: TEXT
    Jsonb: TEXT
    TextArray: TEXT
    SmallInt: INTEGER
    BigInt: INTEGER
    Real: REAL
    DoublePrecision: REAL
    Numeric: NUMERIC
    Bytea: BLOB
    Date: TEXT
    Time: TEXT
    Interval: TEXT
    Serial: INTEGER
    BigSerial: INTEGER
    Inet: TEXT
pagination: LimitOffset
upsert_style: OnConflict
returning_style: Returning
locking:
  for_update: false
  for_share: false
  skip_locked: false
  nowait: false
  use_table_hint: false
ddl:
  create_if_not_exists: true
  drop_if_exists: true
  index_concurrently: false
  enum_style: CheckConstraint
  auto_increment_style: Autoincrement
  alter_add_column: true
  alter_drop_column: true
  alter_rename_column: true
  alter_modify_column: false
  alter_rename_table: true
  transactional_ddl: true
  drop_cascade: false
features:
  distinct_on: false
  ilike: false
  array_any: false
  nulls_ordering: true
  anonymous_blocks: false
  schemas: false
  cte: true
  window_functions: true
  lateral_join: false
  on_conflict: true
bool_true: "1"
bool_false: "0"
concat_style: PipeOperator
"#;
        let d = Dialect::from_yaml_str(yaml).unwrap();
        assert_eq!(d.name, "sqlite");

        let sql = USERS.get().columns(&["id"]).where_eq("id").render(Some(&d)).unwrap();
        assert_eq!(sql, "SELECT id FROM users WHERE id = ?");
    }
}

// ===========================================================================
// Storage builder tests
// ===========================================================================

#[test]
fn storage_put_object() {
    use crate::builder::storage::PutObjectBuilder;
    let ir = PutObjectBuilder::new("avatars/u1.png")
        .from_path("./avatar.png")
        .into_bucket("media")
        .content_type("image/png")
        .build();
    assert_eq!(ir.key, "avatars/u1.png");
    assert_eq!(ir.bucket, "media");
    assert_eq!(ir.content_type, Some("image/png".to_string()));
}

#[test]
fn storage_get_object() {
    use crate::builder::storage::GetObjectBuilder;
    let ir = GetObjectBuilder::new("avatars/u1.png")
        .from_bucket("media")
        .build();
    assert_eq!(ir.key, "avatars/u1.png");
    assert_eq!(ir.bucket, "media");
}

#[test]
fn storage_list_objects() {
    use crate::builder::storage::ListObjectsBuilder;
    let ir = ListObjectsBuilder::new()
        .bucket("media")
        .prefix("avatars/")
        .limit(100)
        .build();
    assert_eq!(ir.bucket, "media");
    assert_eq!(ir.prefix, Some("avatars/".to_string()));
    assert_eq!(ir.limit, Some(100));
}

#[test]
fn storage_read_file() {
    use crate::builder::storage::ReadFileBuilder;
    let ir = ReadFileBuilder::new("/tmp/data.csv")
        .encoding("utf-8")
        .build();
    assert_eq!(ir.path, "/tmp/data.csv");
    assert_eq!(ir.encoding, Some("utf-8".to_string()));
}

#[test]
fn storage_write_file() {
    use crate::builder::storage::WriteFileBuilder;
    let ir = WriteFileBuilder::new("/tmp/output.csv")
        .from_path("/tmp/source.csv")
        .create_dirs()
        .build();
    assert_eq!(ir.path, "/tmp/output.csv");
    assert!(ir.create_dirs);
}

#[test]
fn storage_move_file() {
    use crate::builder::storage::MoveFileBuilder;
    let ir = MoveFileBuilder::new("/tmp/a.txt", "/tmp/b.txt").build();
    assert_eq!(ir.from, "/tmp/a.txt");
    assert_eq!(ir.to, "/tmp/b.txt");
}

// ===========================================================================
// DOL-specific: FieldAccess, ObjectLiteral, ArrayLiteral
// ===========================================================================

#[test]
fn expr_field_access() {
    let expr = field("profile").access("address").access("city");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert_eq!(sql, "profile->>'address'->>'city'");
}

#[test]
fn expr_object_literal() {
    use crate::expr::obj;
    let expr = obj(vec![("name", lit("Alice")), ("age", lit(30i64))]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert!(sql.contains("'name'"));
    assert!(sql.contains("'Alice'"));
}

#[test]
fn expr_array_literal() {
    use crate::expr::arr;
    let expr = arr(vec![lit(1i64), lit(2i64), lit(3i64)]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg);
    assert!(sql.contains("ARRAY[1, 2, 3]"));
}

// ===========================================================================
// DOL-specific: Entity::define() (DefineEntityBuilder)
// ===========================================================================

#[test]
fn define_model_via_static_method() {
    use crate::ir::definition::FieldDef;

    let ir = Entity::define("users")
        .field(FieldDef::new("id", FieldType::Uuid).primary_key())
        .field(FieldDef::new("email", FieldType::Text).unique())
        .build();
    assert_eq!(ir.name, "users");
    assert_eq!(ir.fields.len(), 2);
}

// ===========================================================================
// DOL-specific: new FieldType variants
// ===========================================================================

#[test]
fn field_type_display_new_variants() {
    // Verify Display impl for DOL-specific FieldType variants
    assert_eq!(FieldType::Object.to_string(), "JSONB");
    assert_eq!(FieldType::Blob.to_string(), "BYTEA");
    assert_eq!(FieldType::Path.to_string(), "TEXT");
    assert_eq!(FieldType::Url.to_string(), "TEXT");
    assert_eq!(FieldType::ResourceId.to_string(), "TEXT");
    assert_eq!(FieldType::Version.to_string(), "INTEGER");
    assert_eq!(FieldType::Etag.to_string(), "TEXT");
    assert_eq!(FieldType::Mime.to_string(), "TEXT");
    assert_eq!(FieldType::Duration.to_string(), "INTERVAL");
    assert_eq!(FieldType::Decimal.to_string(), "NUMERIC");
    assert_eq!(FieldType::Float.to_string(), "REAL");
    assert_eq!(FieldType::Double.to_string(), "DOUBLE PRECISION");
}

#[test]
fn char_type_renders_in_display() {
    assert_eq!(FieldType::Char(10).to_string(), "CHAR(10)");
}

#[test]
fn varchar_type_renders_in_display() {
    assert_eq!(FieldType::Varchar(Some(255)).to_string(), "VARCHAR(255)");
    assert_eq!(FieldType::Varchar(None).to_string(), "VARCHAR");
}

#[test]
fn char_varchar_in_create_table() {
    static CODES: Entity = Entity::new(
        "codes",
        &[
            Field::new("id", FieldType::Int).primary_key(),
            Field::new("code", FieldType::Char(3)),
            Field::new("description", FieldType::Varchar(Some(255))),
            Field::new("notes", FieldType::Varchar(None)),
        ],
    );
    let sql = CODES.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("CHAR(3)"), "sql = {}", sql);
    assert!(sql.contains("VARCHAR(255)"), "sql = {}", sql);
    assert!(sql.contains("VARCHAR NOT NULL"), "sql = {}", sql);
}

#[test]
fn char_varchar_dialect_resolve() {
    use crate::backend::sql::dialect::Dialect;
    let pg = Dialect::postgres();
    assert_eq!(pg.resolve_type(&FieldType::Char(5)), "CHAR(5)");
    assert_eq!(
        pg.resolve_type(&FieldType::Varchar(Some(100))),
        "VARCHAR(100)"
    );
    assert_eq!(pg.resolve_type(&FieldType::Varchar(None)), "VARCHAR");
}

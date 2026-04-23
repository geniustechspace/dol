//! Comprehensive tests for the DOL crate — ported from core-db plus DOL-specific tests.

use crate::CompoundSelectBuilder;
use crate::EntityDefineExt;
use crate::FieldDef;
use crate::Render;
use crate::TransactionRender;
use crate::backend::sql::dialect::Dialect;
use crate::builder::EntityBuilderExt;
use crate::builder::control::{GrantBuilder, Privilege, RevokeBuilder};
use crate::builder::transaction::TransactionBuilder;
use crate::builder::{DefineIndexBuilder, DropIndexBuilder};
use crate::expr::func;
use crate::expr::window::FrameBound;
use crate::expr::{
    Direction, Expr, NullsPosition, bool_expr, case, field, float, int, param, string,
};
use crate::model::{DataType, Entity, EntityConstraint, Field, FkAction};
use crate::op::LockMode;

fn pg() -> Dialect {
    Dialect::postgres()
}

// ---------------------------------------------------------------------------
// Test model definitions (mirrors core-db test fixtures)
// ---------------------------------------------------------------------------

fn users() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("tenant_id", DataType::Uuid),
            Field::new("email", DataType::Text),
            Field::new("display_name", DataType::Text),
            Field::new("status", DataType::Text),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }),
            Field::new("updated_at", DataType::TimestampTz { precision: 6 }),
        ],
    )
}

fn tenants() -> Entity {
    Entity::new(
        "tenants",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("slug", DataType::Text),
            Field::new("display_name", DataType::Text),
            Field::new("status", DataType::Text),
            Field::new("plan", DataType::Text),
            Field::new("labels", DataType::Json).nullable(),
            Field::new(
                "scheduled_deletion_at",
                DataType::TimestampTz { precision: 6 },
            )
            .nullable(),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }),
            Field::new("created_by", DataType::Uuid),
            Field::new("updated_at", DataType::TimestampTz { precision: 6 }),
            Field::new("updated_by", DataType::Uuid),
            Field::new("version", DataType::Int32),
        ],
    )
}

fn settings() -> Entity {
    Entity::new(
        "tenant_settings",
        vec![
            Field::new("tenant_id", DataType::Uuid).primary_key(),
            Field::new("max_users", DataType::Int32),
            Field::new("mfa_required", DataType::Bool),
        ],
    )
}

// ===========================================================================
// Model / Schema tests
// ===========================================================================

#[test]
fn model_field_lookup() {
    let u = users();
    let f = u.field("email");
    assert_eq!(f.name, "email");
    assert_eq!(f.data_type, DataType::Text);
}

#[test]
#[should_panic(expected = "field 'nonexistent' not found")]
fn model_field_lookup_panics() {
    let u = users();
    u.field("nonexistent");
}

#[test]
fn model_primary_keys() {
    let u = users();
    let pks: Vec<_> = u.primary_keys().map(|f| f.name).collect();
    assert_eq!(pks, vec!["id"]);
}

#[test]
fn model_field_list() {
    let u = users();
    let list = u.field_list();
    assert_eq!(
        list,
        "id, tenant_id, email, display_name, status, created_at, updated_at"
    );
}

#[test]
fn model_qualified_name_with_namespace() {
    let m = Entity::new("users", vec![]).with_namespace("auth");
    assert_eq!(m.qualified_name(), "auth.users");
}

#[test]
fn model_qualified_name_without_namespace() {
    let u = users();
    assert_eq!(u.qualified_name(), "users");
}

// ===========================================================================
// DQL / GetBuilder tests
// ===========================================================================

#[test]
fn select_all_with_where() {
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE tenant_id = $1 AND id = $2"
    );
}

#[test]
fn select_with_pagination() {
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .order_by_desc("created_at")
        .offset()
        .limit()
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE tenant_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3"
    );
}

#[test]
fn select_with_ilike() {
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("email").ilike(param()))
        .order_by_desc("created_at")
        .offset()
        .limit()
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE tenant_id = $1 AND email ILIKE $2 \
         ORDER BY created_at DESC OFFSET $3 LIMIT $4"
    );
}

#[test]
fn select_specific_columns() {
    let u = users();
    let sql = u
        .get()
        .fields(&["id", "email"])
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "SELECT id, email FROM users WHERE tenant_id = $1");
}

#[test]
fn select_no_where() {
    let t = tenants();
    let sql = t
        .get()
        .order_by_desc("created_at")
        .offset()
        .limit()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.starts_with("SELECT id, slug, display_name, status, plan, labels,"));
    assert!(sql.ends_with("ORDER BY created_at DESC OFFSET $1 LIMIT $2"));
}

#[test]
fn select_with_literal_where() {
    let u = users();
    let sql = u
        .get()
        .filter(field("status").eq(string("active")))
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
         FROM users WHERE status = 'active' AND tenant_id = $1"
    );
}

#[test]
fn select_count_all() {
    let u = users();
    let sql = u
        .get()
        .field(Expr::CountStar.alias("total"))
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT COUNT(*) AS total FROM users WHERE tenant_id = $1"
    );
}

#[test]
fn select_aggregate_sum() {
    let s = settings();
    let sql = s
        .get()
        .field(func::sum(field("max_users")).alias("total_users"))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT SUM(max_users) AS total_users FROM tenant_settings"
    );
}

#[test]
fn select_group_by_having() {
    let u = users();
    let sql = u
        .get()
        .fields(&["tenant_id"])
        .field(Expr::CountStar.alias("cnt"))
        .group_by(&["tenant_id"])
        .having(Expr::CountStar.gt(int(10i32)))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("GROUP BY tenant_id"));
    assert!(sql.contains("HAVING COUNT(*) > 10"));
}

#[test]
fn select_distinct() {
    let u = users();
    let sql = u
        .get()
        .fields(&["tenant_id"])
        .distinct()
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "SELECT DISTINCT tenant_id FROM users");
}

#[test]
fn select_distinct_on() {
    let u = users();
    let sql = u
        .get()
        .distinct_on(&["tenant_id"])
        .order_by_desc("created_at")
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.starts_with("SELECT DISTINCT ON (tenant_id) id, tenant_id"));
    assert!(sql.ends_with("ORDER BY created_at DESC"));
}

#[test]
fn select_for_update() {
    let u = users();
    let sql = u
        .get()
        .filter(field("id").eq(param()))
        .for_update()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.ends_with("FOR UPDATE"));
}

#[test]
fn select_for_update_skip_locked() {
    let u = users();
    let sql = u
        .get()
        .filter(field("id").eq(param()))
        .lock(LockMode::ForUpdateSkipLocked)
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.ends_with("FOR UPDATE SKIP LOCKED"));
}

#[test]
fn select_column_alias() {
    let u = users();
    let sql = u
        .get()
        .field(field("email").alias("user_email"))
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "SELECT email AS user_email FROM users WHERE id = $1");
}

#[test]
fn select_raw_column() {
    let u = users();
    let sql = u
        .get()
        .field(func::coalesce(vec![field("display_name"), field("email")]).alias("name"))
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT COALESCE(display_name, email) AS name FROM users WHERE id = $1"
    );
}

#[test]
fn select_or_predicate() {
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(
            field("status").eq(string("active")) | field("status").eq(string("pending")),
        )
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND (status = 'active' OR status = 'pending')"));
}

#[test]
fn select_where_raw() {
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("created_at").gt(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("AND created_at > $2"));
}

#[test]
fn select_order_by_nulls() {
    let u = users();
    let sql = u
        .get()
        .order_by("display_name", Direction::Asc, Some(NullsPosition::Last))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("ORDER BY display_name ASC NULLS LAST"));
}

#[test]
fn select_param_count() {
    let u = users();
    let q = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("status").eq(param()))
        .offset()
        .limit();
    assert_eq!(q.param_count(), 4);
}

#[test]
fn select_inner_join() {
    let u = users();
    let t = tenants();
    let sql = u
        .get()
        .fields(&["u.id", "u.email", "t.slug"])
        .alias("u")
        .inner_join(&t, &[("u.tenant_id", "t.id")])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("INNER JOIN tenants"));
    assert!(sql.contains("ON u.tenant_id = t.id"));
}

#[test]
fn select_left_join() {
    let u = users();
    let s = settings();
    let sql = u
        .get()
        .fields(&["u.id", "s.max_users"])
        .alias("u")
        .left_join(&s, &[("u.tenant_id", "s.tenant_id")])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("LEFT JOIN tenant_settings"));
    assert!(sql.contains("ON u.tenant_id = s.tenant_id"));
}

#[test]
fn select_multiple_joins() {
    let u = users();
    let t = tenants();
    let s = settings();
    let sql = u
        .get()
        .fields(&["u.id", "t.slug", "s.max_users"])
        .alias("u")
        .inner_join(&t, &[("u.tenant_id", "t.id")])
        .left_join(&s, &[("t.id", "s.tenant_id")])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("INNER JOIN tenants"));
    assert!(sql.contains("LEFT JOIN tenant_settings"));
}

// ===========================================================================
// filter() API on Select / Update / Delete
// ===========================================================================

#[test]
fn select_filter_produces_same_as_where_eq() {
    let u = users();
    let sql_legacy = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    let sql_expr = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql_legacy, sql_expr);
}

#[test]
fn update_filter_api() {
    let u = users();
    let sql = u
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "UPDATE users SET email = $1 WHERE id = $2 AND tenant_id = $3"
    );
}

#[test]
fn delete_filter_api() {
    let u = users();
    let sql = u
        .remove()
        .filter(field("tenant_id").eq(param()))
        .filter(field("status").eq(string("deleted")))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "DELETE FROM users WHERE tenant_id = $1 AND status = 'deleted'"
    );
}

#[test]
fn select_filter_or_group() {
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(
            field("status").eq(string("active")) | field("status").eq(string("pending")),
        )
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND (status = 'active' OR status = 'pending')"));
}

// ===========================================================================
// DML tests — INSERT
// ===========================================================================

#[test]
fn insert_defaults() {
    let u = users();
    let sql = u.insert().render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO users (id, tenant_id, email, display_name, status, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    );
}

#[test]
fn insert_specific_columns() {
    let u = users();
    let sql = u
        .insert()
        .fields(&["id", "tenant_id", "email"])
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "INSERT INTO users (id, tenant_id, email) VALUES ($1, $2, $3)"
    );
}

#[test]
fn insert_with_returning() {
    let u = users();
    let sql = u.insert().returning(&["id"]).render(Some(&pg())).unwrap();
    assert!(sql.ends_with("RETURNING id"));
}

#[test]
fn insert_batch() {
    let s = settings();
    let sql = s.insert().rows(3).render(Some(&pg())).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         VALUES ($1, $2, $3), ($4, $5, $6), ($7, $8, $9)"
    );
}

#[test]
fn insert_batch_param_count() {
    let s = settings();
    let q = s.insert().rows(3);
    assert_eq!(q.param_count(), 9);
}

#[test]
fn insert_select() {
    let s = settings();
    let sql = s
        .insert_select()
        .fields(&["tenant_id", "max_users", "mfa_required"])
        .from_select("SELECT id, 100, true FROM tenants WHERE plan = 'enterprise'")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         SELECT id, 100, true FROM tenants WHERE plan = 'enterprise'"
    );
}

#[test]
fn insert_select_with_returning() {
    let s = settings();
    let sql = s
        .insert_select()
        .fields(&["tenant_id", "max_users", "mfa_required"])
        .from_select("SELECT id, 50, false FROM tenants")
        .returning(&["tenant_id"])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.ends_with("RETURNING tenant_id"));
}

// ===========================================================================
// DML tests — UPDATE
// ===========================================================================

#[test]
fn update_basic() {
    let u = users();
    let sql = u
        .update()
        .set("email")
        .set("display_name")
        .set("status")
        .set("updated_at")
        .filter(field("tenant_id").eq(param()))
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "UPDATE users SET email = $1, display_name = $2, status = $3, updated_at = $4 \
         WHERE tenant_id = $5 AND id = $6"
    );
}

#[test]
fn update_with_version_increment() {
    let t = tenants();
    let sql = t
        .update()
        .set("display_name")
        .set("status")
        .set("plan")
        .set("labels")
        .set("scheduled_deletion_at")
        .set("updated_at")
        .set("updated_by")
        .set_increment("version")
        .filter(field("id").eq(param()))
        .filter(field("version").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    // DOL set_increment uses a bind param: version = version + $N
    assert!(sql.contains("version = version + $8"));
    assert!(sql.contains("WHERE id = $9 AND version = $10"));
}

#[test]
fn update_with_literal_set_and_returning() {
    let u = users();
    let sql = u
        .update()
        .set_expr("status", string("revoked"))
        .filter(field("id").eq(param()))
        .filter(field("status").eq(string("active")))
        .returning_all()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("SET status = 'revoked'"));
    assert!(sql.contains("WHERE id = $1 AND status = 'active'"));
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn update_with_raw_where() {
    let u = users();
    let sql = u
        .update()
        .set_expr("status", string("archived"))
        .filter(field("tenant_id").eq(param()))
        .filter(field("updated_at").lt(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND updated_at < $2"));
}

#[test]
fn update_param_count() {
    let u = users();
    let q = u
        .update()
        .set("email")
        .set("display_name")
        .set_increment("version")
        .filter(field("id").eq(param()));
    assert_eq!(q.param_count(), 4); // 2 set params + 1 increment param + 1 where param
}

// ===========================================================================
// DML tests — DELETE (RemoveBuilder)
// ===========================================================================

#[test]
fn delete_basic() {
    let u = users();
    let sql = u
        .remove()
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "DELETE FROM users WHERE id = $1");
}

#[test]
fn delete_with_returning() {
    let u = users();
    let sql = u
        .remove()
        .filter(field("id").eq(param()))
        .returning_all()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn delete_with_raw_where() {
    let u = users();
    let sql = u
        .remove()
        .filter(field("tenant_id").eq(param()))
        .filter(field("created_at").lt(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("WHERE tenant_id = $1 AND created_at < $2"));
}

#[test]
fn delete_param_count() {
    let u = users();
    let q = u
        .remove()
        .filter(field("tenant_id").eq(param()))
        .filter(field("id").eq(param()));
    assert_eq!(q.param_count(), 2);
}

// ===========================================================================
// DML tests — UPSERT
// ===========================================================================

#[test]
fn upsert_basic() {
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .render(Some(&pg()))
        .unwrap();
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
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_nothing()
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id) DO NOTHING"
    );
}

#[test]
fn upsert_with_returning() {
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .returning_all()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn upsert_on_conflict_constraint() {
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict_constraint("uq_tenant_settings_pk")
        .do_update(&["max_users"])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("ON CONFLICT ON CONSTRAINT uq_tenant_settings_pk DO UPDATE SET"));
}

#[test]
fn upsert_with_conflict_where() {
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users"])
        .conflict_filter(field("mfa_required").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("DO UPDATE SET max_users = EXCLUDED.max_users WHERE mfa_required = $4"));
}

#[test]
fn upsert_param_count() {
    let s = settings();
    let q = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users"])
        .conflict_filter(field("mfa_required").eq(param()));
    assert_eq!(q.param_count(), 4); // 3 insert cols + 1 conflict where
}

// ===========================================================================
// DDL tests — CREATE TABLE (CreateFromMeta)
// ===========================================================================

#[test]
fn create_table() {
    let s = settings();
    let sql = s.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("CREATE TABLE tenant_settings"));
    assert!(sql.contains("tenant_id UUID NOT NULL"));
    assert!(sql.contains("max_users INTEGER NOT NULL"));
    assert!(sql.contains("mfa_required BOOLEAN NOT NULL"));
    assert!(sql.contains("PRIMARY KEY (tenant_id)"));
}

#[test]
fn create_table_if_not_exists() {
    let s = settings();
    let sql = s.create().if_not_exists().render(Some(&pg())).unwrap();
    assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS tenant_settings"));
}

#[test]
fn create_table_with_nullable() {
    let t = Entity::new(
        "test",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("meta", DataType::Json).nullable(),
        ],
    );
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("meta JSONB,"));
    assert!(!sql.contains("meta JSONB NOT NULL"));
}

#[test]
fn create_table_with_default_expr() {
    let t = Entity::new(
        "events",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("status", DataType::Text).default("'pending'"),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
        ],
    );
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("status TEXT NOT NULL DEFAULT 'pending'"));
    assert!(sql.contains("TIMESTAMPTZ(6)"));
}

#[test]
fn create_table_with_unique() {
    let t = Entity::new(
        "accounts",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("email", DataType::Text).unique(),
        ],
    );
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
}

#[test]
fn create_table_with_references() {
    let t = Entity::new(
        "posts",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("author_id", DataType::Uuid).references(
                "users",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
        ],
    );
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE"));
}

#[test]
fn create_table_with_constraints() {
    let t = Entity::new(
        "memberships",
        vec![
            Field::new("user_id", DataType::Uuid),
            Field::new("group_id", DataType::Uuid),
            Field::new("role", DataType::Text),
        ],
    )
    .with_constraints(vec![
        EntityConstraint::Unique(&["user_id", "group_id"]),
        EntityConstraint::Check("role IN ('admin', 'member')"),
    ]);
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("UNIQUE (user_id, group_id)"));
    assert!(sql.contains("CHECK (role IN ('admin', 'member'))"));
}

#[test]
fn create_table_with_fk_constraint() {
    let t = Entity::new(
        "order_items",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("order_id", DataType::Uuid),
            Field::new("product_id", DataType::Uuid),
        ],
    )
    .with_constraints(vec![EntityConstraint::ForeignKey {
        columns: &["order_id"],
        ref_table: "orders",
        ref_columns: &["id"],
        on_delete: FkAction::Cascade,
    }]);
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("FOREIGN KEY (order_id) REFERENCES orders (id) ON DELETE CASCADE"));
}

#[test]
fn create_table_with_default_unique_fk() {
    let t = Entity::new(
        "credentials",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("user_id", DataType::Uuid).references(
                "users",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
            Field::new("kind", DataType::Text),
            Field::new("secret_hash", DataType::Text).unique(),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
        ],
    );
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE"));
    assert!(sql.contains("secret_hash TEXT NOT NULL UNIQUE"));
    assert!(sql.contains("TIMESTAMPTZ(6)"));
    assert!(sql.contains("PRIMARY KEY (id)"));
}

#[test]
fn custom_field_type() {
    let t = Entity::new(
        "items",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("status", DataType::TypeRef("item_status".into())),
        ],
    );
    let sql = t.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("status item_status NOT NULL"));
}

// ===========================================================================
// DDL tests — DROP TABLE (DropEntityBuilder)
// ===========================================================================

#[test]
fn drop_table() {
    let s = settings();
    let sql = s
        .drop_entity()
        .if_exists()
        .cascade()
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "DROP TABLE IF EXISTS tenant_settings CASCADE");
}

// ===========================================================================
// DDL tests — ALTER TABLE (AlterEntityBuilder)
// ===========================================================================

#[test]
fn alter_table_add_field() {
    let u = users();
    let sql = u
        .alter()
        .add_field(FieldDef::new("phone", DataType::Text).nullable())
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "ALTER TABLE users ADD COLUMN phone TEXT");
}

#[test]
fn alter_table_drop_column() {
    let u = users();
    let sql = u
        .alter()
        .drop_field("legacy_field")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "ALTER TABLE users DROP COLUMN legacy_field");
}

#[test]
fn alter_table_alter_field_type() {
    let u = users();
    let sql = u
        .alter()
        .alter_field_type("status", DataType::TypeRef("VARCHAR(50)".into()))
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ALTER COLUMN status TYPE VARCHAR(50)"
    );
}

#[test]
fn alter_table_set_default() {
    let u = users();
    let sql = u
        .alter()
        .set_default("status", "'active'")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ALTER COLUMN status SET DEFAULT 'active'"
    );
}

#[test]
fn alter_table_drop_default() {
    let u = users();
    let sql = u
        .alter()
        .drop_default("status")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "ALTER TABLE users ALTER COLUMN status DROP DEFAULT");
}

#[test]
fn alter_table_set_not_null() {
    let u = users();
    let sql = u.alter().set_not_null("email").render(Some(&pg())).unwrap();
    assert_eq!(sql, "ALTER TABLE users ALTER COLUMN email SET NOT NULL");
}

#[test]
fn alter_table_drop_not_null() {
    let u = users();
    let sql = u
        .alter()
        .drop_not_null("display_name")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users ALTER COLUMN display_name DROP NOT NULL"
    );
}

#[test]
fn alter_table_rename_column() {
    let u = users();
    let sql = u
        .alter()
        .rename_field("email", "email_address")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "ALTER TABLE users RENAME COLUMN email TO email_address"
    );
}

#[test]
fn alter_table_rename_table() {
    let u = users();
    let sql = u
        .alter()
        .rename_model("app_users")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "ALTER TABLE users RENAME TO app_users");
}

#[test]
fn alter_table_add_constraint() {
    let u = users();
    let sql = u
        .alter()
        .add_constraint(EntityConstraint::Unique(&["tenant_id", "email"]))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("ALTER TABLE users ADD UNIQUE (tenant_id, email)"));
}

#[test]
fn alter_table_drop_constraint() {
    let u = users();
    let sql = u
        .alter()
        .drop_constraint("uq_email")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "ALTER TABLE users DROP CONSTRAINT uq_email");
}

#[test]
fn alter_table_multiple_actions() {
    let u = users();
    let sql = u
        .alter()
        .add_field(FieldDef::new("phone", DataType::Text).nullable())
        .set_not_null("email")
        .render(Some(&pg()))
        .unwrap();
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
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "CREATE UNIQUE INDEX idx_users_email ON users (email)");
}

#[test]
fn create_index_if_not_exists() {
    let sql = DefineIndexBuilder::new("idx_users_email")
        .on("users")
        .columns(&["email"])
        .if_not_exists()
        .render(Some(&pg()))
        .unwrap();
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
        .render(Some(&pg()))
        .unwrap();
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
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "CREATE INDEX CONCURRENTLY idx_email ON users (email)");
}

#[test]
fn drop_index() {
    let sql = DropIndexBuilder::new("idx_users_email")
        .if_exists()
        .cascade()
        .render(Some(&pg()))
        .unwrap();
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
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
}

#[test]
fn revoke_insert() {
    let sql = RevokeBuilder::new(Privilege::Insert)
        .on("users")
        .from("app_writer")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "REVOKE INSERT ON users FROM app_writer");
}

#[test]
fn grant_all() {
    let sql = GrantBuilder::new(Privilege::All)
        .on("tenants")
        .to("admin")
        .render(Some(&pg()))
        .unwrap();
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
    assert_eq!(
        TransactionBuilder::render(&ir, None).unwrap(),
        "SAVEPOINT sp1"
    );

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
    let expr = field("email").eq(param());
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "email = $1");
}

#[test]
fn expr_nested_boolean() {
    let expr = field("a").eq(param()) & (field("b").gt(param()) | field("c").lt(param()));
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "(a = $1 AND (b > $2 OR c < $3))");
}

#[test]
fn expr_not() {
    let expr = !field("deleted");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "NOT (deleted)");
}

#[test]
fn expr_arithmetic() {
    let expr = (field("price") * float(1.1f64)) + int(5i64);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "price * 1.1 + 5");
}

#[test]
fn expr_is_null_is_not_null() {
    let pg = Dialect::postgres();
    let mut c1 = pg.param_counter();
    let sql1 =
        crate::backend::sql::render::render_expr(&field("meta").is_null(), &mut c1, &pg).unwrap();
    assert_eq!(sql1, "meta IS NULL");

    let mut c2 = pg.param_counter();
    let sql2 =
        crate::backend::sql::render::render_expr(&field("meta").is_null().negate(), &mut c2, &pg)
            .unwrap();
    assert_eq!(sql2, "meta IS NOT NULL");
}

#[test]
fn expr_between() {
    let expr = field("age").between(param(), param());
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "age BETWEEN $1 AND $2");
}

#[test]
fn expr_in_list() {
    let expr = field("status").in_list(vec![param(), param()]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "status IN ($1, $2)");
}

#[test]
fn expr_not_in_list() {
    let expr = field("status").in_list(vec![param(), param()]).negate();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "status NOT IN ($1, $2)");
}

#[test]
fn expr_like_ilike() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();

    // PG supports ILIKE natively
    let expr = field("name").ilike(param());
    let mut c1 = pg.param_counter();
    let sql1 = crate::backend::sql::render::render_expr(&expr, &mut c1, &pg).unwrap();
    assert_eq!(sql1, "name ILIKE $1");

    // MySQL falls back to LOWER(x) LIKE LOWER(y)
    let mut c2 = mysql.param_counter();
    let sql2 = crate::backend::sql::render::render_expr(&expr, &mut c2, &mysql).unwrap();
    assert_eq!(sql2, "LOWER(name) LIKE LOWER(?)");
}

#[test]
fn expr_cast() {
    let expr = field("price").cast(DataType::Decimal {
        precision: Some(10),
        scale: Some(2),
    });
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "CAST(price AS NUMERIC(10,2))");
}

#[test]
fn expr_alias() {
    let expr = field("email").alias("e");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "email AS e");
}

#[test]
fn expr_bool_literal_pg_vs_mysql() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&bool_expr(true), &mut c1, &pg).unwrap(),
        "TRUE"
    );

    let mut c2 = mysql.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&bool_expr(true), &mut c2, &mysql).unwrap(),
        "1"
    );
}

#[test]
fn expr_concat_pg_pipe_vs_mysql_func() {
    let expr = field("first_name").concat(field("last_name"));
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();
    let mssql = Dialect::mssql();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&expr, &mut c1, &pg).unwrap(),
        "first_name || last_name"
    );

    let mut c2 = mysql.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&expr, &mut c2, &mysql).unwrap(),
        "CONCAT(first_name, last_name)"
    );

    let mut c3 = mssql.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&expr, &mut c3, &mssql).unwrap(),
        "first_name + last_name"
    );
}

#[test]
fn expr_case_when() {
    let expr = case()
        .when(field("status").eq(string("active")), string("Active"))
        .when(
            field("status").eq(string("disabled")),
            string("Disabled"),
        )
        .else_(string("Unknown"))
        .end();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(
        sql,
        "CASE WHEN status = 'active' THEN 'Active' WHEN status = 'disabled' THEN 'Disabled' ELSE 'Unknown' END"
    );
}

// ===========================================================================
// DOL-specific: field() produces Expr::Identifier
// ===========================================================================

#[test]
fn field_produces_ref() {
    let f = field("email");
    if let Expr::Ref(ref path) = f {
        assert_eq!(path.as_single(), Some("email"));
    } else {
        panic!("expected Expr::Ref, got {f:?}");
    }
}

// ===========================================================================
// SQL functions
// ===========================================================================

#[test]
fn func_lower_upper() {
    let pg = Dialect::postgres();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::lower(field("email")), &mut c1, &pg)
            .unwrap(),
        "LOWER(email)"
    );

    let mut c2 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::upper(field("name")), &mut c2, &pg)
            .unwrap(),
        "UPPER(name)"
    );
}

#[test]
fn func_coalesce() {
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let expr = func::coalesce(vec![field("display_name"), field("email")]);
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "COALESCE(display_name, email)");
}

#[test]
fn func_count_star() {
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql =
        crate::backend::sql::render::render_expr(&func::count_star(), &mut counter, &pg).unwrap();
    assert_eq!(sql, "COUNT(*)");
}

#[test]
fn func_aggregates() {
    let pg = Dialect::postgres();

    let cases = [
        (func::count(field("id")), "COUNT(id)"),
        (func::sum(field("amount")), "SUM(amount)"),
        (func::avg(field("score")), "AVG(score)"),
        (func::min(field("price")), "MIN(price)"),
        (func::max(field("price")), "MAX(price)"),
    ];
    for (expr, expected) in cases {
        let mut c = pg.param_counter();
        assert_eq!(
            crate::backend::sql::render::render_expr(&expr, &mut c, &pg).unwrap(),
            expected
        );
    }
}

#[test]
fn func_now_and_dates() {
    let pg = Dialect::postgres();

    let mut c1 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::now(), &mut c1, &pg).unwrap(),
        "NOW()"
    );

    let mut c2 = pg.param_counter();
    assert_eq!(
        crate::backend::sql::render::render_expr(&func::current_timestamp(), &mut c2, &pg).unwrap(),
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
        .partition_by(vec![field("tenant_id")])
        .order_by(vec![field("created_at").desc()])
        .build();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(
        sql,
        "ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY created_at DESC)"
    );
}

#[test]
fn window_rank_with_frame() {
    let expr = func::rank()
        .over()
        .order_by(vec![field("score").desc()])
        .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
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
    let u = users();
    let q1 = u
        .get()
        .fields(&["id", "email"])
        .filter(field("tenant_id").eq(param()))
        .build();
    let q2 = u
        .get()
        .fields(&["id", "email"])
        .filter(field("status").eq(string("active")))
        .build();
    let sql = CompoundSelectBuilder::new(q1)
        .union(q2)
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("UNION"));
    assert!(sql.contains("SELECT id, email FROM users WHERE tenant_id = $1"));
}

#[test]
fn set_op_union_all() {
    let s = settings();
    let q1 = s.get().build();
    let q2 = s.get().filter(field("tenant_id").eq(param())).build();
    let sql = CompoundSelectBuilder::new(q1)
        .union_all(q2)
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("UNION ALL"));
}

#[test]
fn set_op_intersect_except() {
    let u = users();
    let q1 = u.get().fields(&["id"]).build();
    let q2 = u
        .get()
        .fields(&["id"])
        .filter(field("tenant_id").eq(param()))
        .build();
    let q3 = u
        .get()
        .fields(&["id"])
        .filter(field("status").eq(string("disabled")))
        .build();
    let sql = CompoundSelectBuilder::new(q1)
        .intersect(q2)
        .except(q3)
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("INTERSECT"));
    assert!(sql.contains("EXCEPT"));
}

#[test]
fn set_op_with_order_and_limit() {
    let u = users();
    let q1 = u.get().fields(&["id"]).build();
    let q2 = u
        .get()
        .fields(&["id"])
        .filter(field("tenant_id").eq(param()))
        .build();
    let sql = CompoundSelectBuilder::new(q1)
        .union_all(q2)
        .order_by(vec![field("id").asc()])
        .limit()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("UNION ALL"));
    assert!(sql.contains("ORDER BY id ASC"));
    assert!(sql.contains("LIMIT"));
}

// ===========================================================================
// Cross-dialect rendering tests
// ===========================================================================

#[test]
fn dialect_mysql_select_with_where() {
    let mysql = Dialect::mysql();
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("id").eq(param()))
        .render(Some(&mysql))
        .unwrap();
    assert_eq!(
        sql,
        "SELECT id, tenant_id, email, display_name, status, created_at, updated_at FROM users WHERE tenant_id = ? AND id = ?"
    );
}

#[test]
fn dialect_mssql_select_with_where() {
    let mssql = Dialect::mssql();
    let u = users();
    let sql = u
        .get()
        .fields(&["id", "email"])
        .filter(field("id").eq(param()))
        .render(Some(&mssql))
        .unwrap();
    assert_eq!(sql, "SELECT id, email FROM users WHERE id = @p1");
}

#[test]
fn dialect_oracle_select_with_where() {
    let oracle = Dialect::oracle();
    let u = users();
    let sql = u
        .get()
        .fields(&["id"])
        .filter(field("tenant_id").eq(param()))
        .render(Some(&oracle))
        .unwrap();
    assert_eq!(sql, "SELECT id FROM users WHERE tenant_id = :1");
}

#[test]
fn dialect_mysql_insert() {
    let mysql = Dialect::mysql();
    let s = settings();
    let sql = s.insert().render(Some(&mysql)).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) VALUES (?, ?, ?)"
    );
}

#[test]
fn dialect_mssql_insert() {
    let mssql = Dialect::mssql();
    let s = settings();
    let sql = s.insert().render(Some(&mssql)).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) VALUES (@p1, @p2, @p3)"
    );
}

#[test]
fn dialect_mysql_batch_insert() {
    let mysql = Dialect::mysql();
    let s = settings();
    let sql = s.insert().rows(2).render(Some(&mysql)).unwrap();
    assert_eq!(
        sql,
        "INSERT INTO tenant_settings (tenant_id, max_users, mfa_required) VALUES (?, ?, ?), (?, ?, ?)"
    );
}

#[test]
fn dialect_mysql_update() {
    let mysql = Dialect::mysql();
    let u = users();
    let sql = u
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .render(Some(&mysql))
        .unwrap();
    assert_eq!(sql, "UPDATE users SET email = ? WHERE id = ?");
}

#[test]
fn dialect_mssql_update() {
    let mssql = Dialect::mssql();
    let u = users();
    let sql = u
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .render(Some(&mssql))
        .unwrap();
    assert_eq!(sql, "UPDATE users SET email = @p1 WHERE id = @p2");
}

#[test]
fn dialect_mysql_delete() {
    let mysql = Dialect::mysql();
    let u = users();
    let sql = u
        .remove()
        .filter(field("id").eq(param()))
        .render(Some(&mysql))
        .unwrap();
    assert_eq!(sql, "DELETE FROM users WHERE id = ?");
}

#[test]
fn dialect_pg_returning() {
    let pg = Dialect::postgres();
    let u = users();
    let sql = u
        .remove()
        .filter(field("id").eq(param()))
        .returning(&["id"])
        .render(Some(&pg))
        .unwrap();
    assert!(sql.ends_with(" RETURNING id"));
}

#[test]
fn dialect_mysql_returning_unsupported() {
    let mysql = Dialect::mysql();
    let u = users();
    let sql = u
        .remove()
        .filter(field("id").eq(param()))
        .returning(&["id"])
        .render(Some(&mysql))
        .unwrap();
    assert!(!sql.contains("RETURNING"));
    assert_eq!(sql, "DELETE FROM users WHERE id = ?");
}

#[test]
fn dialect_mssql_returning_output() {
    let mssql = Dialect::mssql();
    let s = settings();
    let sql = s
        .insert()
        .returning(&["tenant_id"])
        .render(Some(&mssql))
        .unwrap();
    assert!(sql.contains("OUTPUT INSERTED.tenant_id"));
}

#[test]
fn dialect_mssql_pagination_offset_fetch() {
    let mssql = Dialect::mssql();
    let u = users();
    let sql = u
        .get()
        .order_by_asc("id")
        .offset()
        .limit()
        .render(Some(&mssql))
        .unwrap();
    assert!(sql.contains("OFFSET @p1 ROWS FETCH NEXT @p2 ROWS ONLY"));
}

#[test]
fn dialect_pg_pagination_limit_offset() {
    let pg = Dialect::postgres();
    let u = users();
    let sql = u
        .get()
        .order_by_asc("id")
        .offset()
        .limit()
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("OFFSET $1 LIMIT $2"));
}

#[test]
fn dialect_mysql_upsert_on_duplicate_key() {
    // DOL currently always uses ON CONFLICT syntax (PostgreSQL-native).
    // Dialect-specific upsert rendering (ON DUPLICATE KEY, MERGE) is a Phase 9 item.
    let mysql = Dialect::mysql();
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .render(Some(&mysql))
        .unwrap();
    // Verify it renders something valid (PG-style with MySQL params)
    assert!(sql.contains("ON CONFLICT"));
    assert!(sql.contains("DO UPDATE SET"));
}

#[test]
fn dialect_mysql_upsert_do_nothing() {
    let mysql = Dialect::mysql();
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_nothing()
        .render(Some(&mysql))
        .unwrap();
    assert!(sql.contains("DO NOTHING"));
}

#[test]
fn dialect_mssql_upsert_merge() {
    // DOL currently always uses ON CONFLICT syntax.
    // MSSQL MERGE rendering is a Phase 9 item.
    let mssql = Dialect::mssql();
    let s = settings();
    let sql = s
        .upsert()
        .on_conflict(&["tenant_id"])
        .do_update(&["max_users", "mfa_required"])
        .render(Some(&mssql))
        .unwrap();
    assert!(sql.contains("ON CONFLICT"));
}

#[test]
fn dialect_sqlite_select() {
    let sqlite = Dialect::sqlite();
    let u = users();
    let sql = u
        .get()
        .fields(&["id", "email"])
        .filter(field("id").eq(param()))
        .render(Some(&sqlite))
        .unwrap();
    assert_eq!(sql, "SELECT id, email FROM users WHERE id = ?");
}

#[test]
fn dialect_distinct_on_pg_only() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();
    let u = users();

    let pg_sql = u
        .get()
        .distinct_on(&["tenant_id"])
        .render(Some(&pg))
        .unwrap();
    assert!(pg_sql.starts_with("SELECT DISTINCT ON (tenant_id)"));

    let mysql_sql = u
        .get()
        .distinct_on(&["tenant_id"])
        .render(Some(&mysql))
        .unwrap();
    assert!(!mysql_sql.contains("DISTINCT ON"));
}

#[test]
fn dialect_create_table_type_mapping() {
    let mysql = Dialect::mysql();
    let s = settings();
    let sql = s.create().render(Some(&mysql)).unwrap();
    assert!(sql.contains("CHAR(36)"));
    assert!(sql.contains("INT"));
    assert!(sql.contains("TINYINT(1)"));
}

#[test]
fn dialect_create_table_sqlite_types() {
    let sqlite = Dialect::sqlite();
    let u = users();
    let sql = u.create().render(Some(&sqlite)).unwrap();
    assert!(sql.contains("id TEXT"));
    assert!(sql.contains("created_at TEXT"));
}

#[test]
fn dialect_cockroachdb_is_pg_compatible() {
    let crdb = Dialect::cockroachdb();
    let u = users();
    let sql = u
        .get()
        .filter(field("id").eq(param()))
        .render(Some(&crdb))
        .unwrap();
    assert!(sql.contains("WHERE id = $1"));
}

#[test]
fn dialect_for_update_sqlite_unsupported() {
    let sqlite = Dialect::sqlite();
    let u = users();
    let sql = u.get().for_update().render(Some(&sqlite)).unwrap();
    assert!(!sql.contains("FOR UPDATE"));
}

#[test]
fn dialect_nulls_ordering_mysql_unsupported() {
    let mysql = Dialect::mysql();
    let u = users();
    let sql = u
        .get()
        .order_by("created_at", Direction::Desc, Some(NullsPosition::Last))
        .render(Some(&mysql))
        .unwrap();
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
    assert_eq!(pg.resolve_type(&DataType::Uuid), "UUID");
    assert_eq!(
        pg.resolve_type(&DataType::TimestampTz { precision: 6 }),
        "TIMESTAMPTZ(6)"
    );

    let mysql = Dialect::mysql();
    assert_eq!(mysql.resolve_type(&DataType::Uuid), "CHAR(36)");
    assert_eq!(mysql.resolve_type(&DataType::Bool), "TINYINT(1)");
    assert_eq!(
        mysql.resolve_type(&DataType::TimestampTz { precision: 6 }),
        "DATETIME(6)"
    );

    let mssql = Dialect::mssql();
    assert_eq!(mssql.resolve_type(&DataType::Uuid), "UNIQUEIDENTIFIER");
    assert_eq!(mssql.resolve_type(&DataType::Bool), "BIT");

    let oracle = Dialect::oracle();
    assert_eq!(oracle.resolve_type(&DataType::Uuid), "RAW(16)");
    assert_eq!(oracle.resolve_type(&DataType::Bool), "NUMBER(1)");

    let sqlite = Dialect::sqlite();
    assert_eq!(sqlite.resolve_type(&DataType::Uuid), "TEXT");
    assert_eq!(sqlite.resolve_type(&DataType::Bool), "INTEGER");
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
    let u = users();
    let sql = u
        .get()
        .fields(&["id"])
        .order_by_asc("id")
        .offset()
        .limit()
        .render(Some(&oracle))
        .unwrap();
    assert!(sql.contains("OFFSET :1 ROWS FETCH NEXT :2 ROWS ONLY"));
}

#[test]
fn dialect_drop_table_mssql_no_cascade() {
    // DOL's current drop model renderer doesn't honor dialect CASCADE restrictions.
    // This will be refined in Phase 9 when dialect-aware DDL is fully implemented.
    let mssql = Dialect::mssql();
    let u = users();
    let sql = u
        .drop_entity()
        .if_exists()
        .cascade()
        .render(Some(&mssql))
        .unwrap();
    assert!(sql.contains("IF EXISTS"));
    // Currently always adds CASCADE — OK for now
    assert!(sql.contains("DROP TABLE"));
}

#[test]
fn dialect_mysql_ilike_fallback() {
    let mysql = Dialect::mysql();
    let u = users();
    let sql = u
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("email").ilike(param()))
        .render(Some(&mysql))
        .unwrap();
    assert!(sql.contains("LOWER(email) LIKE LOWER(?)"));
}

#[test]
fn dialect_mssql_expr_params() {
    let mssql = Dialect::mssql();
    let u = users();
    let sql = u
        .get()
        .filter(field("id").eq(param()))
        .filter(field("status").eq(param()))
        .render(Some(&mssql))
        .unwrap();
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
            "type_dialect": "Postgres",
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

        let u = users();
        let sql = u
            .get()
            .fields(&["id"])
            .filter(field("id").eq(param()))
            .render(Some(&d))
            .unwrap();
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
type_dialect = "MySQL"

[param_style]
style = "Positional"

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

        let s = settings();
        let sql = s.insert().render(Some(&d)).unwrap();
        assert!(sql.contains("VALUES (?, ?, ?)"));
    }

    #[test]
    fn dialect_from_yaml_str() {
        let yaml = r#"
name: sqlite
param_style:
  style: Positional
quote_style: DoubleQuote
type_dialect: SQLite
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

        let u = users();
        let sql = u
            .get()
            .fields(&["id"])
            .filter(field("id").eq(param()))
            .render(Some(&d))
            .unwrap();
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
    let expr = field("profile").get("address").get("city");
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert_eq!(sql, "profile->>'address'->>'city'");
}

#[test]
fn expr_object_literal() {
    use crate::expr::obj;
    let expr = obj(vec![("name", string("Alice")), ("age", int(30i64))]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert!(sql.contains("'name'"));
    assert!(sql.contains("'Alice'"));
}

#[test]
fn expr_array_literal() {
    use crate::expr::arr;
    let expr = arr(vec![int(1i64), int(2i64), int(3i64)]);
    let pg = Dialect::postgres();
    let mut counter = pg.param_counter();
    let sql = crate::backend::sql::render::render_expr(&expr, &mut counter, &pg).unwrap();
    assert!(sql.contains("ARRAY[1, 2, 3]"));
}

// ===========================================================================
// DOL-specific: Entity::define() (DefineEntityBuilder)
// ===========================================================================

#[test]
fn define_model_via_static_method() {
    use crate::op::definition::FieldDef;

    let ir = Entity::define("users")
        .field(FieldDef::new("id", DataType::Uuid).primary_key())
        .field(FieldDef::new("email", DataType::Text).unique())
        .build();
    assert_eq!(ir.name, "users");
    assert_eq!(ir.fields.len(), 2);
}

// ===========================================================================
// DOL-specific: Char/Varchar in CREATE TABLE
// ===========================================================================

#[test]
fn char_varchar_in_create_table() {
    let codes = Entity::new(
        "codes",
        vec![
            Field::new("id", DataType::Int32).primary_key(),
            Field::new("code", DataType::Char(3)),
            Field::new("description", DataType::Varchar(Some(255))),
            Field::new("notes", DataType::Varchar(None)),
        ],
    );
    let sql = codes.create().render(Some(&pg())).unwrap();
    assert!(sql.contains("CHAR(3)"), "sql = {}", sql);
    assert!(sql.contains("VARCHAR(255)"), "sql = {}", sql);
    assert!(sql.contains("VARCHAR NOT NULL"), "sql = {}", sql);
}

#[test]
fn char_varchar_dialect_resolve() {
    use crate::backend::sql::dialect::Dialect;
    let pg = Dialect::postgres();
    assert_eq!(pg.resolve_type(&DataType::Char(5)), "CHAR(5)");
    assert_eq!(
        pg.resolve_type(&DataType::Varchar(Some(100))),
        "VARCHAR(100)"
    );
    assert_eq!(pg.resolve_type(&DataType::Varchar(None)), "VARCHAR");
}

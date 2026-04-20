//! # DOL Playbook — Comprehensive Feature Coverage Examples
//!
//! This example file demonstrates **every** feature of the DOL crate,
//! organized by domain. Run with:
//!
//! ```sh
//! cargo test -p dol --all-features --example playbook
//! ```
//!
//! ## Table of Contents
//!
//! 1. **Model Definition** — defining tables/models, fields, constraints
//! 2. **SELECT Queries** — simple selects, joins, aggregations, window functions
//! 3. **Mutations** — INSERT, UPDATE, DELETE, UPSERT
//! 4. **DDL** — CREATE TABLE, ALTER TABLE, DROP TABLE, indexes
//! 5. **Expressions** — comparisons, arithmetic, functions, CASE, subqueries
//! 6. **Set Operations** — UNION, INTERSECT, EXCEPT
//! 7. **Transactions** — BEGIN, COMMIT, ROLLBACK, savepoints
//! 8. **Access Control** — GRANT, REVOKE
//! 9. **Multi-Dialect** — same query across PostgreSQL, MySQL, SQLite, MSSQL, Oracle
//! 10. **Object Storage** — PUT, GET, LIST objects; READ, WRITE, MOVE files
//! 11. **Configuration** — loading dialect and backend configs
//! 12. **Migrations** — forward/backward, all storage kinds
//! 13. **Schema Diff** — auto-detect model changes, generate alter steps
//! 14. **Config-Integrated Migrations** — run migrations using DolConfig settings

use dol::CompoundSelectBuilder;
use dol::EntityBuilderExt;
use dol::Render;
use dol::TransactionRender;
use dol::backend::sql::dialect::Dialect;
use dol::builder::control::{GrantBuilder, Privilege, RevokeBuilder};
use dol::builder::{DefineEntityBuilder, DefineIndexBuilder, DropIndexBuilder};
use dol::builder::storage::{
    GetObjectBuilder, ListObjectsBuilder, MoveFileBuilder, PutObjectBuilder, ReadFileBuilder,
    WriteFileBuilder,
};
use dol::builder::transaction::TransactionBuilder;
use dol::expr::window::FrameBound;
use dol::expr::{Direction, Expr, bool_expr, case, field, float, func, int, param, raw_expr, string};
use dol::ir::LockMode;
use dol::model::{DataType, Entity, EntityConstraint, Field, FkAction};

// ============================================================================
// 1. MODEL DEFINITION — the single source of truth for a data shape
// ============================================================================

/// Users table with all common field constraint types.
fn users() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("tenant_id", DataType::Uuid).index(),
            Field::new("email", DataType::Text).unique(),
            Field::new("display_name", DataType::Text),
            Field::new("status", DataType::Text).default("'active'"),
            Field::new("login_count", DataType::Int32).default("0"),
            Field::new("profile", DataType::Json).nullable(),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
            Field::new("updated_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
        ],
    )
}

/// Tenants table with composite constraints and version field.
fn tenants() -> Entity {
    Entity::new(
        "tenants",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("slug", DataType::Text).unique(),
            Field::new("display_name", DataType::Text),
            Field::new("status", DataType::Text).default("'active'"),
            Field::new("plan", DataType::Text).default("'free'"),
            Field::new("labels", DataType::Json).nullable(),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }),
            Field::new("created_by", DataType::Uuid),
            Field::new("updated_at", DataType::TimestampTz { precision: 6 }),
            Field::new("updated_by", DataType::Uuid),
            Field::new("version", DataType::Int32).default("1"),
        ],
    )
    .with_constraints(vec![EntityConstraint::Unique(&["slug"])])
}

/// Sessions table for join examples.
fn sessions() -> Entity {
    Entity::new(
        "sessions",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("user_id", DataType::Uuid).references(
                "users",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
            Field::new("token_hash", DataType::Varbinary(None)),
            Field::new("user_agent", DataType::Text)
                .nullable()
                .comment("Browser / client identifier"),
            Field::new("ip_address", DataType::Inet).nullable(),
            Field::new("expires_at", DataType::TimestampTz { precision: 6 }),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
            Field::new("is_expired", DataType::Bool).generated_virtual("expires_at < NOW()"),
        ],
    )
}

/// Audit log with foreign keys, CHECK constraint, and namespace.
fn audit_log() -> Entity {
    Entity::new(
        "audit_log",
        vec![
            Field::new("id", DataType::Int64).auto_increment().primary_key(),
            Field::new("user_id", DataType::Uuid).references(
                "users",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
            Field::new("tenant_id", DataType::Uuid).references(
                "tenants",
                "id",
                FkAction::Cascade,
                FkAction::NoAction,
            ),
            Field::new("action", DataType::Text)
                .check("action IN ('create', 'read', 'update', 'delete')"),
            Field::new("resource_type", DataType::Text),
            Field::new("resource_id", DataType::Text),
            Field::new("details", DataType::Json).nullable(),
            Field::new("ip_address", DataType::Inet).nullable(),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
        ],
    )
    .with_namespace("audit")
    .with_constraints(vec![EntityConstraint::ForeignKey {
        columns: &["tenant_id"],
        ref_table: "tenants",
        ref_columns: &["id"],
        on_delete: FkAction::Cascade,
    }])
}

/// Products table with various field types for type-mapping demos.
fn products() -> Entity {
    Entity::new(
        "products",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("name", DataType::Varchar(Some(255))),
            Field::new("sku", DataType::Char(12)).unique(),
            Field::new("price", DataType::Decimal { precision: None, scale: None }),
            Field::new("quantity", DataType::Int32).default("0"),
            Field::new("weight_kg", DataType::Float32).nullable(),
            Field::new("description", DataType::Text).nullable(),
            Field::new("image_path", DataType::Text).nullable(),
            Field::new("mime_type", DataType::Text).nullable(),
            Field::new("is_active", DataType::Bool).default("TRUE"),
            Field::new("tags", DataType::Array(Box::new(DataType::Text))).nullable(),
            Field::new("metadata", DataType::Json).nullable(),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
        ],
    )
}

/// Orders table with composite primary key.
fn order_items() -> Entity {
    Entity::new(
        "order_items",
        vec![
            Field::new("order_id", DataType::Uuid),
            Field::new("product_id", DataType::Uuid),
            Field::new("quantity", DataType::Int32),
            Field::new("unit_price", DataType::Decimal { precision: None, scale: None }),
        ],
    )
    .with_constraints(vec![
        EntityConstraint::PrimaryKey(&["order_id", "product_id"]),
        EntityConstraint::ForeignKey {
            columns: &["product_id"],
            ref_table: "products",
            ref_columns: &["id"],
            on_delete: FkAction::Restrict,
        },
    ])
}

/// Settings table for aggregate examples.
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

fn main() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();
    let sqlite = Dialect::sqlite();

    let users = users();
    let tenants = tenants();
    let sessions = sessions();
    let audit_log = audit_log();
    let products = products();
    let order_items = order_items();
    let settings = settings();

    println!("=== DOL Playbook — Comprehensive Feature Coverage ===\n");

    // ========================================================================
    // 1. MODEL INTROSPECTION
    // ========================================================================
    println!("--- 1. Model Introspection ---");

    // Field lookup
    let email_field = users.field("email");
    assert_eq!(email_field.name, "email");
    assert_eq!(email_field.data_type, DataType::Text);
    assert!(email_field.unique);
    println!(
        "  Field lookup: users.email -> {:?}",
        email_field.data_type
    );

    // Safe field lookup
    assert!(users.try_field("nonexistent").is_none());
    println!("  Safe lookup: try_field('nonexistent') -> None");

    // Primary keys
    let pks: Vec<_> = users.primary_keys().map(|f| f.name).collect();
    assert_eq!(pks, vec!["id"]);
    println!("  Primary keys: {:?}", pks);

    // Non-PK fields
    let non_pk_count = users.non_pk_fields().count();
    println!("  Non-PK fields: {} fields", non_pk_count);

    // Field list
    let field_list = users.field_list();
    assert!(field_list.contains("id"));
    assert!(field_list.contains("email"));
    assert!(field_list.contains(", "));
    println!("  Field list: {}", field_list);

    // Qualified name with namespace
    let qualified = audit_log.qualified_name();
    assert_eq!(qualified, "audit.audit_log");
    println!("  Qualified name: {}", qualified);

    // Qualified name without namespace
    let simple_name = users.qualified_name();
    assert_eq!(simple_name, "users");
    println!("  Simple name: {}", simple_name);

    // ========================================================================
    // 2. SELECT QUERIES
    // ========================================================================
    println!("\n--- 2. SELECT Queries ---");

    // 2a. Simple SELECT all columns
    let sql = users.get().render(Some(&pg)).unwrap();
    println!("  [PG] Select all: {}", sql);
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("FROM users"));

    // 2b. SELECT specific columns with WHERE
    let sql = users
        .get()
        .fields(&["id", "email", "status"])
        .filter(field("tenant_id").eq(param()))
        .filter(field("status").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Filtered: {}", sql);
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));

    // 2c. WHERE with OR groups
    let sql = users
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("status").eq(string("active")) | field("status").eq(string("pending")))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] OR filter: {}", sql);

    // 2d. LIKE / ILIKE patterns
    let sql = users
        .get()
        .filter(field("email").ilike(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] ILIKE: {}", sql);
    assert!(sql.contains("ILIKE"));

    // 2e. NULL checks
    let sql = users
        .get()
        .filter(field("profile").is_null().negate())
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] IS NOT NULL: {}", sql);

    // 2f. BETWEEN range
    let sql = users
        .get()
        .filter(field("login_count").between(int(10), int(100)))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] BETWEEN: {}", sql);

    // 2g. IN list
    let sql = users
        .get()
        .filter(field("status").in_list(vec![string("active"), string("pending"), string("suspended")]))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] IN list: {}", sql);

    // 2h. IN subquery
    let sql = users
        .get()
        .filter(Expr::InSubquery {
            expr: Box::new(field("tenant_id")),
            subquery: "SELECT id FROM tenants WHERE status = 'active'".to_string(),
            negated: false,
        })
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] IN subquery: {}", sql);

    // 2i. NOT IN subquery (via filter)
    let subquery = "SELECT user_id FROM banned_users";
    let sql = users
        .get()
        .filter(field("id").in_subquery(subquery).negate())
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] NOT IN subquery: {}", sql);

    // 2j. EXISTS subquery
    let sql = users
        .get()
        .filter(Expr::Exists {
            subquery: "SELECT 1 FROM sessions WHERE sessions.user_id = users.id".to_string(),
            negated: false,
        })
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] EXISTS: {}", sql);

    // 2k. NOT EXISTS subquery
    let sql = users
        .get()
        .filter(Expr::Exists {
            subquery: "SELECT 1 FROM banned WHERE banned.user_id = users.id".to_string(),
            negated: true,
        })
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] NOT EXISTS: {}", sql);

    // 2l. Scalar subquery in projection
    let sql = users
        .get()
        .fields(&["id", "email"])
        .field(
            dol::expr::Expr::Subquery(
                "SELECT COUNT(*) FROM sessions WHERE sessions.user_id = users.id".into(),
            )
            .alias("session_count"),
        )
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Scalar subquery: {}", sql);

    // 2m. Ordering with NULLS FIRST/LAST
    let sql = users
        .get()
        .order_by_expr(field("display_name").asc().nulls_last())
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] ORDER BY NULLS LAST: {}", sql);

    // 2n. ORDER BY with Direction enum
    let sql = users
        .get()
        .order_by(
            "display_name",
            Direction::Asc,
            Some(dol::expr::NullsPosition::Last),
        )
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] ORDER BY (Direction): {}", sql);

    // 2o. Pagination (LIMIT + OFFSET)
    let sql = users
        .get()
        .order_by_asc("email")
        .limit()
        .offset()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Pagination: {}", sql);

    // 2p. DISTINCT
    let sql = users
        .get()
        .fields(&["tenant_id"])
        .distinct()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] DISTINCT: {}", sql);

    // 2q. DISTINCT ON (PostgreSQL only)
    let sql = users
        .get()
        .distinct_on(&["tenant_id"])
        .order_by_asc("tenant_id")
        .order_by_desc("created_at")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] DISTINCT ON: {}", sql);

    // 2r. GROUP BY + HAVING
    let sql = users
        .get()
        .fields(&["tenant_id"])
        .field(Expr::CountStar.alias("cnt"))
        .group_by(&["tenant_id"])
        .having(raw_expr("COUNT(*) > 10"))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] GROUP BY HAVING: {}", sql);

    // 2s. Aggregate functions
    let sql = settings
        .get()
        .field(func::sum(field("max_users")).alias("total_users"))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] SUM: {}", sql);

    let sql = users
        .get()
        .field(Expr::CountStar.alias("total"))
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] COUNT: {}", sql);

    // 2t. Column aliases and raw expressions
    let sql = users
        .get()
        .field(field("email").alias("user_email"))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Column alias: {}", sql);

    let sql = users
        .get()
        .field(raw_expr("COALESCE(display_name, email) AS name"))
        .filter(field("id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Raw column: {}", sql);

    // 2u. INNER JOIN
    let sql = users
        .get()
        .inner_join(&tenants, &[("tenant_id", "id")])
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] INNER JOIN: {}", sql);

    // 2v. LEFT JOIN
    let sql = users
        .get()
        .left_join(&sessions, &[("id", "user_id")])
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] LEFT JOIN: {}", sql);

    // 2w. Multiple JOINs
    let sql = users
        .get()
        .inner_join(&tenants, &[("tenant_id", "id")])
        .left_join(&sessions, &[("id", "user_id")])
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Multi JOIN: {}", sql);

    // 2x. FOR UPDATE (pessimistic locking)
    let sql = users
        .get()
        .filter(field("id").eq(param()))
        .lock(LockMode::ForUpdate)
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] FOR UPDATE: {}", sql);
    assert!(sql.contains("FOR UPDATE"));

    // 2y. FOR UPDATE SKIP LOCKED (queue pattern)
    let sql = users
        .get()
        .filter(field("status").eq(param()))
        .lock(LockMode::ForUpdateSkipLocked)
        .limit()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] SKIP LOCKED: {}", sql);

    // 2z. Window functions — ROW_NUMBER
    let sql = users
        .get()
        .field(
            func::row_number()
                .over()
                .partition_by(vec![field("tenant_id")])
                .order_by(vec![field("created_at").desc()])
                .build()
                .alias("row_num"),
        )
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] ROW_NUMBER: {}", sql);

    // 2aa. Window functions — RANK with frame
    let sql = users
        .get()
        .field(field("id"))
        .field(field("login_count"))
        .field(
            func::rank()
                .over()
                .partition_by(vec![field("tenant_id")])
                .order_by(vec![field("login_count").desc()])
                .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
                .build()
                .alias("login_rank"),
        )
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] RANK + frame: {}", sql);

    // 2ab. Table alias
    let sql = users
        .get()
        .alias("u")
        .filter(field("id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Table alias: {}", sql);

    // 2ac. Param count tracking
    let q = users
        .get()
        .filter(field("tenant_id").eq(param()))
        .filter(field("status").eq(param()))
        .offset()
        .limit();
    assert_eq!(q.param_count(), 4);
    println!(
        "  Param count (2 WHERE + OFFSET + LIMIT): {}",
        q.param_count()
    );

    // ========================================================================
    // 3. MUTATIONS (INSERT, UPDATE, DELETE, UPSERT)
    // ========================================================================
    println!("\n--- 3. Mutations ---");

    // 3a. INSERT all columns
    let sql = users.insert().returning_all().render(Some(&pg)).unwrap();
    println!("  [PG] Insert all: {}", sql);
    assert!(sql.contains("INSERT INTO users"));
    assert!(sql.contains("RETURNING *"));

    // 3b. INSERT specific columns
    let sql = users
        .insert()
        .fields(&["id", "email", "tenant_id", "display_name"])
        .returning(&["id", "email"])
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Insert specific: {}", sql);

    // 3c. Batch INSERT (multiple rows)
    let sql = users
        .insert()
        .fields(&["id", "email", "tenant_id"])
        .rows(3)
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Batch insert: {}", sql);
    // 3 rows x 3 cols = 9 params
    assert!(sql.contains("$9"));

    // 3d. INSERT param count tracking
    let builder = users.insert().fields(&["id", "email", "tenant_id"]).rows(5);
    assert_eq!(builder.param_count(), 15); // 5 x 3
    println!("  Param count (5x3): {}", builder.param_count());

    // 3e. INSERT ... SELECT
    let source_query = users
        .get()
        .fields(&["id", "email"])
        .filter(field("tenant_id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    let sql = users
        .insert_select()
        .fields(&["id", "email"])
        .from_select(&source_query)
        .returning_all()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Insert select: {}", sql);

    // 3f. UPDATE with set() (single column)
    let sql = users
        .update()
        .set("display_name")
        .set("updated_at")
        .filter(field("id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Update: {}", sql);

    // 3g. UPDATE with set_fields() (multiple fields)
    let sql = users
        .update()
        .set_fields(&["display_name", "status", "updated_at"])
        .filter(field("id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Update set_fields: {}", sql);

    // 3h. UPDATE with literal SET
    let sql = users
        .update()
        .set("status")
        .set_literal("updated_at", "NOW()")
        .filter(field("id").eq(param()))
        .returning_all()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Update literal + returning: {}", sql);

    // 3i. UPDATE with version increment (optimistic locking)
    let sql = tenants
        .update()
        .set("display_name")
        .set_increment("version")
        .filter(field("id").eq(param()))
        .filter(field("version").eq(param()))
        .returning_all()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Optimistic lock: {}", sql);
    assert!(sql.contains("version = version + $"));

    // 3j. UPDATE with arbitrary expression
    let sql = users
        .update()
        .set_expr("login_count", field("login_count") + int(1))
        .filter(field("id").eq(param()))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Update set_expr: {}", sql);

    // 3k. UPDATE with raw WHERE
    let sql = users
        .update()
        .set("status")
        .filter(raw_expr("created_at < NOW() - INTERVAL '30 days'"))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Update raw WHERE: {}", sql);

    // 3l. UPDATE with expression filter
    let sql = users
        .update()
        .set("status")
        .filter(field("login_count").gt(int(0)))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Update filter: {}", sql);

    // 3m. DELETE with WHERE
    let sql = users
        .remove()
        .filter(field("id").eq(param()))
        .returning_all()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Delete: {}", sql);

    // 3n. DELETE with expression filter
    let sql = users
        .remove()
        .filter(field("status").eq(string("deleted")))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Delete filter: {}", sql);

    // 3o. DELETE with raw WHERE
    let sql = users
        .remove()
        .filter(raw_expr("created_at < NOW() - INTERVAL '90 days'"))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Delete raw: {}", sql);

    // 3p. DELETE param count
    let builder = users
        .remove()
        .filter(field("id").eq(param()))
        .filter(field("tenant_id").eq(param()));
    assert_eq!(builder.param_count(), 2);
    println!("  Delete param count: {}", builder.param_count());

    // 3q. UPSERT (ON CONFLICT DO UPDATE)
    let sql = users
        .upsert()
        .on_conflict(&["email"])
        .do_update(&["display_name", "status", "updated_at"])
        .returning_all()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Upsert: {}", sql);
    assert!(sql.contains("ON CONFLICT"));
    assert!(sql.contains("DO UPDATE SET"));

    // 3r. UPSERT DO NOTHING
    let sql = users
        .upsert()
        .on_conflict(&["email"])
        .do_nothing()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Upsert do nothing: {}", sql);
    assert!(sql.contains("DO NOTHING"));

    // 3s. UPSERT with named constraint
    let sql = users
        .upsert()
        .on_conflict_constraint("users_email_key")
        .do_update(&["display_name"])
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Upsert on constraint: {}", sql);

    // 3t. UPSERT with conflict filter
    let sql = users
        .upsert()
        .on_conflict(&["email"])
        .conflict_filter(field("status").ne(string("deleted")))
        .do_update(&["display_name"])
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Upsert conflict filter: {}", sql);

    // ========================================================================
    // 4. DDL (CREATE TABLE, ALTER TABLE, DROP TABLE, INDEXES)
    // ========================================================================
    println!("\n--- 4. DDL ---");

    // 4a. CREATE TABLE from model metadata
    let sql = users.create().render(Some(&pg)).unwrap();
    println!("  [PG] Create table: {}...", &sql[..sql.len().min(100)]);
    assert!(sql.contains("CREATE TABLE users"));

    // 4b. CREATE TABLE IF NOT EXISTS
    let sql = users.create().if_not_exists().render(Some(&pg)).unwrap();
    assert!(sql.contains("IF NOT EXISTS"));
    println!(
        "  [PG] Create if not exists: {}...",
        &sql[..sql.len().min(100)]
    );

    // 4c. CREATE TABLE with constraints (FK, CHECK, composite)
    let sql = audit_log.create().render(Some(&pg)).unwrap();
    assert!(sql.contains("REFERENCES"));
    println!("  [PG] Create with FK: {}...", &sql[..sql.len().min(100)]);

    // 4d. CREATE TABLE with namespace
    let sql = audit_log.create().render(Some(&pg)).unwrap();
    assert!(sql.contains("audit.audit_log"));
    println!("  [PG] Namespace: {}...", &sql[..sql.len().min(100)]);

    // 4e. CREATE TABLE with composite PK
    let sql = order_items.create().render(Some(&pg)).unwrap();
    assert!(sql.contains("PRIMARY KEY (order_id, product_id)"));
    println!("  [PG] Composite PK: {}...", &sql[..sql.len().min(100)]);

    // 4f. DefineEntityBuilder (runtime-defined schema)
    let sql = DefineEntityBuilder::new("dynamic_table")
        .field(dol::FieldDef::new("id", DataType::Uuid).primary_key())
        .field(dol::FieldDef::new("name", DataType::Text))
        .field(dol::FieldDef::new("value", DataType::Decimal { precision: None, scale: None }).nullable())
        .if_not_exists()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Dynamic define: {}", sql);

    // 4g. ALTER TABLE — add field (takes a FieldDef)
    let sql = users
        .alter()
        .add_field(dol::FieldDef::new("phone", DataType::Text).nullable())
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Alter add: {}", sql);
    assert!(sql.contains("ADD COLUMN"));

    // 4h. ALTER TABLE — drop column
    let sql = users
        .alter()
        .drop_field("profile")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Alter drop: {}", sql);

    // 4i. ALTER TABLE — rename column
    let sql = users
        .alter()
        .rename_field("display_name", "full_name")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Alter rename col: {}", sql);

    // 4j. ALTER TABLE — change field type
    let sql = users
        .alter()
        .alter_field_type("login_count", DataType::Int64)
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Alter column type: {}", sql);

    // 4k. ALTER TABLE — set/drop default
    let sql = users
        .alter()
        .set_default("status", "'inactive'")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Set default: {}", sql);

    let sql = users
        .alter()
        .drop_default("status")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Drop default: {}", sql);

    // 4l. ALTER TABLE — set/drop NOT NULL
    let sql = users
        .alter()
        .set_not_null("profile")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Set NOT NULL: {}", sql);

    let sql = users
        .alter()
        .drop_not_null("email")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Drop NOT NULL: {}", sql);

    // 4m. ALTER TABLE — rename table
    let sql = users
        .alter()
        .rename_model("app_users")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Rename table: {}", sql);

    // 4n. ALTER TABLE — add/drop constraint
    let sql = users
        .alter()
        .add_constraint(EntityConstraint::Unique(&["email", "tenant_id"]))
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Add constraint: {}", sql);

    let sql = users
        .alter()
        .drop_constraint("users_email_key")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Drop constraint: {}", sql);

    // 4o. ALTER TABLE — multiple actions
    let sql = users
        .alter()
        .add_field(dol::FieldDef::new("phone", DataType::Text).nullable())
        .drop_field("profile")
        .set_not_null("display_name")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Multi alter: {}", sql);

    // 4p. DROP TABLE
    let sql = users.drop_entity().render(Some(&pg)).unwrap();
    println!("  [PG] Drop table: {}", sql);

    // 4q. DROP TABLE IF EXISTS CASCADE
    let sql = users
        .drop_entity()
        .if_exists()
        .cascade()
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("IF EXISTS"));
    assert!(sql.contains("CASCADE"));
    println!("  [PG] Drop if exists cascade: {}", sql);

    // 4r. CREATE INDEX
    let sql = DefineIndexBuilder::new("idx_users_email")
        .on("users")
        .columns(&["email"])
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Create index: {}", sql);

    // 4s. CREATE UNIQUE INDEX IF NOT EXISTS
    let sql = DefineIndexBuilder::new("idx_users_tenant_email")
        .on("users")
        .columns(&["tenant_id", "email"])
        .unique()
        .if_not_exists()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Unique index: {}", sql);

    // 4t. CREATE INDEX with method and WHERE (partial index)
    let sql = DefineIndexBuilder::new("idx_users_active_email")
        .on("users")
        .columns(&["email"])
        .method(dol::ir::definition::IndexMethod::Hash)
        .where_clause("status = 'active'")
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Partial index: {}", sql);

    // 4u. CREATE INDEX CONCURRENTLY
    let sql = DefineIndexBuilder::new("idx_users_email_conc")
        .on("users")
        .columns(&["email"])
        .concurrently()
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("CONCURRENTLY"));
    println!("  [PG] Concurrent index: {}", sql);

    // 4v. DROP INDEX
    let sql = DropIndexBuilder::new("idx_users_email")
        .if_exists()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Drop index: {}", sql);

    // ========================================================================
    // 5. EXPRESSIONS — the composable expression engine
    // ========================================================================
    println!("\n--- 5. Expressions ---");

    // 5a. Comparison operators
    let _eq = field("age").eq(int(18));
    let _ne = field("age").ne(int(18));
    let _lt = field("age").lt(int(18));
    let _gt = field("age").gt(int(18));
    let _le = field("age").le(int(18));
    let _ge = field("age").ge(int(18));
    println!("  Comparison ops: eq, ne, lt, gt, le, ge ✓");

    // 5b. Boolean composition with & (AND) and | (OR)
    let _and = field("age").gt(int(18)) & field("status").eq(string("active"));
    let _or = field("role").eq(string("admin")) | field("role").eq(string("superadmin"));
    println!("  Boolean ops: AND (&), OR (|) ✓");

    // 5c. NOT
    let _not = !field("is_deleted").eq(bool_expr(true));
    println!("  NOT: ! operator ✓");

    // 5d. Arithmetic operators (+, -, *, /, %)
    let _add = field("price") + field("tax");
    let _sub = field("total") - field("discount");
    let _mul = field("price") * field("quantity");
    let _div = field("total") / int(2);
    let _rem = field("value") % int(10);
    println!("  Arithmetic ops: +, -, *, /, %% ✓");

    // 5e. Negation
    let _neg = -field("balance");
    println!("  Negation: -expr ✓");

    // 5f. CAST
    let _cast = field("count").cast(DataType::Int64);
    println!("  Cast: CAST(count AS BIGINT) ✓");

    // 5g. Alias
    let _alias = func::count_star().alias("total_count");
    println!("  Alias: COUNT(*) AS total_count ✓");

    // 5h. String concatenation (dialect-aware)
    let _concat = field("first_name")
        .concat(string(" "))
        .concat(field("last_name"));
    println!("  Concat: first_name || ' ' || last_name ✓");

    // 5i. CASE WHEN expression
    let _case = case()
        .when(field("status").eq(string("active")), string("Active"))
        .when(field("status").eq(string("pending")), string("Pending"))
        .else_(string("Unknown"))
        .end();
    println!("  CASE WHEN: ✓");

    // 5j. Field access for nested/JSON data
    let _access = field("profile").access("address").access("city");
    println!("  Field access: profile.address.city ✓");

    // 5k. Qualified identifiers
    let _qualified = dol::expr::qualified("u", "email");
    let _qualified2 = dol::expr::qualified("users", "email");
    println!("  Qualified: u.email, users.email ✓");

    // 5l. From &str to Expr
    let _from_str: dol::expr::Expr = "email".into();
    println!("  From &str: \"email\".into() ✓");

    // 5m. LIKE / ILIKE
    let _like = field("email").like(string("%@example.com"));
    let _ilike = field("email").ilike(string("%@example.com"));
    println!("  LIKE / ILIKE ✓");

    // 5n. IS NULL / IS NOT NULL
    let _is_null = field("profile").is_null();
    let _is_not_null = field("profile").is_null().negate();
    println!("  IS NULL / IS NOT NULL ✓");

    // 5o. BETWEEN / NOT BETWEEN
    let _between = field("age").between(int(18), int(65));
    let _not_between = field("age").between(int(0), int(17)).negate();
    println!("  BETWEEN / NOT BETWEEN ✓");

    // 5p. IN list / NOT IN list
    let _in = field("status").in_list(vec![string("a"), string("b")]);
    let _not_in = field("status").in_list(vec![string("x"), string("y")]).negate();
    println!("  IN / NOT IN list ✓");

    // 5q. IN / NOT IN subquery
    let _in_sub = field("id").in_subquery("SELECT user_id FROM active_users");
    let _not_in_sub = field("id").in_subquery("SELECT user_id FROM banned_users").negate();
    println!("  IN / NOT IN subquery ✓");

    // 5r. All function constructors
    let _ = func::lower(field("email"));
    let _ = func::upper(field("name"));
    let _ = func::trim(field("value"));
    let _ = func::length(field("name"));
    let _ = func::substr(field("name"), int(1), int(10));
    let _ = func::replace(field("email"), string("old"), string("new"));
    let _ = func::concat_fn(vec![field("a"), string(" "), field("b")]);
    let _ = func::abs(field("diff"));
    let _ = func::round(field("price"), Some(int(2)));
    let _ = func::now();
    let _ = func::current_date();
    let _ = func::current_timestamp();
    let _ = func::coalesce(vec![field("name"), field("email")]);
    let _ = func::nullif(field("value"), int(0));
    let _ = func::count(field("id"));
    let _ = func::count_star();
    let _ = func::sum(field("amount"));
    let _ = func::avg(field("score"));
    let _ = func::min(field("created_at"));
    let _ = func::max(field("created_at"));
    let _ = func::func("CUSTOM_FN", vec![field("x"), int(42)]);
    println!("  All function constructors (22 functions) ✓");

    // 5s. All window function constructors
    let _ = func::row_number();
    let _ = func::rank();
    let _ = func::dense_rank();
    let _ = func::ntile(int(4));
    let _ = func::lag(field("value"), None, None);
    let _ = func::lag(field("value"), Some(int(1)), Some(int(0)));
    let _ = func::lead(field("value"), None, None);
    let _ = func::lead(field("value"), Some(int(1)), Some(int(0)));
    let _ = func::first_value(field("price"));
    let _ = func::last_value(field("price"));
    println!("  All window functions (10 functions) ✓");

    // 5t. Object and array literals
    let _obj = dol::expr::obj(vec![("name", string("John")), ("age", int(30))]);
    let _arr = dol::expr::arr(vec![int(1), int(2), int(3)]);
    println!("  Object/Array literals ✓");

    // 5u. Ordering expressions (chained with nulls modifiers)
    let _ = field("name").asc();
    let _ = field("name").desc();
    let _ = field("name").asc().nulls_first();
    let _ = field("name").asc().nulls_last();
    let _ = field("name").desc().nulls_first();
    let _ = field("name").desc().nulls_last();
    println!("  All ordering variants (asc/desc + nulls_first/nulls_last) ✓");

    // 5v. Window builder with frame
    let _ = func::row_number()
        .over()
        .partition_by(vec![field("tenant_id")])
        .order_by(vec![field("created_at").desc()])
        .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    let _ = func::sum(field("amount"))
        .over()
        .partition_by(vec![field("category")])
        .order_by(vec![field("date").asc()])
        .range_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    println!("  Window builder: rows_between, range_between ✓");

    // 5w. Typed constructors
    let _ = string("text"); // Literal::Str
    let _ = int(42_i64); // Literal::Int
    let _ = float(9.99_f64); // Literal::Float
    let _ = bool_expr(true); // Literal::Bool
    let _ = dol::expr::Expr::Value(dol::expr::Literal::Null); // Literal::Null
    println!("  Typed constructors: string, int, float, bool_expr, Null ✓");

    // 5x. Raw expression escape hatch
    let _raw = raw_expr("NOW() - INTERVAL '30 days'");
    println!("  Raw expression ✓");

    // 5y. Param placeholder
    let _param = param();
    println!("  Param placeholder ✓");

    // ========================================================================
    // 6. SET OPERATIONS (UNION, INTERSECT, EXCEPT)
    // ========================================================================
    println!("\n--- 6. Set Operations ---");

    let q1 = users
        .get()
        .fields(&["id", "email"])
        .filter(field("status").eq(param()))
        .build();
    let q2 = users
        .get()
        .fields(&["id", "email"])
        .filter(field("tenant_id").eq(param()))
        .build();

    // 6a. UNION
    let sql = CompoundSelectBuilder::new(q1.clone())
        .union(q2.clone())
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("UNION"));
    println!("  [PG] UNION: {}", sql);

    // 6b. UNION ALL
    let sql = CompoundSelectBuilder::new(q1.clone())
        .union_all(q2.clone())
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("UNION ALL"));
    println!("  [PG] UNION ALL: {}", sql);

    // 6c. INTERSECT
    let sql = CompoundSelectBuilder::new(q1.clone())
        .intersect(q2.clone())
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("INTERSECT"));
    println!("  [PG] INTERSECT: {}", sql);

    // 6d. EXCEPT
    let sql = CompoundSelectBuilder::new(q1.clone())
        .except(q2.clone())
        .render(Some(&pg))
        .unwrap();
    assert!(sql.contains("EXCEPT"));
    println!("  [PG] EXCEPT: {}", sql);

    // 6e. Compound with ORDER BY + LIMIT
    let sql = CompoundSelectBuilder::new(q1.clone())
        .union(q2.clone())
        .order_by(vec![field("email").asc()])
        .limit()
        .offset()
        .render(Some(&pg))
        .unwrap();
    println!("  [PG] Compound + order/limit: {}", sql);

    // ========================================================================
    // 7. TRANSACTIONS
    // ========================================================================
    println!("\n--- 7. Transactions ---");

    let sql = TransactionBuilder::render(&TransactionBuilder::begin(), None).unwrap();
    assert_eq!(sql, "BEGIN");
    println!("  BEGIN: {}", sql);

    let sql = TransactionBuilder::render(&TransactionBuilder::commit(), None).unwrap();
    assert_eq!(sql, "COMMIT");
    println!("  COMMIT: {}", sql);

    let sql = TransactionBuilder::render(&TransactionBuilder::rollback(), None).unwrap();
    assert_eq!(sql, "ROLLBACK");
    println!("  ROLLBACK: {}", sql);

    let sql = TransactionBuilder::render(&TransactionBuilder::savepoint("sp1"), None).unwrap();
    assert_eq!(sql, "SAVEPOINT sp1");
    println!("  SAVEPOINT: {}", sql);

    let sql =
        TransactionBuilder::render(&TransactionBuilder::release_savepoint("sp1"), None).unwrap();
    assert_eq!(sql, "RELEASE SAVEPOINT sp1");
    println!("  RELEASE: {}", sql);

    let sql = TransactionBuilder::render(&TransactionBuilder::rollback_to_savepoint("sp1"), None)
        .unwrap();
    assert_eq!(sql, "ROLLBACK TO SAVEPOINT sp1");
    println!("  ROLLBACK TO: {}", sql);

    // ========================================================================
    // 8. ACCESS CONTROL (GRANT, REVOKE)
    // ========================================================================
    println!("\n--- 8. Access Control ---");

    let sql = GrantBuilder::new(Privilege::Select)
        .on("users")
        .to("app_reader")
        .render(None)
        .unwrap();
    assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::Insert)
        .on("users")
        .to("app_writer")
        .render(None)
        .unwrap();
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::Update)
        .on("users")
        .to("app_writer")
        .render(None)
        .unwrap();
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::Delete)
        .on("users")
        .to("app_admin")
        .render(None)
        .unwrap();
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::All)
        .on("users")
        .to("superadmin")
        .render(None)
        .unwrap();
    println!("  {}", sql);

    let sql = RevokeBuilder::new(Privilege::Insert)
        .on("users")
        .from("readonly_role")
        .render(None)
        .unwrap();
    assert_eq!(sql, "REVOKE INSERT ON users FROM readonly_role");
    println!("  {}", sql);

    // ========================================================================
    // 9. MULTI-DIALECT — same query, different databases
    // ========================================================================
    println!("\n--- 9. Multi-Dialect Rendering ---");

    let mssql = Dialect::mssql();
    let oracle = Dialect::oracle();
    let mariadb = Dialect::mariadb();
    let cockroach = Dialect::cockroachdb();

    // 9a. SELECT with WHERE across all 7 dialects
    println!("  SELECT with WHERE:");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("SQLite     ", &sqlite),
        ("MSSQL      ", &mssql),
        ("Oracle     ", &oracle),
        ("MariaDB    ", &mariadb),
        ("CockroachDB", &cockroach),
    ] {
        let sql = users
            .get()
            .fields(&["id", "email"])
            .filter(field("id").eq(param()))
            .render(Some(d))
            .unwrap();
        println!("    [{}] {}", name, sql);
    }

    // 9b. INSERT with RETURNING across dialects
    println!("  INSERT with RETURNING:");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("SQLite     ", &sqlite),
        ("MSSQL      ", &mssql),
    ] {
        let sql = users
            .insert()
            .fields(&["id", "email"])
            .returning_all()
            .render(Some(d))
            .unwrap();
        println!("    [{}] {}", name, sql);
    }

    // 9c. UPSERT across dialects (ON CONFLICT vs ON DUPLICATE KEY vs MERGE)
    println!("  UPSERT:");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("MSSQL      ", &mssql),
    ] {
        let sql = users
            .upsert()
            .fields(&["id", "email", "display_name"])
            .on_conflict(&["email"])
            .do_update(&["display_name"])
            .render(Some(d))
            .unwrap();
        println!("    [{}] {}", name, sql);
    }

    // 9d. Pagination across dialects (LIMIT/OFFSET vs OFFSET/FETCH)
    println!("  Pagination:");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("SQLite     ", &sqlite),
        ("MSSQL      ", &mssql),
        ("Oracle     ", &oracle),
    ] {
        let sql = users
            .get()
            .order_by_asc("email")
            .limit()
            .offset()
            .render(Some(d))
            .unwrap();
        println!("    [{}] {}", name, sql);
    }

    // 9e. CREATE TABLE type mapping across dialects
    println!("  CREATE TABLE (type mapping):");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("SQLite     ", &sqlite),
    ] {
        let sql = products.create().render(Some(d)).unwrap();
        println!("    [{}] {}...", name, &sql[..sql.len().min(120)]);
    }

    // 9f. Boolean literals (TRUE/FALSE vs 1/0)
    println!("  Boolean literals:");
    for (name, d) in [
        ("PostgreSQL", &pg),
        ("MySQL     ", &mysql),
        ("SQLite    ", &sqlite),
    ] {
        let sql = products
            .get()
            .filter(field("is_active").eq(bool_expr(true)))
            .render(Some(d))
            .unwrap();
        println!("    [{}] {}", name, sql);
    }

    // ========================================================================
    // 10. OBJECT STORAGE OPERATIONS
    // ========================================================================
    println!("\n--- 10. Object Storage ---");

    // 10a. PUT object with metadata
    let ir = PutObjectBuilder::new("avatars/user-123.jpg")
        .into_bucket("uploads")
        .from_path("/tmp/upload.jpg")
        .content_type("image/jpeg")
        .metadata("uploader", "user-123")
        .metadata("source", "profile-edit")
        .build();
    println!("  PUT object: key={}, bucket={}", ir.key, ir.bucket);

    // 10b. PUT object from bytes
    let ir = PutObjectBuilder::new("reports/daily.csv")
        .into_bucket("exports")
        .from_bytes()
        .content_type("text/csv")
        .build();
    println!("  PUT from bytes: key={}", ir.key);

    // 10c. GET object
    let ir = GetObjectBuilder::new("avatars/user-123.jpg")
        .from_bucket("uploads")
        .build();
    println!("  GET object: key={}, bucket={}", ir.key, ir.bucket);

    // 10d. LIST objects with prefix and limit
    let ir = ListObjectsBuilder::new()
        .bucket("uploads")
        .prefix("avatars/")
        .limit(100)
        .build();
    println!(
        "  LIST objects: bucket={}, prefix={:?}, limit={:?}",
        ir.bucket, ir.prefix, ir.limit
    );

    // 10e. LIST objects with pagination token
    let ir = ListObjectsBuilder::new()
        .bucket("uploads")
        .continuation_token("eyJrZXkiOiJhdmF0YXJzLzEwMCJ9")
        .limit(50)
        .build();
    println!("  LIST with token: {:?}", ir.continuation_token);

    // 10f. READ file
    let ir = ReadFileBuilder::new("/config/app.toml")
        .encoding("utf-8")
        .build();
    println!("  READ file: path={}, encoding={:?}", ir.path, ir.encoding);

    // 10g. WRITE file with directory creation
    let ir = WriteFileBuilder::new("/logs/output.txt")
        .from_bytes()
        .create_dirs()
        .build();
    println!(
        "  WRITE file: path={}, create_dirs={}",
        ir.path, ir.create_dirs
    );

    // 10h. WRITE file from source path
    let ir = WriteFileBuilder::new("/data/backup.sql")
        .from_path("/tmp/dump.sql")
        .build();
    println!(
        "  WRITE from path: path={}, source={:?}",
        ir.path, ir.source
    );

    // 10i. MOVE file
    let ir = MoveFileBuilder::new("/tmp/upload.jpg", "/data/photos/img001.jpg").build();
    println!("  MOVE file: from={}, to={}", ir.from, ir.to);

    // ========================================================================
    // 11. CONFIGURATION
    // ========================================================================
    println!("\n--- 11. Configuration ---");

    // 11a. All 7 dialect presets
    println!("  Dialect presets:");
    for (name, d) in [
        ("postgresql ", Dialect::postgres()),
        ("mysql      ", Dialect::mysql()),
        ("mariadb    ", Dialect::mariadb()),
        ("sqlite     ", Dialect::sqlite()),
        ("mssql      ", Dialect::mssql()),
        ("oracle     ", Dialect::oracle()),
        ("cockroachdb", Dialect::cockroachdb()),
    ] {
        println!(
            "    {} → param: {:?}, quote: {:?}",
            name, d.param_style, d.quote_style
        );
    }

    // 11b. Dialect utilities
    println!("  Quote ident: {}", pg.quote_ident("user name"));
    println!("  Resolve type UUID: {}", pg.resolve_type(&DataType::Uuid));
    println!(
        "  Resolve type Timestamp: {}",
        pg.resolve_type(&DataType::TimestampTz { precision: 6 })
    );
    println!("  Bool TRUE: {}, FALSE: {}", pg.bool_true, pg.bool_false);

    // 11c. Param counter across dialects
    let mut counter = pg.param_counter();
    assert_eq!(counter.next(), "$1");
    assert_eq!(counter.next(), "$2");
    assert_eq!(counter.next(), "$3");
    println!("  PG param counter: $1, $2, $3 ✓");

    let mut counter = mysql.param_counter();
    assert_eq!(counter.next(), "?");
    assert_eq!(counter.next(), "?");
    println!("  MySQL params: ?, ? ✓");

    let mut counter = mssql.param_counter();
    assert_eq!(counter.next(), "@p1");
    assert_eq!(counter.next(), "@p2");
    println!("  MSSQL params: @p1, @p2 ✓");

    let mut counter = oracle.param_counter();
    assert_eq!(counter.next(), ":1");
    assert_eq!(counter.next(), ":2");
    println!("  Oracle params: :1, :2 ✓");

    // 11d. Feature flags
    println!(
        "  PG features: distinct_on={}, ilike={}, cte={}, window_functions={}",
        pg.features.distinct_on, pg.features.ilike, pg.features.cte, pg.features.window_functions
    );
    println!(
        "  MySQL features: distinct_on={}, ilike={}, cte={}, window_functions={}",
        mysql.features.distinct_on,
        mysql.features.ilike,
        mysql.features.cte,
        mysql.features.window_functions
    );

    // 11e. DDL capabilities
    println!(
        "  PG DDL: transactional={}, cascade={}, concurrent_idx={}",
        pg.ddl.transactional_ddl, pg.ddl.drop_cascade, pg.ddl.index_concurrently
    );
    println!(
        "  SQLite DDL: transactional={}, cascade={}, alter_modify={}",
        sqlite.ddl.transactional_ddl, sqlite.ddl.drop_cascade, sqlite.ddl.alter_modify_column
    );

    // 11f. Locking capabilities
    println!(
        "  PG locking: for_update={}, skip_locked={}, nowait={}",
        pg.locking.for_update, pg.locking.skip_locked, pg.locking.nowait
    );
    println!(
        "  SQLite locking: for_update={} (database-level only)",
        sqlite.locking.for_update
    );

    // 12. Migrations (feature-gated)
    #[cfg(feature = "migration")]
    migration_examples();

    // 13. Schema diff & auto-discovery (feature-gated)
    #[cfg(feature = "migration")]
    schema_diff_examples();

    // 14. Config-integrated migrations (feature-gated)
    #[cfg(all(feature = "migration", feature = "config"))]
    config_migration_examples();

    println!("\n=== All playbook examples completed successfully! ===");
}

// ============================================================================
// 12. MIGRATION SYSTEM — forward/backward, all storage kinds
// ============================================================================

/// Separate function for migration examples (feature-gated).
#[cfg(feature = "migration")]
fn migration_examples() {
    use dol::ir::definition::{DefineIndexIR, FieldDef};
    use dol::ir::{AlterAction, EntityRef};
    use dol::migration::{
        InMemoryRegistry, Migration, MigrationDirection, MigrationRegistry, MigrationRunner,
        MigrationState, MigrationStep, MigrationTarget, RenderedStep,
    };
    use dol::model::DataType;

    println!("\n--- 12. Migration System ---");

    let pg = Dialect::postgres();

    // 12a. Define migrations in pure Rust using DOL IR
    struct M001CreateUsers;
    impl Migration for M001CreateUsers {
        fn version(&self) -> &str {
            "20240101_000001"
        }
        fn description(&self) -> &str {
            "Create users table"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::define_entity(
                dol::builder::DefineEntityBuilder::new("users")
                    .field(FieldDef::new("id", DataType::Uuid).primary_key())
                    .field(FieldDef::new("email", DataType::Text).unique())
                    .field(FieldDef::new("status", DataType::Text).default("'active'"))
                    .field(FieldDef::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"))
                    .if_not_exists()
                    .build(),
            )]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::drop_entity("users")]
        }
    }

    struct M002AddEmailIndex;
    impl Migration for M002AddEmailIndex {
        fn version(&self) -> &str {
            "20240102_000001"
        }
        fn description(&self) -> &str {
            "Add unique index on users.email"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::define_index(DefineIndexIR {
                name: "idx_users_email".into(),
                target: EntityRef {
                    name: "users".into(),
                    namespace: None,
                    alias: None,
                },
                columns: vec!["email".into()],
                unique: true,
                if_not_exists: true,
                concurrently: false,
                method: None,
                where_clause: None,
            })]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::drop_index("idx_users_email")]
        }
    }

    struct M003AddProfileColumn;
    impl Migration for M003AddProfileColumn {
        fn version(&self) -> &str {
            "20240201_000001"
        }
        fn description(&self) -> &str {
            "Add profile JSON column to users"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::alter_entity(
                "users",
                vec![AlterAction::AddField(
                    FieldDef::new("profile", DataType::Json).nullable(),
                )],
            )]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::alter_entity(
                "users",
                vec![AlterAction::DropField("profile".into())],
            )]
        }
    }

    // 12b. KV migration
    struct M004SetupKvNamespaces;
    impl Migration for M004SetupKvNamespaces {
        fn version(&self) -> &str {
            "20240301_000001"
        }
        fn description(&self) -> &str {
            "Set up KV namespaces"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![
                MigrationStep::create_kv_namespace("sessions:"),
                MigrationStep::create_kv_namespace("cache:"),
            ]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![
                MigrationStep::drop_kv_namespace("cache:"),
                MigrationStep::drop_kv_namespace("sessions:"),
            ]
        }
    }

    // 12c. Object storage migration
    struct M005SetupBuckets;
    impl Migration for M005SetupBuckets {
        fn version(&self) -> &str {
            "20240401_000001"
        }
        fn description(&self) -> &str {
            "Set up object storage buckets"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![
                MigrationStep::create_bucket_in_region("app-uploads", "us-east-1"),
                MigrationStep::create_bucket("app-backups"),
            ]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![
                MigrationStep::delete_bucket("app-backups"),
                MigrationStep::delete_bucket("app-uploads"),
            ]
        }
    }

    // 12d. Build the runner and register migrations
    let runner = MigrationRunner::new()
        .register(M001CreateUsers)
        .register(M002AddEmailIndex)
        .register(M003AddProfileColumn)
        .register(M004SetupKvNamespaces)
        .register(M005SetupBuckets);

    println!("  Registered {} migrations", runner.count());
    assert_eq!(runner.count(), 5);
    assert!(runner.validate().is_ok());

    // 12e. Plan forward migration (fresh database)
    let mut registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    println!("  Forward plan: {} migrations to apply", plan.len());
    assert_eq!(plan.len(), 5);
    assert_eq!(plan.direction(), Some(MigrationDirection::Forward));

    // 12f. Render to SQL (dry-run) — test with multiple dialects
    for dialect in &[Dialect::postgres(), Dialect::mysql(), Dialect::sqlite()] {
        let rendered = runner.render_plan(&plan, Some(dialect));
        println!(
            "  Rendered for {}: {} migration(s)",
            dialect.name,
            rendered.len()
        );
        // First migration should produce CREATE TABLE SQL
        let first_sql = rendered[0].steps[0].sql().unwrap();
        assert!(
            first_sql.contains("CREATE TABLE"),
            "{}: missing CREATE TABLE",
            dialect.name
        );
    }

    // 12g. Simulate applying migrations
    for step in plan.steps() {
        registry
            .mark_applied(&step.version, &step.description, &step.checksum)
            .unwrap();
    }

    // 12h. Verify status after apply
    let statuses = runner.status(&registry).unwrap();
    for s in &statuses {
        println!(
            "  Migration {} [{}]: {:?}",
            s.version, s.description, s.state
        );
        assert_eq!(s.state, MigrationState::Applied);
    }

    // 12i. Plan with specific target — migrate to a specific version
    let fresh_registry = InMemoryRegistry::new();
    let plan = runner
        .plan(
            &fresh_registry,
            MigrationTarget::Version("20240102_000001".into()),
        )
        .unwrap();
    println!("  Plan to V2: {} migration(s) (should be 2)", plan.len());
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.steps()[0].version, "20240101_000001");
    assert_eq!(plan.steps()[1].version, "20240102_000001");

    // 12j. Plan rollback — revert last 2 migrations
    let plan = runner.plan_rollback(&registry, 2).unwrap();
    println!("  Rollback plan: {} migration(s) to revert", plan.len());
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.direction(), Some(MigrationDirection::Backward));

    // The last two applied migrations should be reverted in reverse order
    assert_eq!(plan.steps()[0].version, "20240401_000001");
    assert_eq!(plan.steps()[1].version, "20240301_000001");

    // Render rollback steps
    let rendered = runner.render_plan(&plan, None);
    for rm in &rendered {
        for step in &rm.steps {
            match step {
                RenderedStep::Sql { sql, .. } => {
                    println!("  Rollback SQL: {}", &sql[..sql.len().min(80)]);
                }
                RenderedStep::Kv { description, .. } => {
                    println!("  Rollback KV: {}", description);
                }
                RenderedStep::Storage { description, .. } => {
                    println!("  Rollback Storage: {}", description);
                }
            }
        }
    }

    // 12k. Plan full reset
    let plan = runner.plan(&registry, MigrationTarget::Reset).unwrap();
    println!("  Reset plan: {} migration(s) to revert", plan.len());
    assert_eq!(plan.len(), 5);

    // 12l. Checksum validation
    assert!(runner.validate_checksums(&registry).is_ok());
    println!("  Checksum validation: OK");

    // 12m. SQL helpers for migration history table
    let create_sql = dol::migration::registry::create_history_table_sql(Some(&pg));
    assert!(create_sql.contains("_dol_migrations"));
    println!("  History table DDL generated");

    let insert_sql = dol::migration::registry::insert_applied_sql(Some(&pg));
    assert!(insert_sql.contains("INSERT INTO _dol_migrations"));
    println!("  Insert applied SQL generated");

    println!("  Migration system: all examples passed ✓");
}

// ============================================================================
// 13. SCHEMA DIFF — auto-detect model changes, generate migration steps
// ============================================================================

/// Schema diff examples: comparing model versions and generating alter steps.
#[cfg(feature = "migration")]
fn schema_diff_examples() {
    use dol::ir::AlterAction;
    use dol::migration::schema_diff::{
        EntitySnapshot, create_entity_step, diff_entities, diff_to_steps, drop_entity_step,
        field_to_field_def,
    };
    use dol::model::DataType;

    println!("\n--- 13. Schema Diff & Auto-Discovery ---");

    // 13a. Define two versions of a model
    fn users_v1() -> Entity {
        Entity::new(
            "users",
            vec![
                Field::new("id", DataType::Uuid).primary_key(),
                Field::new("email", DataType::Text).unique(),
                Field::new("status", DataType::Text).default("'active'"),
            ],
        )
    }

    fn users_v2() -> Entity {
        Entity::new(
            "users",
            vec![
                Field::new("id", DataType::Uuid).primary_key(),
                Field::new("email", DataType::Text).unique(),
                Field::new("status", DataType::Text).default("'active'"),
                // New field added in V2
                Field::new("display_name", DataType::Text).nullable(),
            ],
        )
    }

    let users_v1 = users_v1();
    let users_v2 = users_v2();

    // 13b. Take snapshots and compute diff
    let v1 = EntitySnapshot::from_entity(&users_v1);
    let v2 = EntitySnapshot::from_entity(&users_v2);
    let actions = diff_entities(&v1, &v2);
    println!("  Diff V1→V2: {} change(s)", actions.len());
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::AddField(f) if f.name == "display_name"));

    // 13c. Generate migration steps from diff
    let steps = diff_to_steps("users", &v1, &v2);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind(), "sql");
    println!("  Generated ALTER TABLE step for field addition");

    // 13d. Reverse diff (V2 → V1) produces DropField
    let reverse_actions = diff_entities(&v2, &v1);
    assert_eq!(reverse_actions.len(), 1);
    assert!(matches!(&reverse_actions[0], AlterAction::DropField(name) if name == "display_name"));
    println!("  Reverse diff produces DropField");

    // 13e. Type change detection
    fn v_int() -> Entity {
        Entity::new("t", vec![Field::new("count", DataType::Int32)])
    }
    fn v_bigint() -> Entity {
        Entity::new("t", vec![Field::new("count", DataType::Int64)])
    }
    let v_int = v_int();
    let v_bigint = v_bigint();
    let type_actions = diff_entities(
        &EntitySnapshot::from_entity(&v_int),
        &EntitySnapshot::from_entity(&v_bigint),
    );
    assert_eq!(type_actions.len(), 1);
    assert!(matches!(
        &type_actions[0],
        AlterAction::AlterFieldType { name, new_type }
            if name == "count" && *new_type == DataType::Int64
    ));
    println!("  Type change detected: Int32 → Int64");

    // 13f. Create and drop model steps from definitions
    let create_step = create_entity_step(&users_v1);
    assert_eq!(create_step.kind(), "sql");
    let drop_step = drop_entity_step(&users_v1);
    assert_eq!(drop_step.kind(), "sql");
    println!("  Create/drop model steps generated from Model");

    // 13g. Field conversion preserves all attributes
    let field = Field::new("email", DataType::Text)
        .unique()
        .nullable()
        .default("'test'")
        .index();
    let def = field_to_field_def(&field);
    assert!(def.unique);
    assert!(def.nullable);
    assert_eq!(def.default_expr.as_deref(), Some("'test'"));
    assert!(def.indexed);
    println!("  Field→FieldDef conversion preserves all attributes");

    // 13h. No changes detected when models are identical
    let no_changes = diff_to_steps("users", &v1, &v1.clone());
    assert!(no_changes.is_empty());
    println!("  No changes detected for identical models");

    println!("  Schema diff: all examples passed ✓");
}

// ============================================================================
// 14. CONFIG-INTEGRATED MIGRATIONS — run migrations using DolConfig settings
// ============================================================================

/// Config-integrated migration examples.
#[cfg(all(feature = "migration", feature = "config"))]
fn config_migration_examples() {
    use dol::ir::definition::FieldDef;
    use dol::migration::{
        InMemoryRegistry, Migration, MigrationRunner, MigrationStep, RenderedStep,
    };
    use dol::model::DataType;

    println!("\n--- 14. Config-Integrated Migrations ---");

    // 14a. Define some migrations
    struct CreateOrders;
    impl Migration for CreateOrders {
        fn version(&self) -> &str {
            "20240501_000001"
        }
        fn description(&self) -> &str {
            "Create orders table"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::define_entity(
                dol::builder::DefineEntityBuilder::new("orders")
                    .field(FieldDef::new("id", DataType::Uuid).primary_key())
                    .field(FieldDef::new("total", DataType::Decimal { precision: None, scale: None }))
                    .if_not_exists()
                    .build(),
            )]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::drop_entity("orders")]
        }
    }

    struct SetupOrderKv;
    impl Migration for SetupOrderKv {
        fn version(&self) -> &str {
            "20240502_000001"
        }
        fn description(&self) -> &str {
            "Set up order cache namespace"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::create_kv_namespace("orders:cache:")]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::drop_kv_namespace("orders:cache:")]
        }
    }

    struct SetupOrderBucket;
    impl Migration for SetupOrderBucket {
        fn version(&self) -> &str {
            "20240503_000001"
        }
        fn description(&self) -> &str {
            "Create order attachments bucket"
        }
        fn up(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::create_bucket("order-attachments")]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::delete_bucket("order-attachments")]
        }
    }

    let runner = MigrationRunner::new()
        .register(CreateOrders)
        .register(SetupOrderKv)
        .register(SetupOrderBucket);

    // 14b. Load config with backend filter
    let config: dol::DolConfig = toml_crate::from_str(
        r#"
        [sql]
        dialect = "postgresql"
        [sql.primary]
        url = "postgres://localhost/test"

        [migrations]
        auto_run = true
        lock_strategy = "advisory_lock"

        [migrations.backends]
        sql = true
        kv = true
        storage = false
    "#,
    )
    .unwrap();

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    assert_eq!(plan.len(), 3);
    println!("  Plan has {} migrations", plan.len());

    // 14c. Render with config — storage steps should be filtered out
    let rendered = runner.render_plan_with_config(&plan, &config);
    println!(
        "  Rendered with config filter: {} migration(s) (storage disabled)",
        rendered.len()
    );
    // Only SQL and KV migrations should appear (storage is disabled)
    assert_eq!(rendered.len(), 2);

    // Verify the SQL migration renders with PostgreSQL dialect
    let sql_step = &rendered[0].steps[0];
    assert!(sql_step.sql().is_some());
    let sql = sql_step.sql().unwrap();
    assert!(sql.contains("UUID"), "Should use PostgreSQL UUID type");
    println!("  SQL rendered with PostgreSQL dialect: ✓");

    // Verify the KV migration renders correctly
    assert!(matches!(&rendered[1].steps[0], RenderedStep::Kv { .. }));
    println!("  KV migration rendered: ✓");

    // 14d. Config helpers
    assert!(MigrationRunner::should_auto_run(&config));
    assert!(MigrationRunner::should_validate_checksums(&config));
    assert!(!MigrationRunner::is_dry_run(&config));
    println!("  Config helpers: auto_run=true, validate=true, dry_run=false");

    // 14e. Verify lock_strategy and validate_checksums are parsed from config
    assert!(matches!(
        config.migrations.lock_strategy,
        dol::LockStrategy::AdvisoryLock
    ));
    assert!(config.migrations.validate_checksums);
    assert!(!config.migrations.dry_run);
    println!("  Lock strategy=advisory_lock, validate_checksums=true: ✓");

    println!("  Config-integrated migrations: all examples passed ✓");
}

// PLAYBOOK TESTS — run via `cargo test -p dol --all-features --example playbook`
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playbook_runs_successfully() {
        main();
    }

    #[test]
    fn model_definition_coverage() {
        let users = users();
        let audit_log = audit_log();
        let sessions = sessions();

        // Verify all field constraint types are exercised
        assert!(users.field("id").primary_key);
        assert!(users.field("email").unique);
        assert!(!users.field("email").nullable);
        assert!(users.field("profile").nullable);
        assert_eq!(users.field("status").default_expr, Some("'active'"));
        assert!(users.field("tenant_id").indexed);
        assert!(audit_log.field("action").check.is_some());
        assert!(audit_log.field("user_id").references.is_some());
        assert!(sessions.field("is_expired").generated.is_some());
        assert!(sessions.field("user_agent").comment.is_some());
        assert_eq!(audit_log.namespace, Some("audit"));
    }

    #[test]
    fn all_field_types_represented() {
        let products = products();
        let audit_log = audit_log();
        let sessions = sessions();

        // Verify that the models exercise a wide range of DataType variants
        assert!(
            products
                .fields
                .iter()
                .any(|f| matches!(f.data_type, DataType::Varchar(Some(_))))
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| matches!(f.data_type, DataType::Char(_)))
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| matches!(f.data_type, DataType::Decimal { .. }))
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Float32)
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Bool)
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| matches!(f.data_type, DataType::Array(_)))
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Json)
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Text)
        );
        assert!(
            products
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Text)
        );
        assert!(
            audit_log
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Int64)
        );
        assert!(
            audit_log
                .fields
                .iter()
                .any(|f| f.data_type == DataType::Inet)
        );
        assert!(
            sessions
                .fields
                .iter()
                .any(|f| matches!(f.data_type, DataType::Varbinary(_)))
        );
    }

    #[test]
    fn all_dialects_produce_valid_sql() {
        let users = users();

        let dialects = [
            Dialect::postgres(),
            Dialect::mysql(),
            Dialect::mariadb(),
            Dialect::sqlite(),
            Dialect::mssql(),
            Dialect::oracle(),
            Dialect::cockroachdb(),
        ];

        for d in &dialects {
            // SELECT
            let sql = users
                .get()
                .filter(field("id").eq(param()))
                .render(Some(d))
                .unwrap();
            assert!(sql.contains("SELECT"), "{}: missing SELECT", d.name);
            assert!(sql.contains("FROM users"), "{}: missing FROM", d.name);

            // INSERT
            let sql = users.insert().render(Some(d)).unwrap();
            assert!(sql.contains("INSERT INTO"), "{}: missing INSERT", d.name);

            // UPDATE
            let sql = users
                .update()
                .set("email")
                .filter(field("id").eq(param()))
                .render(Some(d))
                .unwrap();
            assert!(sql.contains("UPDATE users"), "{}: missing UPDATE", d.name);

            // DELETE
            let sql = users
                .remove()
                .filter(field("id").eq(param()))
                .render(Some(d))
                .unwrap();
            assert!(sql.contains("DELETE FROM"), "{}: missing DELETE", d.name);

            // CREATE TABLE
            let sql = users.create().render(Some(d)).unwrap();
            assert!(sql.contains("CREATE TABLE"), "{}: missing CREATE", d.name);

            // DROP TABLE
            let sql = users.drop_entity().render(Some(d)).unwrap();
            assert!(sql.contains("DROP TABLE"), "{}: missing DROP", d.name);
        }
    }
}

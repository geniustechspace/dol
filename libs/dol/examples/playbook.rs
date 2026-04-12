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
use dol::ModelBuilderExt;
use dol::ToSql;
use dol::TransactionSqlExt;
use dol::backend::sql::dialect::Dialect;
use dol::builder::control::{GrantBuilder, Privilege, RevokeBuilder};
use dol::builder::definition::{DefineIndexBuilder, DefineModelBuilder, DropIndexBuilder};
use dol::builder::storage::{
    GetObjectBuilder, ListObjectsBuilder, MoveFileBuilder, PutObjectBuilder, ReadFileBuilder,
    WriteFileBuilder,
};
use dol::builder::transaction::TransactionBuilder;
use dol::expr::window::FrameBound;
use dol::expr::{Direction, case, col, field, func, lit, param, raw_expr};
use dol::ir::LockMode;
use dol::model::{Field, FieldType, FkAction, Model, ModelConstraint};

// ============================================================================
// 1. MODEL DEFINITION — the single source of truth for a data shape
// ============================================================================

/// Users table with all common field constraint types.
static USERS: Model = Model::new(
    "users",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("tenant_id", FieldType::Uuid).index(),
        Field::new("email", FieldType::Text).unique(),
        Field::new("display_name", FieldType::Text),
        Field::new("status", FieldType::Text).default("'active'"),
        Field::new("login_count", FieldType::Int).default("0"),
        Field::new("profile", FieldType::Json).nullable(),
        Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        Field::new("updated_at", FieldType::Timestamp).default("NOW()"),
    ],
);

/// Tenants table with composite constraints and version field.
static TENANTS: Model = Model::new(
    "tenants",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("slug", FieldType::Text).unique(),
        Field::new("display_name", FieldType::Text),
        Field::new("status", FieldType::Text).default("'active'"),
        Field::new("plan", FieldType::Text).default("'free'"),
        Field::new("labels", FieldType::Json).nullable(),
        Field::new("created_at", FieldType::Timestamp),
        Field::new("created_by", FieldType::Uuid),
        Field::new("updated_at", FieldType::Timestamp),
        Field::new("updated_by", FieldType::Uuid),
        Field::new("version", FieldType::Int).default("1"),
    ],
)
.with_constraints(&[ModelConstraint::Unique(&["slug"])]);

/// Sessions table for join examples.
static SESSIONS: Model = Model::new(
    "sessions",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("user_id", FieldType::Uuid).references(
            "users",
            "id",
            FkAction::Cascade,
            FkAction::NoAction,
        ),
        Field::new("token_hash", FieldType::Bytes),
        Field::new("user_agent", FieldType::Text)
            .nullable()
            .comment("Browser / client identifier"),
        Field::new("ip_address", FieldType::Inet).nullable(),
        Field::new("expires_at", FieldType::Timestamp),
        Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        Field::new("is_expired", FieldType::Bool).generated_virtual("expires_at < NOW()"),
    ],
);

/// Audit log with foreign keys, CHECK constraint, and namespace.
static AUDIT_LOG: Model = Model::new(
    "audit_log",
    &[
        Field::new("id", FieldType::BigSerial).primary_key(),
        Field::new("user_id", FieldType::Uuid).references(
            "users",
            "id",
            FkAction::Cascade,
            FkAction::NoAction,
        ),
        Field::new("tenant_id", FieldType::Uuid).references(
            "tenants",
            "id",
            FkAction::Cascade,
            FkAction::NoAction,
        ),
        Field::new("action", FieldType::Text)
            .check("action IN ('create', 'read', 'update', 'delete')"),
        Field::new("resource_type", FieldType::Text),
        Field::new("resource_id", FieldType::Text),
        Field::new("details", FieldType::Json).nullable(),
        Field::new("ip_address", FieldType::Inet).nullable(),
        Field::new("created_at", FieldType::Timestamp).default("NOW()"),
    ],
)
.with_namespace("audit")
.with_constraints(&[ModelConstraint::ForeignKey {
    columns: &["tenant_id"],
    ref_table: "tenants",
    ref_columns: &["id"],
    on_delete: FkAction::Cascade,
}]);

/// Products table with various field types for type-mapping demos.
static PRODUCTS: Model = Model::new(
    "products",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("name", FieldType::Varchar(Some(255))),
        Field::new("sku", FieldType::Char(12)).unique(),
        Field::new("price", FieldType::Decimal),
        Field::new("quantity", FieldType::Int).default("0"),
        Field::new("weight_kg", FieldType::Float).nullable(),
        Field::new("description", FieldType::Text).nullable(),
        Field::new("image_path", FieldType::Path).nullable(),
        Field::new("mime_type", FieldType::Mime).nullable(),
        Field::new("is_active", FieldType::Bool).default("TRUE"),
        Field::new("tags", FieldType::TextArray).nullable(),
        Field::new("metadata", FieldType::Object).nullable(),
        Field::new("created_at", FieldType::Timestamp).default("NOW()"),
    ],
);

/// Orders table with composite primary key.
static ORDER_ITEMS: Model = Model::new(
    "order_items",
    &[
        Field::new("order_id", FieldType::Uuid),
        Field::new("product_id", FieldType::Uuid),
        Field::new("quantity", FieldType::Int),
        Field::new("unit_price", FieldType::Decimal),
    ],
)
.with_constraints(&[
    ModelConstraint::PrimaryKey(&["order_id", "product_id"]),
    ModelConstraint::ForeignKey {
        columns: &["product_id"],
        ref_table: "products",
        ref_columns: &["id"],
        on_delete: FkAction::Restrict,
    },
]);

/// Settings table for aggregate examples.
static SETTINGS: Model = Model::new(
    "tenant_settings",
    &[
        Field::new("tenant_id", FieldType::Uuid).primary_key(),
        Field::new("max_users", FieldType::Int),
        Field::new("mfa_required", FieldType::Bool),
    ],
);

fn main() {
    let pg = Dialect::postgres();
    let mysql = Dialect::mysql();
    let sqlite = Dialect::sqlite();

    println!("=== DOL Playbook — Comprehensive Feature Coverage ===\n");

    // ========================================================================
    // 1. MODEL INTROSPECTION
    // ========================================================================
    println!("--- 1. Model Introspection ---");

    // Field lookup
    let email_field = USERS.field("email");
    assert_eq!(email_field.name, "email");
    assert_eq!(email_field.field_type, FieldType::Text);
    assert!(email_field.unique);
    println!(
        "  Field lookup: users.email -> {:?}",
        email_field.field_type
    );

    // Safe field lookup
    assert!(USERS.try_field("nonexistent").is_none());
    println!("  Safe lookup: try_field('nonexistent') -> None");

    // Primary keys
    let pks: Vec<_> = USERS.primary_keys().map(|f| f.name).collect();
    assert_eq!(pks, vec!["id"]);
    println!("  Primary keys: {:?}", pks);

    // Non-PK fields
    let non_pk_count = USERS.non_pk_fields().count();
    println!("  Non-PK fields: {} fields", non_pk_count);

    // Field list
    let field_list = USERS.field_list();
    assert!(field_list.contains("id"));
    assert!(field_list.contains("email"));
    assert!(field_list.contains(", "));
    println!("  Field list: {}", field_list);

    // Qualified name with namespace
    let qualified = AUDIT_LOG.qualified_name();
    assert_eq!(qualified, "audit.audit_log");
    println!("  Qualified name: {}", qualified);

    // Qualified name without namespace
    let simple_name = USERS.qualified_name();
    assert_eq!(simple_name, "users");
    println!("  Simple name: {}", simple_name);

    // ========================================================================
    // 2. SELECT QUERIES
    // ========================================================================
    println!("\n--- 2. SELECT Queries ---");

    // 2a. Simple SELECT all columns
    let sql = USERS.get().all_columns().to_sql(Some(&pg));
    println!("  [PG] Select all: {}", sql);
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("FROM users"));

    // 2b. SELECT specific columns with WHERE
    let sql = USERS
        .get()
        .columns(&["id", "email", "status"])
        .where_eq("tenant_id")
        .where_eq("status")
        .to_sql(Some(&pg));
    println!("  [PG] Filtered: {}", sql);
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));

    // 2c. WHERE with OR groups
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .filter(field("status").eq(lit("active")) | field("status").eq(lit("pending")))
        .to_sql(Some(&pg));
    println!("  [PG] OR filter: {}", sql);

    // 2d. LIKE / ILIKE patterns
    let sql = USERS
        .get()
        .all_columns()
        .where_ilike("email")
        .to_sql(Some(&pg));
    println!("  [PG] ILIKE: {}", sql);
    assert!(sql.contains("ILIKE"));

    // 2e. NULL checks
    let sql = USERS
        .get()
        .all_columns()
        .filter(field("profile").is_not_null())
        .to_sql(Some(&pg));
    println!("  [PG] IS NOT NULL: {}", sql);

    // 2f. BETWEEN range
    let sql = USERS
        .get()
        .all_columns()
        .filter(field("login_count").between(lit(10), lit(100)))
        .to_sql(Some(&pg));
    println!("  [PG] BETWEEN: {}", sql);

    // 2g. IN list
    let sql = USERS
        .get()
        .all_columns()
        .filter(field("status").in_list(vec![lit("active"), lit("pending"), lit("suspended")]))
        .to_sql(Some(&pg));
    println!("  [PG] IN list: {}", sql);

    // 2h. IN subquery
    let sql = USERS
        .get()
        .all_columns()
        .where_in_subquery(
            "tenant_id",
            "SELECT id FROM tenants WHERE status = 'active'",
        )
        .to_sql(Some(&pg));
    println!("  [PG] IN subquery: {}", sql);

    // 2i. NOT IN subquery (via filter)
    let subquery = "SELECT user_id FROM banned_users";
    let sql = USERS
        .get()
        .all_columns()
        .filter(field("id").not_in_subquery(subquery))
        .to_sql(Some(&pg));
    println!("  [PG] NOT IN subquery: {}", sql);

    // 2j. EXISTS subquery
    let sql = USERS
        .get()
        .all_columns()
        .where_exists("SELECT 1 FROM sessions WHERE sessions.user_id = users.id")
        .to_sql(Some(&pg));
    println!("  [PG] EXISTS: {}", sql);

    // 2k. NOT EXISTS subquery
    let sql = USERS
        .get()
        .all_columns()
        .where_not_exists("SELECT 1 FROM banned WHERE banned.user_id = users.id")
        .to_sql(Some(&pg));
    println!("  [PG] NOT EXISTS: {}", sql);

    // 2l. Scalar subquery in projection
    let sql = USERS
        .get()
        .columns(&["id", "email"])
        .select_item(
            dol::expr::Expr::Subquery(
                "SELECT COUNT(*) FROM sessions WHERE sessions.user_id = users.id".into(),
            )
            .alias("session_count"),
        )
        .to_sql(Some(&pg));
    println!("  [PG] Scalar subquery: {}", sql);

    // 2m. Ordering with NULLS FIRST/LAST
    let sql = USERS
        .get()
        .all_columns()
        .order_by_expr(field("display_name").asc_nulls_last())
        .to_sql(Some(&pg));
    println!("  [PG] ORDER BY NULLS LAST: {}", sql);

    // 2n. ORDER BY with Direction enum
    let sql = USERS
        .get()
        .all_columns()
        .order_by(
            "display_name",
            Direction::Asc,
            Some(dol::expr::NullsPosition::Last),
        )
        .to_sql(Some(&pg));
    println!("  [PG] ORDER BY (Direction): {}", sql);

    // 2o. Pagination (LIMIT + OFFSET)
    let sql = USERS
        .get()
        .all_columns()
        .order_by_asc("email")
        .limit()
        .offset()
        .to_sql(Some(&pg));
    println!("  [PG] Pagination: {}", sql);

    // 2p. DISTINCT
    let sql = USERS
        .get()
        .columns(&["tenant_id"])
        .distinct()
        .to_sql(Some(&pg));
    println!("  [PG] DISTINCT: {}", sql);

    // 2q. DISTINCT ON (PostgreSQL only)
    let sql = USERS
        .get()
        .all_columns()
        .distinct_on(&["tenant_id"])
        .order_by_asc("tenant_id")
        .order_by_desc("created_at")
        .to_sql(Some(&pg));
    println!("  [PG] DISTINCT ON: {}", sql);

    // 2r. GROUP BY + HAVING
    let sql = USERS
        .get()
        .columns(&["tenant_id"])
        .count_all_as("cnt")
        .group_by(&["tenant_id"])
        .having(raw_expr("COUNT(*) > 10"))
        .to_sql(Some(&pg));
    println!("  [PG] GROUP BY HAVING: {}", sql);

    // 2s. Aggregate functions
    let sql = SETTINGS
        .get()
        .select_item(func::sum(col("max_users")).alias("total_users"))
        .to_sql(Some(&pg));
    println!("  [PG] SUM: {}", sql);

    let sql = USERS
        .get()
        .count_all_as("total")
        .where_eq("tenant_id")
        .to_sql(Some(&pg));
    println!("  [PG] COUNT: {}", sql);

    // 2t. Column aliases and raw expressions
    let sql = USERS
        .get()
        .column_as("email", "user_email")
        .to_sql(Some(&pg));
    println!("  [PG] Column alias: {}", sql);

    let sql = USERS
        .get()
        .raw_column("COALESCE(display_name, email) AS name")
        .where_eq("id")
        .to_sql(Some(&pg));
    println!("  [PG] Raw column: {}", sql);

    // 2u. INNER JOIN
    let sql = USERS
        .get()
        .all_columns()
        .inner_join(&TENANTS, &[("tenant_id", "id")])
        .where_eq("tenant_id")
        .to_sql(Some(&pg));
    println!("  [PG] INNER JOIN: {}", sql);

    // 2v. LEFT JOIN
    let sql = USERS
        .get()
        .all_columns()
        .left_join(&SESSIONS, &[("id", "user_id")])
        .to_sql(Some(&pg));
    println!("  [PG] LEFT JOIN: {}", sql);

    // 2w. Multiple JOINs
    let sql = USERS
        .get()
        .all_columns()
        .inner_join(&TENANTS, &[("tenant_id", "id")])
        .left_join(&SESSIONS, &[("id", "user_id")])
        .where_eq("tenant_id")
        .to_sql(Some(&pg));
    println!("  [PG] Multi JOIN: {}", sql);

    // 2x. FOR UPDATE (pessimistic locking)
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("id")
        .lock(LockMode::ForUpdate)
        .to_sql(Some(&pg));
    println!("  [PG] FOR UPDATE: {}", sql);
    assert!(sql.contains("FOR UPDATE"));

    // 2y. FOR UPDATE SKIP LOCKED (queue pattern)
    let sql = USERS
        .get()
        .all_columns()
        .where_eq("status")
        .lock(LockMode::ForUpdateSkipLocked)
        .limit()
        .to_sql(Some(&pg));
    println!("  [PG] SKIP LOCKED: {}", sql);

    // 2z. Window functions — ROW_NUMBER
    let sql = USERS
        .get()
        .all_columns()
        .select_item(
            func::row_number()
                .over()
                .partition_by(vec![col("tenant_id")])
                .order_by(vec![field("created_at").desc()])
                .build()
                .alias("row_num"),
        )
        .to_sql(Some(&pg));
    println!("  [PG] ROW_NUMBER: {}", sql);

    // 2aa. Window functions — RANK with frame
    let sql = USERS
        .get()
        .select_item(col("id"))
        .select_item(col("login_count"))
        .select_item(
            func::rank()
                .over()
                .partition_by(vec![col("tenant_id")])
                .order_by(vec![field("login_count").desc()])
                .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
                .build()
                .alias("login_rank"),
        )
        .to_sql(Some(&pg));
    println!("  [PG] RANK + frame: {}", sql);

    // 2ab. Table alias
    let sql = USERS
        .get()
        .alias("u")
        .all_columns()
        .where_eq("id")
        .to_sql(Some(&pg));
    println!("  [PG] Table alias: {}", sql);

    // 2ac. Param count tracking
    let q = USERS
        .get()
        .all_columns()
        .where_eq("tenant_id")
        .where_eq("status")
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
    let sql = USERS
        .insert()
        .all_columns()
        .returning_all()
        .to_sql(Some(&pg));
    println!("  [PG] Insert all: {}", sql);
    assert!(sql.contains("INSERT INTO users"));
    assert!(sql.contains("RETURNING *"));

    // 3b. INSERT specific columns
    let sql = USERS
        .insert()
        .columns(&["id", "email", "tenant_id", "display_name"])
        .returning(&["id", "email"])
        .to_sql(Some(&pg));
    println!("  [PG] Insert specific: {}", sql);

    // 3c. Batch INSERT (multiple rows)
    let sql = USERS
        .insert()
        .columns(&["id", "email", "tenant_id"])
        .rows(3)
        .to_sql(Some(&pg));
    println!("  [PG] Batch insert: {}", sql);
    // 3 rows × 3 cols = 9 params
    assert!(sql.contains("$9"));

    // 3d. INSERT param count tracking
    let builder = USERS
        .insert()
        .columns(&["id", "email", "tenant_id"])
        .rows(5);
    assert_eq!(builder.param_count(), 15); // 5 × 3
    println!("  Param count (5×3): {}", builder.param_count());

    // 3e. INSERT ... SELECT
    let source_query = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("tenant_id")
        .to_sql(Some(&pg));
    let sql = USERS
        .insert_select()
        .columns(&["id", "email"])
        .from_select(&source_query)
        .returning_all()
        .to_sql(Some(&pg));
    println!("  [PG] Insert select: {}", sql);

    // 3f. UPDATE with set() (single column)
    let sql = USERS
        .update()
        .set("display_name")
        .set("updated_at")
        .where_eq("id")
        .to_sql(Some(&pg));
    println!("  [PG] Update: {}", sql);

    // 3g. UPDATE with set_columns() (multiple columns)
    let sql = USERS
        .update()
        .set_columns(&["display_name", "status", "updated_at"])
        .where_eq("id")
        .to_sql(Some(&pg));
    println!("  [PG] Update set_columns: {}", sql);

    // 3h. UPDATE with literal SET
    let sql = USERS
        .update()
        .set("status")
        .set_literal("updated_at", "NOW()")
        .where_eq("id")
        .returning_all()
        .to_sql(Some(&pg));
    println!("  [PG] Update literal + returning: {}", sql);

    // 3i. UPDATE with version increment (optimistic locking)
    let sql = TENANTS
        .update()
        .set("display_name")
        .set_increment("version")
        .where_eq("id")
        .where_eq("version")
        .returning_all()
        .to_sql(Some(&pg));
    println!("  [PG] Optimistic lock: {}", sql);
    assert!(sql.contains("version = version + $"));

    // 3j. UPDATE with arbitrary expression
    let sql = USERS
        .update()
        .set_expr("login_count", col("login_count") + lit(1))
        .where_eq("id")
        .to_sql(Some(&pg));
    println!("  [PG] Update set_expr: {}", sql);

    // 3k. UPDATE with raw WHERE
    let sql = USERS
        .update()
        .set("status")
        .where_raw("created_at < NOW() - INTERVAL '30 days'")
        .to_sql(Some(&pg));
    println!("  [PG] Update raw WHERE: {}", sql);

    // 3l. UPDATE with expression filter
    let sql = USERS
        .update()
        .set("status")
        .filter(field("login_count").gt(lit(0)))
        .to_sql(Some(&pg));
    println!("  [PG] Update filter: {}", sql);

    // 3m. DELETE with WHERE
    let sql = USERS
        .remove()
        .where_eq("id")
        .returning_all()
        .to_sql(Some(&pg));
    println!("  [PG] Delete: {}", sql);

    // 3n. DELETE with expression filter
    let sql = USERS
        .remove()
        .filter(field("status").eq(lit("deleted")))
        .to_sql(Some(&pg));
    println!("  [PG] Delete filter: {}", sql);

    // 3o. DELETE with raw WHERE
    let sql = USERS
        .remove()
        .where_raw("created_at < NOW() - INTERVAL '90 days'")
        .to_sql(Some(&pg));
    println!("  [PG] Delete raw: {}", sql);

    // 3p. DELETE param count
    let builder = USERS.remove().where_eq("id").where_eq("tenant_id");
    assert_eq!(builder.param_count(), 2);
    println!("  Delete param count: {}", builder.param_count());

    // 3q. UPSERT (ON CONFLICT DO UPDATE)
    let sql = USERS
        .upsert()
        .all_columns()
        .on_conflict(&["email"])
        .do_update(&["display_name", "status", "updated_at"])
        .returning_all()
        .to_sql(Some(&pg));
    println!("  [PG] Upsert: {}", sql);
    assert!(sql.contains("ON CONFLICT"));
    assert!(sql.contains("DO UPDATE SET"));

    // 3r. UPSERT DO NOTHING
    let sql = USERS
        .upsert()
        .all_columns()
        .on_conflict(&["email"])
        .do_nothing()
        .to_sql(Some(&pg));
    println!("  [PG] Upsert do nothing: {}", sql);
    assert!(sql.contains("DO NOTHING"));

    // 3s. UPSERT with named constraint
    let sql = USERS
        .upsert()
        .all_columns()
        .on_conflict_constraint("users_email_key")
        .do_update(&["display_name"])
        .to_sql(Some(&pg));
    println!("  [PG] Upsert on constraint: {}", sql);

    // 3t. UPSERT with conflict filter
    let sql = USERS
        .upsert()
        .all_columns()
        .on_conflict(&["email"])
        .conflict_filter(field("status").ne(lit("deleted")))
        .do_update(&["display_name"])
        .to_sql(Some(&pg));
    println!("  [PG] Upsert conflict filter: {}", sql);

    // ========================================================================
    // 4. DDL (CREATE TABLE, ALTER TABLE, DROP TABLE, INDEXES)
    // ========================================================================
    println!("\n--- 4. DDL ---");

    // 4a. CREATE TABLE from static model metadata
    let sql = USERS.create().to_sql(Some(&pg));
    println!("  [PG] Create table: {}...", &sql[..sql.len().min(100)]);
    assert!(sql.contains("CREATE TABLE users"));

    // 4b. CREATE TABLE IF NOT EXISTS
    let sql = USERS.create().if_not_exists().to_sql(Some(&pg));
    assert!(sql.contains("IF NOT EXISTS"));
    println!(
        "  [PG] Create if not exists: {}...",
        &sql[..sql.len().min(100)]
    );

    // 4c. CREATE TABLE with constraints (FK, CHECK, composite)
    let sql = AUDIT_LOG.create().to_sql(Some(&pg));
    assert!(sql.contains("REFERENCES"));
    println!("  [PG] Create with FK: {}...", &sql[..sql.len().min(100)]);

    // 4d. CREATE TABLE with namespace
    let sql = AUDIT_LOG.create().to_sql(Some(&pg));
    assert!(sql.contains("audit.audit_log"));
    println!("  [PG] Namespace: {}...", &sql[..sql.len().min(100)]);

    // 4e. CREATE TABLE with composite PK
    let sql = ORDER_ITEMS.create().to_sql(Some(&pg));
    assert!(sql.contains("PRIMARY KEY (order_id, product_id)"));
    println!("  [PG] Composite PK: {}...", &sql[..sql.len().min(100)]);

    // 4f. DefineModelBuilder (runtime-defined schema)
    let sql = DefineModelBuilder::new("dynamic_table")
        .field(dol::FieldDef::new("id", FieldType::Uuid).primary_key())
        .field(dol::FieldDef::new("name", FieldType::Text))
        .field(dol::FieldDef::new("value", FieldType::Decimal).nullable())
        .if_not_exists()
        .to_sql(Some(&pg));
    println!("  [PG] Dynamic define: {}", sql);

    // 4g. ALTER TABLE — add column (takes a Field)
    let sql = USERS
        .alter()
        .add_column(Field::new("phone", FieldType::Text).nullable())
        .to_sql(Some(&pg));
    println!("  [PG] Alter add: {}", sql);
    assert!(sql.contains("ADD COLUMN"));

    // 4h. ALTER TABLE — drop column
    let sql = USERS.alter().drop_field("profile").to_sql(Some(&pg));
    println!("  [PG] Alter drop: {}", sql);

    // 4i. ALTER TABLE — rename column
    let sql = USERS
        .alter()
        .rename_field("display_name", "full_name")
        .to_sql(Some(&pg));
    println!("  [PG] Alter rename col: {}", sql);

    // 4j. ALTER TABLE — change column type
    let sql = USERS
        .alter()
        .alter_column_type("login_count", FieldType::BigInt)
        .to_sql(Some(&pg));
    println!("  [PG] Alter column type: {}", sql);

    // 4k. ALTER TABLE — set/drop default
    let sql = USERS
        .alter()
        .set_default("status", "'inactive'")
        .to_sql(Some(&pg));
    println!("  [PG] Set default: {}", sql);

    let sql = USERS.alter().drop_default("status").to_sql(Some(&pg));
    println!("  [PG] Drop default: {}", sql);

    // 4l. ALTER TABLE — set/drop NOT NULL
    let sql = USERS.alter().set_not_null("profile").to_sql(Some(&pg));
    println!("  [PG] Set NOT NULL: {}", sql);

    let sql = USERS.alter().drop_not_null("email").to_sql(Some(&pg));
    println!("  [PG] Drop NOT NULL: {}", sql);

    // 4m. ALTER TABLE — rename table
    let sql = USERS.alter().rename_model("app_users").to_sql(Some(&pg));
    println!("  [PG] Rename table: {}", sql);

    // 4n. ALTER TABLE — add/drop constraint
    let sql = USERS
        .alter()
        .add_constraint(ModelConstraint::Unique(&["email", "tenant_id"]))
        .to_sql(Some(&pg));
    println!("  [PG] Add constraint: {}", sql);

    let sql = USERS
        .alter()
        .drop_constraint("users_email_key")
        .to_sql(Some(&pg));
    println!("  [PG] Drop constraint: {}", sql);

    // 4o. ALTER TABLE — multiple actions
    let sql = USERS
        .alter()
        .add_column(Field::new("phone", FieldType::Text).nullable())
        .drop_field("profile")
        .set_not_null("display_name")
        .to_sql(Some(&pg));
    println!("  [PG] Multi alter: {}", sql);

    // 4p. DROP TABLE
    let sql = USERS.drop_model().to_sql(Some(&pg));
    println!("  [PG] Drop table: {}", sql);

    // 4q. DROP TABLE IF EXISTS CASCADE
    let sql = USERS.drop_model().if_exists().cascade().to_sql(Some(&pg));
    assert!(sql.contains("IF EXISTS"));
    assert!(sql.contains("CASCADE"));
    println!("  [PG] Drop if exists cascade: {}", sql);

    // 4r. CREATE INDEX
    let sql = DefineIndexBuilder::new("idx_users_email")
        .on("users")
        .columns(&["email"])
        .to_sql(Some(&pg));
    println!("  [PG] Create index: {}", sql);

    // 4s. CREATE UNIQUE INDEX IF NOT EXISTS
    let sql = DefineIndexBuilder::new("idx_users_tenant_email")
        .on("users")
        .columns(&["tenant_id", "email"])
        .unique()
        .if_not_exists()
        .to_sql(Some(&pg));
    println!("  [PG] Unique index: {}", sql);

    // 4t. CREATE INDEX with method and WHERE (partial index)
    let sql = DefineIndexBuilder::new("idx_users_active_email")
        .on("users")
        .columns(&["email"])
        .method(dol::ir::definition::IndexMethod::Hash)
        .where_clause("status = 'active'")
        .to_sql(Some(&pg));
    println!("  [PG] Partial index: {}", sql);

    // 4u. CREATE INDEX CONCURRENTLY
    let sql = DefineIndexBuilder::new("idx_users_email_conc")
        .on("users")
        .columns(&["email"])
        .concurrently()
        .to_sql(Some(&pg));
    assert!(sql.contains("CONCURRENTLY"));
    println!("  [PG] Concurrent index: {}", sql);

    // 4v. DROP INDEX
    let sql = DropIndexBuilder::new("idx_users_email")
        .if_exists()
        .to_sql(Some(&pg));
    println!("  [PG] Drop index: {}", sql);

    // ========================================================================
    // 5. EXPRESSIONS — the composable expression engine
    // ========================================================================
    println!("\n--- 5. Expressions ---");

    // 5a. Comparison operators
    let _eq = field("age").eq(lit(18));
    let _ne = field("age").ne(lit(18));
    let _lt = field("age").lt(lit(18));
    let _gt = field("age").gt(lit(18));
    let _le = field("age").le(lit(18));
    let _ge = field("age").ge(lit(18));
    println!("  Comparison ops: eq, ne, lt, gt, le, ge ✓");

    // 5b. Boolean composition with & (AND) and | (OR)
    let _and = field("age").gt(lit(18)) & field("status").eq(lit("active"));
    let _or = field("role").eq(lit("admin")) | field("role").eq(lit("superadmin"));
    println!("  Boolean ops: AND (&), OR (|) ✓");

    // 5c. NOT
    let _not = !field("is_deleted").eq(lit(true));
    println!("  NOT: ! operator ✓");

    // 5d. Arithmetic operators (+, -, *, /, %)
    let _add = field("price") + field("tax");
    let _sub = field("total") - field("discount");
    let _mul = field("price") * field("quantity");
    let _div = field("total") / lit(2);
    let _rem = field("value") % lit(10);
    println!("  Arithmetic ops: +, -, *, /, %% ✓");

    // 5e. Negation
    let _neg = -field("balance");
    println!("  Negation: -expr ✓");

    // 5f. CAST
    let _cast = field("count").cast("BIGINT");
    println!("  Cast: CAST(count AS BIGINT) ✓");

    // 5g. Alias
    let _alias = func::count_star().alias("total_count");
    println!("  Alias: COUNT(*) AS total_count ✓");

    // 5h. String concatenation (dialect-aware)
    let _concat = field("first_name")
        .concat(lit(" "))
        .concat(field("last_name"));
    println!("  Concat: first_name || ' ' || last_name ✓");

    // 5i. CASE WHEN expression
    let _case = case()
        .when(field("status").eq(lit("active")), lit("Active"))
        .when(field("status").eq(lit("pending")), lit("Pending"))
        .else_(lit("Unknown"))
        .end();
    println!("  CASE WHEN: ✓");

    // 5j. Field access for nested/JSON data
    let _access = field("profile").access("address").access("city");
    println!("  Field access: profile.address.city ✓");

    // 5k. Qualified identifiers
    let _qualified = dol::expr::qualified("u", "email");
    let _qualified_col = dol::expr::qualified_col("users", "email");
    println!("  Qualified: u.email, users.email ✓");

    // 5l. From &str to Expr
    let _from_str: dol::expr::Expr = "email".into();
    println!("  From &str: \"email\".into() ✓");

    // 5m. LIKE / ILIKE
    let _like = field("email").like(lit("%@example.com"));
    let _ilike = field("email").ilike(lit("%@example.com"));
    println!("  LIKE / ILIKE ✓");

    // 5n. IS NULL / IS NOT NULL
    let _is_null = field("profile").is_null();
    let _is_not_null = field("profile").is_not_null();
    println!("  IS NULL / IS NOT NULL ✓");

    // 5o. BETWEEN / NOT BETWEEN
    let _between = field("age").between(lit(18), lit(65));
    let _not_between = field("age").not_between(lit(0), lit(17));
    println!("  BETWEEN / NOT BETWEEN ✓");

    // 5p. IN list / NOT IN list
    let _in = field("status").in_list(vec![lit("a"), lit("b")]);
    let _not_in = field("status").not_in_list(vec![lit("x"), lit("y")]);
    println!("  IN / NOT IN list ✓");

    // 5q. IN / NOT IN subquery
    let _in_sub = field("id").in_subquery("SELECT user_id FROM active_users");
    let _not_in_sub = field("id").not_in_subquery("SELECT user_id FROM banned_users");
    println!("  IN / NOT IN subquery ✓");

    // 5r. All function constructors
    let _ = func::lower(field("email"));
    let _ = func::upper(field("name"));
    let _ = func::trim(field("value"));
    let _ = func::length(field("name"));
    let _ = func::substr(field("name"), lit(1), lit(10));
    let _ = func::replace(field("email"), lit("old"), lit("new"));
    let _ = func::concat_fn(vec![field("a"), lit(" "), field("b")]);
    let _ = func::abs(field("diff"));
    let _ = func::round(field("price"), Some(lit(2)));
    let _ = func::now();
    let _ = func::current_date();
    let _ = func::current_timestamp();
    let _ = func::coalesce(vec![field("name"), field("email")]);
    let _ = func::nullif(field("value"), lit(0));
    let _ = func::count(field("id"));
    let _ = func::count_star();
    let _ = func::sum(field("amount"));
    let _ = func::avg(field("score"));
    let _ = func::min(field("created_at"));
    let _ = func::max(field("created_at"));
    let _ = func::func("CUSTOM_FN", vec![field("x"), lit(42)]);
    println!("  All function constructors (22 functions) ✓");

    // 5s. All window function constructors
    let _ = func::row_number();
    let _ = func::rank();
    let _ = func::dense_rank();
    let _ = func::ntile(lit(4));
    let _ = func::lag(field("value"), None, None);
    let _ = func::lag(field("value"), Some(lit(1)), Some(lit(0)));
    let _ = func::lead(field("value"), None, None);
    let _ = func::lead(field("value"), Some(lit(1)), Some(lit(0)));
    let _ = func::first_value(field("price"));
    let _ = func::last_value(field("price"));
    println!("  All window functions (10 functions) ✓");

    // 5t. Object and array literals
    let _obj = dol::expr::obj(vec![("name", lit("John")), ("age", lit(30))]);
    let _arr = dol::expr::arr(vec![lit(1), lit(2), lit(3)]);
    println!("  Object/Array literals ✓");

    // 5u. Ordering expressions (all 6 variants)
    let _ = field("name").asc();
    let _ = field("name").desc();
    let _ = field("name").asc_nulls_first();
    let _ = field("name").asc_nulls_last();
    let _ = field("name").desc_nulls_first();
    let _ = field("name").desc_nulls_last();
    println!("  All 6 ordering variants ✓");

    // 5v. Window builder with frame
    let _ = func::row_number()
        .over()
        .partition_by(vec![col("tenant_id")])
        .order_by(vec![field("created_at").desc()])
        .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    let _ = func::sum(col("amount"))
        .over()
        .partition_by(vec![col("category")])
        .order_by(vec![field("date").asc()])
        .range_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    println!("  Window builder: rows_between, range_between ✓");

    // 5w. Literal types
    let _ = lit("text"); // Literal::Str
    let _ = lit(42_i64); // Literal::Int
    let _ = lit(9.99_f64); // Literal::Float
    let _ = lit(true); // Literal::Bool
    let _ = dol::expr::Expr::Literal(dol::expr::Literal::Null); // Literal::Null
    println!("  Literal types: Str, Int, Float, Bool, Null ✓");

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

    let q1 = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("status")
        .to_sql(Some(&pg));
    let q2 = USERS
        .get()
        .columns(&["id", "email"])
        .where_eq("tenant_id")
        .to_sql(Some(&pg));

    // 6a. UNION
    let sql = CompoundSelectBuilder::new(q1.clone())
        .union(q2.clone())
        .to_sql(Some(&pg));
    assert!(sql.contains("UNION"));
    println!("  [PG] UNION: {}", sql);

    // 6b. UNION ALL
    let sql = CompoundSelectBuilder::new(q1.clone())
        .union_all(q2.clone())
        .to_sql(Some(&pg));
    assert!(sql.contains("UNION ALL"));
    println!("  [PG] UNION ALL: {}", sql);

    // 6c. INTERSECT
    let sql = CompoundSelectBuilder::new(q1.clone())
        .intersect(q2.clone())
        .to_sql(Some(&pg));
    assert!(sql.contains("INTERSECT"));
    println!("  [PG] INTERSECT: {}", sql);

    // 6d. EXCEPT
    let sql = CompoundSelectBuilder::new(q1.clone())
        .except(q2.clone())
        .to_sql(Some(&pg));
    assert!(sql.contains("EXCEPT"));
    println!("  [PG] EXCEPT: {}", sql);

    // 6e. Compound with ORDER BY + LIMIT
    let sql = CompoundSelectBuilder::new(q1.clone())
        .union(q2.clone())
        .order_by(vec![field("email").asc()])
        .limit()
        .offset()
        .to_sql(Some(&pg));
    println!("  [PG] Compound + order/limit: {}", sql);

    // ========================================================================
    // 7. TRANSACTIONS
    // ========================================================================
    println!("\n--- 7. Transactions ---");

    let sql = TransactionBuilder::to_sql(&TransactionBuilder::begin());
    assert_eq!(sql, "BEGIN");
    println!("  BEGIN: {}", sql);

    let sql = TransactionBuilder::to_sql(&TransactionBuilder::commit());
    assert_eq!(sql, "COMMIT");
    println!("  COMMIT: {}", sql);

    let sql = TransactionBuilder::to_sql(&TransactionBuilder::rollback());
    assert_eq!(sql, "ROLLBACK");
    println!("  ROLLBACK: {}", sql);

    let sql = TransactionBuilder::to_sql(&TransactionBuilder::savepoint("sp1"));
    assert_eq!(sql, "SAVEPOINT sp1");
    println!("  SAVEPOINT: {}", sql);

    let sql = TransactionBuilder::to_sql(&TransactionBuilder::release_savepoint("sp1"));
    assert_eq!(sql, "RELEASE SAVEPOINT sp1");
    println!("  RELEASE: {}", sql);

    let sql = TransactionBuilder::to_sql(&TransactionBuilder::rollback_to_savepoint("sp1"));
    assert_eq!(sql, "ROLLBACK TO SAVEPOINT sp1");
    println!("  ROLLBACK TO: {}", sql);

    // ========================================================================
    // 8. ACCESS CONTROL (GRANT, REVOKE)
    // ========================================================================
    println!("\n--- 8. Access Control ---");

    let sql = GrantBuilder::new(Privilege::Select)
        .on("users")
        .to("app_reader")
        .to_sql(None);
    assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::Insert)
        .on("users")
        .to("app_writer")
        .to_sql(None);
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::Update)
        .on("users")
        .to("app_writer")
        .to_sql(None);
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::Delete)
        .on("users")
        .to("app_admin")
        .to_sql(None);
    println!("  {}", sql);

    let sql = GrantBuilder::new(Privilege::All)
        .on("users")
        .to("superadmin")
        .to_sql(None);
    println!("  {}", sql);

    let sql = RevokeBuilder::new(Privilege::Insert)
        .on("users")
        .from("readonly_role")
        .to_sql(None);
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
        let sql = USERS
            .get()
            .columns(&["id", "email"])
            .where_eq("id")
            .to_sql(Some(d));
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
        let sql = USERS
            .insert()
            .columns(&["id", "email"])
            .returning_all()
            .to_sql(Some(d));
        println!("    [{}] {}", name, sql);
    }

    // 9c. UPSERT across dialects (ON CONFLICT vs ON DUPLICATE KEY vs MERGE)
    println!("  UPSERT:");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("MSSQL      ", &mssql),
    ] {
        let sql = USERS
            .upsert()
            .columns(&["id", "email", "display_name"])
            .on_conflict(&["email"])
            .do_update(&["display_name"])
            .to_sql(Some(d));
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
        let sql = USERS
            .get()
            .all_columns()
            .order_by_asc("email")
            .limit()
            .offset()
            .to_sql(Some(d));
        println!("    [{}] {}", name, sql);
    }

    // 9e. CREATE TABLE type mapping across dialects
    println!("  CREATE TABLE (type mapping):");
    for (name, d) in [
        ("PostgreSQL ", &pg),
        ("MySQL      ", &mysql),
        ("SQLite     ", &sqlite),
    ] {
        let sql = PRODUCTS.create().to_sql(Some(d));
        println!("    [{}] {}...", name, &sql[..sql.len().min(120)]);
    }

    // 9f. Boolean literals (TRUE/FALSE vs 1/0)
    println!("  Boolean literals:");
    for (name, d) in [
        ("PostgreSQL", &pg),
        ("MySQL     ", &mysql),
        ("SQLite    ", &sqlite),
    ] {
        let sql = PRODUCTS
            .get()
            .all_columns()
            .filter(field("is_active").eq(lit(true)))
            .to_sql(Some(d));
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
    println!("  Resolve type UUID: {}", pg.resolve_type(&FieldType::Uuid));
    println!(
        "  Resolve type Timestamp: {}",
        pg.resolve_type(&FieldType::Timestamp)
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
    use dol::ir::{AlterAction, ModelRef};
    use dol::migration::{
        InMemoryRegistry, Migration, MigrationDirection, MigrationRegistry, MigrationRunner,
        MigrationState, MigrationStep, MigrationTarget, RenderedStep,
    };
    use dol::model::FieldType;

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
            vec![MigrationStep::define_model(
                dol::builder::DefineModelBuilder::new("users")
                    .field(FieldDef::new("id", FieldType::Uuid).primary_key())
                    .field(FieldDef::new("email", FieldType::Text).unique())
                    .field(FieldDef::new("status", FieldType::Text).default("'active'"))
                    .field(FieldDef::new("created_at", FieldType::Timestamp).default("NOW()"))
                    .if_not_exists()
                    .build(),
            )]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::drop_model("users")]
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
                target: ModelRef {
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
            vec![MigrationStep::alter_model(
                "users",
                vec![AlterAction::AddField(
                    FieldDef::new("profile", FieldType::Json).nullable(),
                )],
            )]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::alter_model(
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
        ModelSnapshot, create_model_step, diff_models, diff_to_steps, drop_model_step,
        field_to_field_def,
    };

    println!("\n--- 13. Schema Diff & Auto-Discovery ---");

    // 13a. Define two versions of a model
    static USERS_V1: Model = Model::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("email", FieldType::Text).unique(),
            Field::new("status", FieldType::Text).default("'active'"),
        ],
    );

    static USERS_V2: Model = Model::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("email", FieldType::Text).unique(),
            Field::new("status", FieldType::Text).default("'active'"),
            // New field added in V2
            Field::new("display_name", FieldType::Text).nullable(),
        ],
    );

    // 13b. Take snapshots and compute diff
    let v1 = ModelSnapshot::from_model(&USERS_V1);
    let v2 = ModelSnapshot::from_model(&USERS_V2);
    let actions = diff_models(&v1, &v2);
    println!("  Diff V1→V2: {} change(s)", actions.len());
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::AddField(f) if f.name == "display_name"));

    // 13c. Generate migration steps from diff
    let steps = diff_to_steps("users", &v1, &v2);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind(), "sql");
    println!("  Generated ALTER TABLE step for field addition");

    // 13d. Reverse diff (V2 → V1) produces DropField
    let reverse_actions = diff_models(&v2, &v1);
    assert_eq!(reverse_actions.len(), 1);
    assert!(matches!(&reverse_actions[0], AlterAction::DropField(name) if name == "display_name"));
    println!("  Reverse diff produces DropField");

    // 13e. Type change detection
    static V_INT: Model = Model::new("t", &[Field::new("count", FieldType::Int)]);
    static V_BIGINT: Model = Model::new("t", &[Field::new("count", FieldType::BigInt)]);
    let type_actions = diff_models(
        &ModelSnapshot::from_model(&V_INT),
        &ModelSnapshot::from_model(&V_BIGINT),
    );
    assert_eq!(type_actions.len(), 1);
    assert!(matches!(
        &type_actions[0],
        AlterAction::AlterFieldType { name, new_type }
            if name == "count" && *new_type == FieldType::BigInt
    ));
    println!("  Type change detected: Int → BigInt");

    // 13f. Create and drop model steps from static definitions
    let create_step = create_model_step(&USERS_V1);
    assert_eq!(create_step.kind(), "sql");
    let drop_step = drop_model_step(&USERS_V1);
    assert_eq!(drop_step.kind(), "sql");
    println!("  Create/drop model steps generated from static Model");

    // 13g. Field conversion preserves all attributes
    let field = Field::new("email", FieldType::Text)
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
            vec![MigrationStep::define_model(
                dol::builder::DefineModelBuilder::new("orders")
                    .field(FieldDef::new("id", FieldType::Uuid).primary_key())
                    .field(FieldDef::new("total", FieldType::Decimal))
                    .if_not_exists()
                    .build(),
            )]
        }
        fn down(&self) -> Vec<MigrationStep> {
            vec![MigrationStep::drop_model("orders")]
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
        // Verify all field constraint types are exercised
        assert!(USERS.field("id").primary_key);
        assert!(USERS.field("email").unique);
        assert!(!USERS.field("email").nullable);
        assert!(USERS.field("profile").nullable);
        assert_eq!(USERS.field("status").default_expr, Some("'active'"));
        assert!(USERS.field("tenant_id").indexed);
        assert!(AUDIT_LOG.field("action").check.is_some());
        assert!(AUDIT_LOG.field("user_id").references.is_some());
        assert!(SESSIONS.field("is_expired").generated.is_some());
        assert!(SESSIONS.field("user_agent").comment.is_some());
        assert_eq!(AUDIT_LOG.namespace, Some("audit"));
    }

    #[test]
    fn all_field_types_represented() {
        // Verify that the models exercise a wide range of FieldType variants
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| matches!(f.field_type, FieldType::Varchar(Some(_))))
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| matches!(f.field_type, FieldType::Char(_)))
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Decimal)
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Float)
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Bool)
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::TextArray)
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Object)
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Path)
        );
        assert!(
            PRODUCTS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Mime)
        );
        assert!(
            AUDIT_LOG
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::BigSerial)
        );
        assert!(
            AUDIT_LOG
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Inet)
        );
        assert!(
            SESSIONS
                .fields
                .iter()
                .any(|f| f.field_type == FieldType::Bytes)
        );
    }

    #[test]
    fn all_dialects_produce_valid_sql() {
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
            let sql = USERS.get().all_columns().where_eq("id").to_sql(Some(d));
            assert!(sql.contains("SELECT"), "{}: missing SELECT", d.name);
            assert!(sql.contains("FROM users"), "{}: missing FROM", d.name);

            // INSERT
            let sql = USERS.insert().all_columns().to_sql(Some(d));
            assert!(sql.contains("INSERT INTO"), "{}: missing INSERT", d.name);

            // UPDATE
            let sql = USERS.update().set("email").where_eq("id").to_sql(Some(d));
            assert!(sql.contains("UPDATE users"), "{}: missing UPDATE", d.name);

            // DELETE
            let sql = USERS.remove().where_eq("id").to_sql(Some(d));
            assert!(sql.contains("DELETE FROM"), "{}: missing DELETE", d.name);

            // CREATE TABLE
            let sql = USERS.create().to_sql(Some(d));
            assert!(sql.contains("CREATE TABLE"), "{}: missing CREATE", d.name);

            // DROP TABLE
            let sql = USERS.drop_model().to_sql(Some(d));
            assert!(sql.contains("DROP TABLE"), "{}: missing DROP", d.name);
        }
    }
}

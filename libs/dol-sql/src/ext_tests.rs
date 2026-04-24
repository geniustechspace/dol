use super::*;
use dol_query::builder::EntityBuilderExt;
use dol_core::expr::{field, param};
use dol_core::op::definition::FieldDef;
use dol_entity::{Entity, Field, DataType};

fn pg() -> Dialect {
    Dialect::postgres()
}

fn test_model() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("tenant_id", DataType::Uuid),
            Field::new("email", DataType::Text).unique(),
            Field::new("status", DataType::Text).default("'active'"),
            Field::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"),
        ],
    )
}

fn ns_model() -> Entity {
    Entity::new("users", vec![Field::new("id", DataType::Uuid).primary_key()]).with_namespace("auth")
}

// -- GetBuilder --

#[test]
fn get_builder_render() {
    let model = test_model();
    let sql = model
        .get()
        .filter(field("status").eq(param()))
        .order_by_desc("email")
        .limit()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("FROM users"));
}

// -- InsertBuilder --

#[test]
fn insert_defaults() {
    let model = test_model();
    let sql = model.insert().render(Some(&pg())).unwrap();
    assert!(sql.contains("INSERT INTO users (id, tenant_id, email, status, created_at)"));
    assert!(sql.contains("VALUES ($1, $2, $3, $4, $5)"));
}

#[test]
fn insert_specific_columns() {
    let model = test_model();
    let sql = model
        .insert()
        .fields(&["id", "email"])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("(id, email)"));
    assert!(sql.contains("($1, $2)"));
}

#[test]
fn insert_multiple_rows() {
    let model = test_model();
    let sql = model
        .insert()
        .fields(&["id", "email"])
        .rows(3)
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("($1, $2), ($3, $4), ($5, $6)"));
}

#[test]
fn insert_returning_all() {
    let model = test_model();
    let sql = model
        .insert()
        .fields(&["id"])
        .output_all()
        .render(None)
        .unwrap();
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn insert_with_namespace() {
    let model = ns_model();
    let sql = model.insert().fields(&["id"]).render(None).unwrap();
    assert!(sql.contains("INSERT INTO auth.users"));
}

// -- UpdateBuilder --

#[test]
fn update_set_and_where() {
    let model = test_model();
    let sql = model
        .update()
        .set("email")
        .set("status")
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("UPDATE users SET"));
    assert!(sql.contains("email = $1"));
    assert!(sql.contains("status = $2"));
    assert!(sql.contains("WHERE id = $3"));
}

// -- RemoveBuilder --

#[test]
fn remove_basic() {
    let model = test_model();
    let sql = model
        .remove()
        .filter(field("id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("DELETE FROM users"));
    assert!(sql.contains("WHERE id = $1"));
}

// -- UpsertBuilder --

#[test]
fn upsert_basic() {
    let model = test_model();
    let sql = model
        .upsert()
        .fields(&["id", "email", "status"])
        .match_on(&["id"])
        .patch(&["email", "status"])
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("INSERT INTO users (id, email, status)"));
    assert!(sql.contains("ON CONFLICT (id)"));
    assert!(sql.contains("DO UPDATE SET"));
}

// -- CreateFromMeta --

#[test]
fn create_from_meta_basic() {
    let model = test_model();
    let sql = model.create().render(Some(&pg())).unwrap();
    assert!(sql.starts_with("CREATE TABLE users ("));
    assert!(sql.contains("id UUID NOT NULL"));
    assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
    assert!(sql.contains("status TEXT NOT NULL DEFAULT 'active'"));
    assert!(sql.contains("PRIMARY KEY (id)"));
}

#[test]
fn create_from_meta_if_not_exists() {
    let model = test_model();
    let sql = model.create().if_not_exists().render(None).unwrap();
    assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS users ("));
}

// -- DefineEntityBuilder --

#[test]
fn define_model_builder_basic() {
    use dol_entity::EntityDefineExt;
    let sql = Entity::define("sessions")
        .field(FieldDef::new("id", DataType::Uuid).primary_key())
        .field(FieldDef::new("user_id", DataType::Uuid))
        .if_not_exists()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS sessions ("));
    assert!(sql.contains("id UUID NOT NULL"));
}

// -- AlterEntityBuilder --

#[test]
fn alter_model_add_and_drop() {
    let model = test_model();
    let sql = model
        .alter()
        .add_field(FieldDef::new("phone", DataType::Text).nullable())
        .drop_field("legacy")
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("ADD COLUMN phone TEXT"));
    assert!(sql.contains("DROP COLUMN legacy"));
}

// -- DropEntityBuilder --

#[test]
fn drop_model_basic() {
    let model = test_model();
    let sql = model.drop_entity().render(None).unwrap();
    assert_eq!(sql, "DROP TABLE users");
}

#[test]
fn drop_model_if_exists_cascade() {
    let model = test_model();
    let sql = model
        .drop_entity()
        .if_exists()
        .cascade()
        .render(None)
        .unwrap();
    assert_eq!(sql, "DROP TABLE IF EXISTS users CASCADE");
}

// -- DefineIndexBuilder --

#[test]
fn define_index_basic() {
    use dol_entity::DefineIndexBuilder;
    let sql = DefineIndexBuilder::new("idx_users_email")
        .on("users")
        .columns(&["tenant_id", "email"])
        .unique()
        .render(None)
        .unwrap();
    assert!(sql.contains("CREATE UNIQUE INDEX"));
    assert!(sql.contains("ON users"));
}

// -- DropIndexBuilder --

#[test]
fn drop_index_basic() {
    use dol_entity::DropIndexBuilder;
    let sql = DropIndexBuilder::new("idx_users_email")
        .render(None)
        .unwrap();
    assert_eq!(sql, "DROP INDEX idx_users_email");
}

// -- GrantBuilder --

#[test]
fn grant_basic() {
    use dol_query::builder::control::{GrantBuilder, Privilege};
    let sql = GrantBuilder::new(Privilege::Select)
        .on("users")
        .to("app_reader")
        .render(None)
        .unwrap();
    assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
}

// -- RevokeBuilder --

#[test]
fn revoke_basic() {
    use dol_query::builder::control::{Privilege, RevokeBuilder};
    let sql = RevokeBuilder::new(Privilege::Insert)
        .on("users")
        .from("app_reader")
        .render(None)
        .unwrap();
    assert_eq!(sql, "REVOKE INSERT ON users FROM app_reader");
}

// -- CompoundSelectBuilder param numbering --

#[test]
fn compound_select_params_globally_unique() {
    let model = test_model();
    let base_ir = model
        .get()
        .fields(&["id"])
        .filter(field("tenant_id").eq(param()))
        .build();
    let part_ir = model
        .get()
        .fields(&["id"])
        .filter(field("status").eq(param()))
        .build();
    let sql = CompoundSelectBuilder::new(base_ir)
        .union(part_ir)
        .limit()
        .offset()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("tenant_id = $1"), "base param: {sql}");
    assert!(sql.contains("status = $2"), "part param: {sql}");
    assert!(
        sql.contains("OFFSET $3"),
        "expected OFFSET $3 in output: {sql}"
    );
    assert!(
        sql.contains("LIMIT $4"),
        "expected LIMIT $4 in output: {sql}"
    );
}

#[test]
fn compound_select_no_params_starts_at_one() {
    let model = test_model();
    let base_ir = model.get().fields(&["id"]).build();
    let part_ir = model.get().fields(&["id"]).build();
    let sql = CompoundSelectBuilder::new(base_ir)
        .union(part_ir)
        .limit()
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("LIMIT $1"), "expected LIMIT $1, got: {sql}");
}

// -- DefineTypeBuilder --

#[test]
fn define_type_postgres() {
    use dol_entity::DefineTypeBuilder;
    let sql = DefineTypeBuilder::new("order_status")
        .variant("pending")
        .variant("shipped")
        .variant("delivered")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(
        sql,
        "CREATE TYPE order_status AS ENUM ('pending', 'shipped', 'delivered')"
    );
}

#[test]
fn define_type_mysql_is_comment() {
    use dol_entity::DefineTypeBuilder;
    let sql = DefineTypeBuilder::new("order_status")
        .variant("pending")
        .variant("shipped")
        .render(Some(&Dialect::mysql()))
        .unwrap();
    assert!(
        sql.starts_with("--"),
        "MySQL type should be a comment: {sql}"
    );
    assert!(sql.contains("ENUM"), "Should mention ENUM: {sql}");
}

#[test]
fn define_type_sqlite_is_comment() {
    use dol_entity::DefineTypeBuilder;
    let sql = DefineTypeBuilder::new("order_status")
        .variant("pending")
        .render(Some(&Dialect::sqlite()))
        .unwrap();
    assert!(
        sql.starts_with("--"),
        "SQLite type should be a comment: {sql}"
    );
    assert!(sql.contains("CHECK"), "Should mention CHECK: {sql}");
}

#[test]
fn define_type_with_namespace() {
    use dol_entity::DefineTypeBuilder;
    let sql = DefineTypeBuilder::new("order_status")
        .namespace("public")
        .variant("pending")
        .variant("shipped")
        .render(Some(&pg()))
        .unwrap();
    assert!(
        sql.contains("public.order_status"),
        "Expected qualified name: {sql}"
    );
}

#[test]
fn define_type_variants_batch() {
    use dol_entity::DefineTypeBuilder;
    let sql = DefineTypeBuilder::new("color")
        .variants(&["red", "green", "blue"])
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "CREATE TYPE color AS ENUM ('red', 'green', 'blue')");
}

// -- DropTypeBuilder --

#[test]
fn drop_type_postgres() {
    use dol_entity::DropTypeBuilder;
    let sql = DropTypeBuilder::new("order_status")
        .if_exists()
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "DROP TYPE IF EXISTS order_status");
}

#[test]
fn drop_type_without_if_exists() {
    use dol_entity::DropTypeBuilder;
    let sql = DropTypeBuilder::new("order_status")
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "DROP TYPE order_status");
}

#[test]
fn drop_type_mysql_is_comment() {
    use dol_entity::DropTypeBuilder;
    let sql = DropTypeBuilder::new("order_status")
        .render(Some(&Dialect::mysql()))
        .unwrap();
    assert!(
        sql.starts_with("--"),
        "MySQL drop type should be a comment: {sql}"
    );
}

// -- DefinePolicyBuilder --

#[test]
fn define_policy_postgres() {
    use dol_query::builder::DefinePolicyBuilder;
    use dol_core::expr::{field, param};
    use dol_ir::control::PolicyAction;

    let sql = DefinePolicyBuilder::new("tenant_isolation")
        .on("orders")
        .for_action(PolicyAction::All)
        .using(field("tenant_id").eq(param()))
        .check(field("tenant_id").eq(param()))
        .render(Some(&pg()))
        .unwrap();
    assert!(
        sql.contains("CREATE POLICY tenant_isolation ON orders FOR ALL"),
        "{sql}"
    );
    assert!(sql.contains("USING (tenant_id = $1)"), "{sql}");
    assert!(sql.contains("WITH CHECK (tenant_id = $2)"), "{sql}");
}

#[test]
fn define_policy_read_only() {
    use dol_query::builder::DefinePolicyBuilder;
    use dol_core::expr::{field, bool_expr};
    use dol_ir::control::PolicyAction;

    let sql = DefinePolicyBuilder::new("public_read")
        .on("posts")
        .for_action(PolicyAction::Read)
        .using(field("published").eq(bool_expr(true)))
        .render(Some(&pg()))
        .unwrap();
    assert!(sql.contains("FOR SELECT"), "{sql}");
    assert!(sql.contains("USING (published = TRUE)"), "{sql}");
    assert!(
        !sql.contains("WITH CHECK"),
        "Read-only policy should not have WITH CHECK: {sql}"
    );
}

#[test]
fn define_policy_no_expressions() {
    use dol_query::builder::DefinePolicyBuilder;
    use dol_ir::control::PolicyAction;

    let sql = DefinePolicyBuilder::new("allow_all")
        .on("logs")
        .for_action(PolicyAction::All)
        .render(Some(&pg()))
        .unwrap();
    assert_eq!(sql, "CREATE POLICY allow_all ON logs FOR ALL");
}


// -- render_with / BuildSession --

#[test]
fn render_with_valid_query() {
    use dol_core::session::BuildSession;

    let model = test_model();
    let session = BuildSession::new();
    let sql = model
        .get()
        .filter(field("status").eq(param()))
        .render_with(&session, Some(&pg()))
        .unwrap();
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("WHERE"));
}

#[test]
fn render_with_aggregate_in_where_rejected() {
    use dol_core::expr::func::count;
    use dol_core::session::{BuildError, BuildSession};
    use dol_core::expr::pass::PassError;

    let model = test_model();
    let session = BuildSession::new();
    // count() is an aggregate — illegal in WHERE context
    let err = model
        .get()
        .filter(count(field("id")).gt(param()))
        .render_with(&session, Some(&pg()))
        .unwrap_err();

    match err {
        BuildError::Validation(errs) => {
            assert!(
                errs.iter().any(|e| matches!(e, PassError::AggregateInWrongContext { .. })),
                "expected AggregateInWrongContext, got: {errs:?}",
            );
        }
        other => panic!("expected Validation error, got: {other}"),
    }
}

#[test]
fn render_with_disallowed_function_rejected() {
    use dol_core::expr::func::count;
    use dol_core::expr::pass::AllowList;
    use dol_core::session::{BuildError, BuildSession};
    use dol_core::expr::pass::PassError;

    let model = test_model();
    // Allowlist permits nothing — count() in SELECT should be rejected
    let session = BuildSession::new()
        .with_allowlist(AllowList::from_funcs([] as [&str; 0]));
    let err = model
        .get()
        .field(count(field("id")))
        .render_with(&session, Some(&pg()))
        .unwrap_err();

    match err {
        BuildError::Validation(errs) => {
            assert!(
                errs.iter().any(|e| matches!(e, PassError::DisallowedFunction { .. })),
                "expected DisallowedFunction, got: {errs:?}",
            );
        }
        other => panic!("expected Validation error, got: {other}"),
    }
}

#[test]
fn render_with_update_aggregate_in_where_rejected() {
    use dol_core::expr::func::sum;
    use dol_core::session::{BuildError, BuildSession};
    use dol_core::expr::pass::PassError;

    let model = test_model();
    let session = BuildSession::new();
    let err = model
        .update()
        .set("status")
        .filter(sum(field("id")).gt(param()))
        .render_with(&session, Some(&pg()))
        .unwrap_err();

    match err {
        BuildError::Validation(errs) => {
            assert!(errs.iter().any(|e| matches!(e, PassError::AggregateInWrongContext { .. })));
        }
        other => panic!("expected Validation, got: {other}"),
    }
}

#[test]
fn render_with_remove_aggregate_in_where_rejected() {
    use dol_core::expr::func::count;
    use dol_core::session::{BuildError, BuildSession};
    use dol_core::expr::pass::PassError;

    let model = test_model();
    let session = BuildSession::new();
    let err = model
        .remove()
        .filter(count(field("id")).gt(param()))
        .render_with(&session, Some(&pg()))
        .unwrap_err();

    match err {
        BuildError::Validation(errs) => {
            assert!(errs.iter().any(|e| matches!(e, PassError::AggregateInWrongContext { .. })));
        }
        other => panic!("expected Validation, got: {other}"),
    }
}


#[test]
fn transaction_block_postgres() {
    use dol_query::builder::transaction::TransactionBuilder;

    let model = test_model();
    let insert_ir = model.insert().fields(&["id", "email"]).build();
    let stmts = vec![dol_core::op::Statement::Insert(insert_ir)];
    let ir = TransactionBuilder::block(stmts);
    let sql = TransactionBuilder::render(&ir, Some(&pg())).unwrap();
    assert!(sql.starts_with("BEGIN"), "Should start with BEGIN: {sql}");
    assert!(sql.contains("INSERT INTO"), "Should contain INSERT: {sql}");
    assert!(sql.ends_with("COMMIT"), "Should end with COMMIT: {sql}");
}

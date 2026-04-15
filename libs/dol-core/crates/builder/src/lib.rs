//! # dol-builder — DOL Builder API
//!
//! Composable method-chain APIs that produce DOL IR.
//!
//! Every builder follows the same pattern:
//! 1. Create via `Entity::get()`, `Entity::insert()`, etc.
//! 2. Chain configuration methods
//! 3. Call `.build()` to produce IR
//!
//! For SQL rendering, import the extension traits from `dol-sql`.

#![deny(unsafe_code)]

pub mod control;
pub mod definition;
pub mod mutation;
pub mod query;
pub mod storage;
pub mod transaction;

pub use control::{DefinePolicyBuilder, GrantBuilder, RevokeBuilder};
pub use definition::{
    AlterEntityBuilder, DefineEntityBuilder, DefineIndexBuilder, DefineTypeBuilder,
    DropEntityBuilder, DropIndexBuilder, DropTypeBuilder, EntityDefineExt,
};
pub use mutation::{
    InsertBuilder, InsertSelectBuilder, RemoveBuilder, UpdateBuilder, UpsertBuilder,
};
pub use query::GetBuilder;
pub use storage::{
    GetObjectBuilder, ListObjectsBuilder, MoveFileBuilder, PutObjectBuilder, ReadFileBuilder,
    WriteFileBuilder,
};
pub use transaction::TransactionBuilder;

use dol_entity::Entity;

// ---------------------------------------------------------------------------
// Model entry points — extension trait for builder access
// ---------------------------------------------------------------------------

/// Extension trait providing builder entry-point methods on [`Model`].
///
/// Import this trait to use `model.get()`, `model.insert()`, etc.
pub trait EntityBuilderExt {
    fn get(&self) -> GetBuilder<'_>;
    fn insert(&self) -> InsertBuilder<'_>;
    fn insert_select(&self) -> InsertSelectBuilder<'_>;
    fn update(&self) -> UpdateBuilder<'_>;
    fn remove(&self) -> RemoveBuilder<'_>;
    fn upsert(&self) -> UpsertBuilder<'_>;
    fn alter(&self) -> AlterEntityBuilder<'_>;
    fn drop_entity(&self) -> DropEntityBuilder<'_>;
    fn create(&self) -> definition::CreateFromMeta<'_>;
}

impl EntityBuilderExt for Entity {
    /// Start building a GET (SELECT) query.
    fn get(&self) -> GetBuilder<'_> {
        GetBuilder::new(self)
    }

    /// Start building an INSERT.
    fn insert(&self) -> InsertBuilder<'_> {
        InsertBuilder::new(self)
    }

    /// Start building an INSERT ... SELECT.
    fn insert_select(&self) -> InsertSelectBuilder<'_> {
        InsertSelectBuilder::new(self)
    }

    /// Start building an UPDATE.
    fn update(&self) -> UpdateBuilder<'_> {
        UpdateBuilder::new(self)
    }

    /// Start building a REMOVE (DELETE).
    fn remove(&self) -> RemoveBuilder<'_> {
        RemoveBuilder::new(self)
    }

    /// Start building an UPSERT (INSERT ... ON CONFLICT).
    fn upsert(&self) -> UpsertBuilder<'_> {
        UpsertBuilder::new(self)
    }

    /// Start building an ALTER MODEL (ALTER TABLE).
    fn alter(&self) -> AlterEntityBuilder<'_> {
        AlterEntityBuilder::new(self)
    }

    /// Start building a DROP MODEL (DROP TABLE).
    fn drop_entity(&self) -> DropEntityBuilder<'_> {
        DropEntityBuilder::new(self)
    }

    /// Build a CREATE TABLE from this model's static metadata.
    fn create(&self) -> definition::CreateFromMeta<'_> {
        definition::CreateFromMeta::new(self)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use control::Privilege;
    use dol_entity::{Field, FieldType};
    use dol_expr::{Direction, Expr, NullsPosition, OrderByExpr, field, param, raw_expr};
    use dol_ir::definition::{AlterAction, FieldDef, IndexMethod};
    use dol_ir::storage::ObjectSource;
    use dol_ir::transaction::TransactionIR;
    use dol_ir::{JoinType, LockMode, OffsetLimit};

    static TEST_MODEL: Entity = Entity::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("email", FieldType::Text).unique(),
            Field::new("name", FieldType::Text),
            Field::new("status", FieldType::Text).default("'active'"),
            Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        ],
    );

    static POSTS_MODEL: Entity = Entity::new(
        "posts",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("user_id", FieldType::Uuid),
            Field::new("title", FieldType::Text),
        ],
    );

    // ── GetBuilder tests ────────────────────────────────────────────────

    #[test]
    fn get_minimal_build() {
        let ir = TEST_MODEL.get().build();
        assert_eq!(ir.source.name, "users");
        assert!(ir.source.namespace.is_none());
        assert!(ir.source.alias.is_none());
        assert!(ir.projections.is_empty());
        assert!(ir.joins.is_empty());
        assert!(ir.filters.is_empty());
        assert!(ir.group_by.is_empty());
        assert!(ir.having.is_empty());
        assert!(ir.order_by.is_empty());
        assert!(ir.offset.is_none());
        assert!(ir.limit.is_none());
        assert!(!ir.distinct);
        assert!(ir.distinct_on.is_empty());
        assert!(ir.lock_mode.is_none());
    }

    #[test]
    fn get_all_fields() {
        let ir = TEST_MODEL.get().all_fields().build();
        assert_eq!(ir.projections.len(), 5);
        assert!(matches!(&ir.projections[0], Expr::Identifier(n) if n == "id"));
        assert!(matches!(&ir.projections[4], Expr::Identifier(n) if n == "created_at"));
    }

    #[test]
    fn get_selected_columns() {
        let ir = TEST_MODEL.get().columns(&["id", "email"]).build();
        assert_eq!(ir.projections.len(), 2);
        assert!(matches!(&ir.projections[0], Expr::Identifier(n) if n == "id"));
        assert!(matches!(&ir.projections[1], Expr::Identifier(n) if n == "email"));
    }

    #[test]
    fn get_column_as() {
        let ir = TEST_MODEL.get().column_as("name", "user_name").build();
        assert_eq!(ir.projections.len(), 1);
        assert!(matches!(&ir.projections[0], Expr::Alias { alias, .. } if alias == "user_name"));
    }

    #[test]
    fn get_count_all() {
        let ir = TEST_MODEL.get().count_all().build();
        assert_eq!(ir.projections.len(), 1);
        assert!(matches!(&ir.projections[0], Expr::CountStar));
    }

    #[test]
    fn get_count_all_as() {
        let ir = TEST_MODEL.get().count_all_as("total").build();
        assert_eq!(ir.projections.len(), 1);
        assert!(matches!(&ir.projections[0], Expr::Alias { alias, .. } if alias == "total"));
    }

    #[test]
    fn get_raw_column() {
        let ir = TEST_MODEL.get().raw_column("1 + 1").build();
        assert_eq!(ir.projections.len(), 1);
        assert!(matches!(&ir.projections[0], Expr::Raw(s) if s == "1 + 1"));
    }

    #[test]
    fn get_alias() {
        let ir = TEST_MODEL.get().alias("u").build();
        assert_eq!(ir.source.alias.as_deref(), Some("u"));
    }

    #[test]
    fn get_where_eq() {
        let ir = TEST_MODEL.get().where_eq("id").build();
        assert_eq!(ir.filters.len(), 1);
        assert!(matches!(&ir.filters[0], Expr::BinaryOp { .. }));
    }

    #[test]
    fn get_where_eq_literal() {
        let ir = TEST_MODEL
            .get()
            .where_eq_literal("status", raw_expr("'active'"))
            .build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn get_where_ilike() {
        let ir = TEST_MODEL.get().where_ilike("name").build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn get_where_raw() {
        let ir = TEST_MODEL.get().where_raw("age > 18").build();
        assert_eq!(ir.filters.len(), 1);
        assert!(matches!(&ir.filters[0], Expr::Raw(s) if s == "age > 18"));
    }

    #[test]
    fn get_where_exists() {
        let ir = TEST_MODEL.get().where_exists("SELECT 1 FROM posts").build();
        assert_eq!(ir.filters.len(), 1);
        assert!(
            matches!(&ir.filters[0], Expr::Exists { subquery, negated } if subquery == "SELECT 1 FROM posts" && !negated)
        );
    }

    #[test]
    fn get_where_not_exists() {
        let ir = TEST_MODEL
            .get()
            .where_not_exists("SELECT 1 FROM bans")
            .build();
        assert!(matches!(&ir.filters[0], Expr::Exists { negated, .. } if *negated));
    }

    #[test]
    fn get_where_in_subquery() {
        let ir = TEST_MODEL
            .get()
            .where_in_subquery("id", "SELECT user_id FROM admins")
            .build();
        assert_eq!(ir.filters.len(), 1);
        assert!(matches!(&ir.filters[0], Expr::InSubquery { negated, .. } if !negated));
    }

    #[test]
    fn get_where_not_in_subquery() {
        let ir = TEST_MODEL
            .get()
            .where_not_in_subquery("id", "SELECT user_id FROM banned")
            .build();
        assert!(matches!(&ir.filters[0], Expr::InSubquery { negated, .. } if *negated));
    }

    #[test]
    fn get_multiple_filters() {
        let ir = TEST_MODEL
            .get()
            .where_eq("id")
            .where_eq("status")
            .filter(field("name").eq(param()))
            .build();
        assert_eq!(ir.filters.len(), 3);
    }

    #[test]
    fn get_inner_join() {
        let ir = TEST_MODEL
            .get()
            .inner_join(&POSTS_MODEL, &[("id", "user_id")])
            .build();
        assert_eq!(ir.joins.len(), 1);
        assert_eq!(ir.joins[0].join_type, JoinType::Inner);
        assert_eq!(ir.joins[0].target.name, "posts");
        assert!(ir.joins[0].target.alias.is_none());
        assert_eq!(
            ir.joins[0].on_conditions,
            vec![("id".to_string(), "user_id".to_string())]
        );
    }

    #[test]
    fn get_left_join() {
        let ir = TEST_MODEL
            .get()
            .left_join(&POSTS_MODEL, &[("id", "user_id")])
            .build();
        assert_eq!(ir.joins[0].join_type, JoinType::Left);
    }

    #[test]
    fn get_right_join() {
        let ir = TEST_MODEL
            .get()
            .right_join(&POSTS_MODEL, &[("id", "user_id")])
            .build();
        assert_eq!(ir.joins[0].join_type, JoinType::Right);
    }

    #[test]
    fn get_full_join() {
        let ir = TEST_MODEL
            .get()
            .full_join(&POSTS_MODEL, &[("id", "user_id")])
            .build();
        assert_eq!(ir.joins[0].join_type, JoinType::Full);
    }

    #[test]
    fn get_join_aliased() {
        let ir = TEST_MODEL
            .get()
            .join_aliased(JoinType::Left, &POSTS_MODEL, "p", &[("id", "user_id")])
            .build();
        assert_eq!(ir.joins[0].target.alias.as_deref(), Some("p"));
    }

    #[test]
    fn get_group_by_having() {
        let ir = TEST_MODEL
            .get()
            .columns(&["status"])
            .count_all_as("cnt")
            .group_by(&["status"])
            .having(raw_expr("COUNT(*) > 1"))
            .build();
        assert_eq!(ir.group_by.len(), 1);
        assert!(matches!(&ir.group_by[0], Expr::Identifier(n) if n == "status"));
        assert_eq!(ir.having.len(), 1);
    }

    #[test]
    fn get_order_by_asc() {
        let ir = TEST_MODEL.get().order_by_asc("name").build();
        assert_eq!(ir.order_by.len(), 1);
        assert!(matches!(ir.order_by[0].direction, Direction::Asc));
    }

    #[test]
    fn get_order_by_desc() {
        let ir = TEST_MODEL.get().order_by_desc("created_at").build();
        assert_eq!(ir.order_by.len(), 1);
        assert!(matches!(ir.order_by[0].direction, Direction::Desc));
    }

    #[test]
    fn get_order_by_nulls_last() {
        let ir = TEST_MODEL
            .get()
            .order_by("name", Direction::Asc, Some(NullsPosition::Last))
            .build();
        assert_eq!(ir.order_by[0].nulls, Some(NullsPosition::Last));
    }

    #[test]
    fn get_order_by_nulls_first() {
        let ir = TEST_MODEL
            .get()
            .order_by("name", Direction::Desc, Some(NullsPosition::First))
            .build();
        assert_eq!(ir.order_by[0].nulls, Some(NullsPosition::First));
    }

    #[test]
    fn get_order_by_expr() {
        let obe = OrderByExpr {
            expr: field("email"),
            direction: Direction::Asc,
            nulls: None,
        };
        let ir = TEST_MODEL.get().order_by_expr(obe).build();
        assert_eq!(ir.order_by.len(), 1);
    }

    #[test]
    fn get_offset_limit() {
        let ir = TEST_MODEL.get().offset().limit().build();
        assert!(matches!(ir.offset, Some(OffsetLimit::Param)));
        assert!(matches!(ir.limit, Some(OffsetLimit::Param)));
    }

    #[test]
    fn get_distinct() {
        let ir = TEST_MODEL.get().distinct().build();
        assert!(ir.distinct);
        assert!(ir.distinct_on.is_empty());
    }

    #[test]
    fn get_distinct_on() {
        let ir = TEST_MODEL.get().distinct_on(&["email"]).build();
        assert!(ir.distinct);
        assert_eq!(ir.distinct_on, vec!["email"]);
    }

    #[test]
    fn get_for_update() {
        let ir = TEST_MODEL.get().for_update().build();
        assert_eq!(ir.lock_mode, Some(LockMode::ForUpdate));
    }

    #[test]
    fn get_for_share() {
        let ir = TEST_MODEL.get().for_share().build();
        assert_eq!(ir.lock_mode, Some(LockMode::ForShare));
    }

    #[test]
    fn get_lock_mode() {
        let ir = TEST_MODEL.get().lock(LockMode::ForUpdateSkipLocked).build();
        assert_eq!(ir.lock_mode, Some(LockMode::ForUpdateSkipLocked));
    }

    #[test]
    fn get_param_count_basic() {
        let count = TEST_MODEL
            .get()
            .where_eq("id")
            .offset()
            .limit()
            .param_count();
        // 1 (where_eq) + 1 (offset) + 1 (limit) = 3
        assert_eq!(count, 3);
    }

    #[test]
    fn get_complex_query() {
        let ir = TEST_MODEL
            .get()
            .alias("u")
            .columns(&["id", "name"])
            .inner_join(&POSTS_MODEL, &[("id", "user_id")])
            .where_eq("status")
            .group_by(&["status"])
            .having(raw_expr("COUNT(*) > 5"))
            .order_by_desc("created_at")
            .offset()
            .limit()
            .distinct()
            .for_update()
            .build();

        assert_eq!(ir.source.name, "users");
        assert_eq!(ir.source.alias.as_deref(), Some("u"));
        assert_eq!(ir.projections.len(), 2);
        assert_eq!(ir.joins.len(), 1);
        assert_eq!(ir.filters.len(), 1);
        assert_eq!(ir.group_by.len(), 1);
        assert_eq!(ir.having.len(), 1);
        assert_eq!(ir.order_by.len(), 1);
        assert!(ir.offset.is_some());
        assert!(ir.limit.is_some());
        assert!(ir.distinct);
        assert_eq!(ir.lock_mode, Some(LockMode::ForUpdate));
    }

    // ── InsertBuilder tests ─────────────────────────────────────────────

    #[test]
    fn insert_all_fields() {
        let ir = TEST_MODEL.insert().all_fields().build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(
            ir.fields,
            vec!["id", "email", "name", "status", "created_at"]
        );
        assert_eq!(ir.row_count, 1);
        assert!(ir.returning.is_empty());
    }

    #[test]
    fn insert_selected_columns() {
        let ir = TEST_MODEL.insert().columns(&["email", "name"]).build();
        assert_eq!(ir.fields, vec!["email", "name"]);
    }

    #[test]
    fn insert_multiple_rows() {
        let ir = TEST_MODEL
            .insert()
            .columns(&["email", "name"])
            .rows(3)
            .build();
        assert_eq!(ir.row_count, 3);
    }

    #[test]
    fn insert_returning_all() {
        let ir = TEST_MODEL.insert().all_fields().returning_all().build();
        assert_eq!(ir.returning, vec!["*"]);
    }

    #[test]
    fn insert_returning_specific() {
        let ir = TEST_MODEL
            .insert()
            .columns(&["email", "name"])
            .returning(&["id", "created_at"])
            .build();
        assert_eq!(ir.returning, vec!["id", "created_at"]);
    }

    #[test]
    fn insert_param_count() {
        let b = TEST_MODEL.insert().columns(&["email", "name"]).rows(3);
        assert_eq!(b.param_count(), 6); // 2 fields * 3 rows
    }

    // ── InsertSelectBuilder tests ───────────────────────────────────────

    #[test]
    fn insert_select_basic() {
        let ir = TEST_MODEL
            .insert_select()
            .columns(&["id", "email"])
            .from_select("SELECT id, email FROM temp_users")
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.fields, vec!["id", "email"]);
        assert_eq!(ir.source_query, "SELECT id, email FROM temp_users");
        assert!(ir.returning.is_empty());
    }

    #[test]
    fn insert_select_returning_all() {
        let ir = TEST_MODEL
            .insert_select()
            .columns(&["email"])
            .from_select("SELECT email FROM staging")
            .returning_all()
            .build();
        assert_eq!(ir.returning, vec!["*"]);
    }

    #[test]
    fn insert_select_returning_specific() {
        let ir = TEST_MODEL
            .insert_select()
            .columns(&["email"])
            .from_select("SELECT email FROM staging")
            .returning(&["id"])
            .build();
        assert_eq!(ir.returning, vec!["id"]);
    }

    // ── UpdateBuilder tests ─────────────────────────────────────────────

    #[test]
    fn update_set_single() {
        let ir = TEST_MODEL.update().set("name").where_eq("id").build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.assignments.len(), 1);
        assert_eq!(ir.assignments[0].0, "name");
        assert!(matches!(&ir.assignments[0].1, Expr::Param));
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn update_set_columns() {
        let ir = TEST_MODEL
            .update()
            .set_columns(&["name", "email", "status"])
            .build();
        assert_eq!(ir.assignments.len(), 3);
        assert_eq!(ir.assignments[0].0, "name");
        assert_eq!(ir.assignments[1].0, "email");
        assert_eq!(ir.assignments[2].0, "status");
    }

    #[test]
    fn update_set_literal() {
        let ir = TEST_MODEL
            .update()
            .set_literal("status", "'inactive'")
            .build();
        assert_eq!(ir.assignments[0].0, "status");
        assert!(matches!(&ir.assignments[0].1, Expr::Raw(s) if s == "'inactive'"));
    }

    #[test]
    fn update_set_increment() {
        let ir = TEST_MODEL.update().set_increment("status").build();
        assert_eq!(ir.assignments[0].0, "status");
        assert!(matches!(&ir.assignments[0].1, Expr::BinaryOp { .. }));
    }

    #[test]
    fn update_set_expr() {
        let ir = TEST_MODEL
            .update()
            .set_expr("name", raw_expr("UPPER(name)"))
            .build();
        assert_eq!(ir.assignments[0].0, "name");
        assert!(matches!(&ir.assignments[0].1, Expr::Raw(s) if s == "UPPER(name)"));
    }

    #[test]
    fn update_where_eq_literal() {
        let ir = TEST_MODEL
            .update()
            .set("name")
            .where_eq_literal("status", "'active'")
            .build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn update_where_raw() {
        let ir = TEST_MODEL
            .update()
            .set("name")
            .where_raw("created_at < NOW() - INTERVAL '1 day'")
            .build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn update_returning_all() {
        let ir = TEST_MODEL.update().set("name").returning_all().build();
        assert_eq!(ir.returning, vec!["*"]);
    }

    #[test]
    fn update_returning_specific() {
        let ir = TEST_MODEL
            .update()
            .set("name")
            .returning(&["id", "name"])
            .build();
        assert_eq!(ir.returning, vec!["id", "name"]);
    }

    #[test]
    fn update_param_count() {
        let b = TEST_MODEL.update().set("name").set("email").where_eq("id");
        // 2 set params + 1 where_eq param = 3
        assert_eq!(b.param_count(), 3);
    }

    // ── RemoveBuilder tests ─────────────────────────────────────────────

    #[test]
    fn remove_with_where_eq() {
        let ir = TEST_MODEL.remove().where_eq("id").build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.filters.len(), 1);
        assert!(ir.returning.is_empty());
    }

    #[test]
    fn remove_with_literal_filter() {
        let ir = TEST_MODEL
            .remove()
            .where_eq_literal("status", "'deleted'")
            .build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn remove_with_raw_filter() {
        let ir = TEST_MODEL
            .remove()
            .where_raw("created_at < '2020-01-01'")
            .build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn remove_with_expr_filter() {
        let ir = TEST_MODEL.remove().filter(field("id").eq(param())).build();
        assert_eq!(ir.filters.len(), 1);
    }

    #[test]
    fn remove_multiple_filters() {
        let ir = TEST_MODEL
            .remove()
            .where_eq("id")
            .where_eq("status")
            .build();
        assert_eq!(ir.filters.len(), 2);
    }

    #[test]
    fn remove_returning_all() {
        let ir = TEST_MODEL.remove().where_eq("id").returning_all().build();
        assert_eq!(ir.returning, vec!["*"]);
    }

    #[test]
    fn remove_returning_specific() {
        let ir = TEST_MODEL
            .remove()
            .where_eq("id")
            .returning(&["id", "email"])
            .build();
        assert_eq!(ir.returning, vec!["id", "email"]);
    }

    #[test]
    fn remove_param_count() {
        let b = TEST_MODEL.remove().where_eq("id").where_eq("status");
        assert_eq!(b.param_count(), 2);
    }

    // ── UpsertBuilder tests ─────────────────────────────────────────────

    #[test]
    fn upsert_do_update() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email", "name"])
            .on_conflict(&["id"])
            .do_update(&["email", "name"])
            .build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.fields, vec!["id", "email", "name"]);
        assert_eq!(ir.conflict_fields, vec!["id"]);
        assert!(ir.conflict_constraint.is_none());
        assert_eq!(ir.update_fields, vec!["email", "name"]);
        assert!(!ir.do_nothing);
        assert!(ir.returning.is_empty());
    }

    #[test]
    fn upsert_do_nothing() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["email"])
            .do_nothing()
            .build();
        assert!(ir.do_nothing);
        assert!(ir.update_fields.is_empty());
    }

    #[test]
    fn upsert_all_fields() {
        let ir = TEST_MODEL
            .upsert()
            .all_fields()
            .on_conflict(&["id"])
            .do_update(&["name"])
            .build();
        assert_eq!(ir.fields.len(), 5);
    }

    #[test]
    fn upsert_on_conflict_constraint() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict_constraint("users_email_key")
            .do_nothing()
            .build();
        assert_eq!(ir.conflict_constraint.as_deref(), Some("users_email_key"));
        assert!(ir.conflict_fields.is_empty());
    }

    #[test]
    fn upsert_returning_all() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_nothing()
            .returning_all()
            .build();
        assert_eq!(ir.returning, vec!["*"]);
    }

    #[test]
    fn upsert_returning_specific() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_nothing()
            .returning(&["id"])
            .build();
        assert_eq!(ir.returning, vec!["id"]);
    }

    #[test]
    fn upsert_conflict_filter() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_update(&["email"])
            .conflict_where_eq("status")
            .build();
        assert_eq!(ir.conflict_filters.len(), 1);
    }

    #[test]
    fn upsert_conflict_where_eq_literal() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_update(&["email"])
            .conflict_where_eq_literal("status", "'active'")
            .build();
        assert_eq!(ir.conflict_filters.len(), 1);
    }

    #[test]
    fn upsert_conflict_filter_expr() {
        let ir = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_update(&["email"])
            .conflict_filter(field("status").eq(raw_expr("'active'")))
            .build();
        assert_eq!(ir.conflict_filters.len(), 1);
    }

    #[test]
    fn upsert_param_count() {
        let b = TEST_MODEL
            .upsert()
            .columns(&["id", "email"])
            .on_conflict(&["id"])
            .do_update(&["email"])
            .conflict_where_eq("status");
        // 2 insert fields + 1 conflict where_eq param = 3
        assert_eq!(b.param_count(), 3);
    }

    // ── AlterEntityBuilder tests ─────────────────────────────────────────

    #[test]
    fn alter_add_field() {
        let fd = FieldDef {
            name: "bio".to_string(),
            field_type: FieldType::Text,
            primary_key: false,
            nullable: true,
            default_expr: None,
            unique: false,
            references: None,
            check: None,
            comment: None,
            collation: None,
            generated: None,
            indexed: false,
        };
        let ir = TEST_MODEL.alter().add_field(fd).build();
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.actions.len(), 1);
        assert!(matches!(&ir.actions[0], AlterAction::AddField(f) if f.name == "bio"));
    }

    #[test]
    fn alter_add_field_from_field_def() {
        let ir = TEST_MODEL
            .alter()
            .add_field(FieldDef::new("bio", FieldType::Text).nullable())
            .build();
        assert_eq!(ir.actions.len(), 1);
        assert!(
            matches!(&ir.actions[0], AlterAction::AddField(f) if f.name == "bio" && f.nullable)
        );
    }

    #[test]
    fn alter_drop_field() {
        let ir = TEST_MODEL.alter().drop_field("status").build();
        assert!(matches!(&ir.actions[0], AlterAction::DropField(n) if n == "status"));
    }

    #[test]
    fn alter_rename_field() {
        let ir = TEST_MODEL.alter().rename_field("name", "full_name").build();
        assert!(
            matches!(&ir.actions[0], AlterAction::RenameField { from, to } if from == "name" && to == "full_name")
        );
    }

    #[test]
    fn alter_column_type() {
        let ir = TEST_MODEL
            .alter()
            .alter_column_type("name", FieldType::Varchar(Some(255)))
            .build();
        assert!(matches!(
            &ir.actions[0],
            AlterAction::AlterFieldType { name, new_type }
            if name == "name" && matches!(new_type, FieldType::Varchar(Some(255)))
        ));
    }

    #[test]
    fn alter_set_default() {
        let ir = TEST_MODEL
            .alter()
            .set_default("status", "'pending'")
            .build();
        assert!(
            matches!(&ir.actions[0], AlterAction::SetFieldDefault { name, expr } if name == "status" && expr == "'pending'")
        );
    }

    #[test]
    fn alter_drop_default() {
        let ir = TEST_MODEL.alter().drop_default("status").build();
        assert!(matches!(&ir.actions[0], AlterAction::DropFieldDefault(n) if n == "status"));
    }

    #[test]
    fn alter_set_not_null() {
        let ir = TEST_MODEL.alter().set_not_null("name").build();
        assert!(matches!(&ir.actions[0], AlterAction::SetFieldNotNull(n) if n == "name"));
    }

    #[test]
    fn alter_drop_not_null() {
        let ir = TEST_MODEL.alter().drop_not_null("name").build();
        assert!(matches!(&ir.actions[0], AlterAction::DropFieldNotNull(n) if n == "name"));
    }

    #[test]
    fn alter_add_constraint() {
        use dol_entity::constraint::EntityConstraint;
        let ir = TEST_MODEL
            .alter()
            .add_constraint(EntityConstraint::Unique(&["email", "name"]))
            .build();
        assert!(matches!(&ir.actions[0], AlterAction::AddConstraint(_)));
    }

    #[test]
    fn alter_drop_constraint() {
        let ir = TEST_MODEL
            .alter()
            .drop_constraint("users_email_key")
            .build();
        assert!(matches!(&ir.actions[0], AlterAction::DropConstraint(n) if n == "users_email_key"));
    }

    #[test]
    fn alter_rename_model() {
        let ir = TEST_MODEL.alter().rename_model("people").build();
        assert!(matches!(&ir.actions[0], AlterAction::RenameEntity(n) if n == "people"));
    }

    #[test]
    fn alter_multiple_actions() {
        let ir = TEST_MODEL
            .alter()
            .drop_field("status")
            .rename_field("name", "full_name")
            .set_not_null("email")
            .build();
        assert_eq!(ir.actions.len(), 3);
    }

    // ── DropEntityBuilder tests ──────────────────────────────────────────

    #[test]
    fn drop_model_minimal() {
        let ir = TEST_MODEL.drop_entity().build();
        assert_eq!(ir.target.name, "users");
        assert!(!ir.if_exists);
        assert!(!ir.cascade);
    }

    #[test]
    fn drop_model_if_exists() {
        let ir = TEST_MODEL.drop_entity().if_exists().build();
        assert!(ir.if_exists);
        assert!(!ir.cascade);
    }

    #[test]
    fn drop_model_cascade() {
        let ir = TEST_MODEL.drop_entity().cascade().build();
        assert!(!ir.if_exists);
        assert!(ir.cascade);
    }

    #[test]
    fn drop_model_if_exists_cascade() {
        let ir = TEST_MODEL.drop_entity().if_exists().cascade().build();
        assert!(ir.if_exists);
        assert!(ir.cascade);
    }

    // ── CreateFromMeta tests ────────────────────────────────────────────

    #[test]
    fn create_from_meta_basic() {
        let ir = TEST_MODEL.create().build();
        assert_eq!(ir.name, "users");
        assert!(ir.namespace.is_none());
        assert_eq!(ir.fields.len(), 5);
        assert!(!ir.if_not_exists);

        // Check first field details
        assert_eq!(ir.fields[0].name, "id");
        assert!(matches!(ir.fields[0].field_type, FieldType::Uuid));
        assert!(ir.fields[0].primary_key);
        assert!(!ir.fields[0].nullable);

        // Check unique field
        assert_eq!(ir.fields[1].name, "email");
        assert!(ir.fields[1].unique);

        // Check default expr
        assert_eq!(ir.fields[3].name, "status");
        assert_eq!(ir.fields[3].default_expr.as_deref(), Some("'active'"));

        assert_eq!(ir.fields[4].name, "created_at");
        assert_eq!(ir.fields[4].default_expr.as_deref(), Some("NOW()"));
    }

    #[test]
    fn create_from_meta_if_not_exists() {
        let builder = TEST_MODEL.create().if_not_exists();
        assert!(builder.has_if_not_exists());
        let ir = builder.build();
        assert!(ir.if_not_exists);
    }

    #[test]
    fn create_from_meta_get_model() {
        let builder = TEST_MODEL.create();
        assert_eq!(builder.get_entity().name, "users");
    }

    #[test]
    fn create_from_meta_with_namespace() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("public");

        let ir = NS_MODEL.create().build();
        assert_eq!(ir.name, "accounts");
        assert_eq!(ir.namespace.as_deref(), Some("public"));
    }

    // ── DefineEntityBuilder tests ────────────────────────────────────────

    #[test]
    fn define_model_basic() {
        let ir = DefineEntityBuilder::new("events")
            .field(FieldDef {
                name: "id".to_string(),
                field_type: FieldType::Uuid,
                primary_key: true,
                nullable: false,
                default_expr: None,
                unique: false,
                references: None,
                check: None,
                comment: None,
                collation: None,
                generated: None,
                indexed: false,
            })
            .build();
        assert_eq!(ir.name, "events");
        assert!(ir.namespace.is_none());
        assert_eq!(ir.fields.len(), 1);
        assert!(!ir.if_not_exists);
    }

    #[test]
    fn define_model_with_namespace() {
        let ir = DefineEntityBuilder::new("logs")
            .namespace("analytics")
            .build();
        assert_eq!(ir.namespace.as_deref(), Some("analytics"));
    }

    #[test]
    fn define_model_namespace_alias() {
        let ir = DefineEntityBuilder::new("logs")
            .namespace("analytics")
            .build();
        assert_eq!(ir.namespace.as_deref(), Some("analytics"));
    }

    #[test]
    fn define_model_if_not_exists() {
        let ir = DefineEntityBuilder::new("events").if_not_exists().build();
        assert!(ir.if_not_exists);
    }

    #[test]
    fn define_model_via_model_define_ext() {
        let ir = Entity::define("sessions").if_not_exists().build();
        assert_eq!(ir.name, "sessions");
        assert!(ir.if_not_exists);
    }

    #[test]
    fn define_model_multiple_fields() {
        let fields = vec![
            FieldDef {
                name: "a".to_string(),
                field_type: FieldType::Int,
                primary_key: false,
                nullable: false,
                default_expr: None,
                unique: false,
                references: None,
                check: None,
                comment: None,
                collation: None,
                generated: None,
                indexed: false,
            },
            FieldDef {
                name: "b".to_string(),
                field_type: FieldType::Text,
                primary_key: false,
                nullable: true,
                default_expr: None,
                unique: false,
                references: None,
                check: None,
                comment: None,
                collation: None,
                generated: None,
                indexed: false,
            },
        ];
        let ir = DefineEntityBuilder::new("test").fields(fields).build();
        assert_eq!(ir.fields.len(), 2);
    }

    #[test]
    fn define_model_with_constraint() {
        use dol_entity::constraint::EntityConstraint;
        let ir = DefineEntityBuilder::new("test")
            .constraint(EntityConstraint::Unique(&["a", "b"]))
            .build();
        assert_eq!(ir.constraints.len(), 1);
    }

    // ── DefineIndexBuilder tests ────────────────────────────────────────

    #[test]
    fn define_index_basic() {
        let ir = DefineIndexBuilder::new("idx_users_email")
            .on("users")
            .columns(&["email"])
            .build();
        assert_eq!(ir.name, "idx_users_email");
        assert_eq!(ir.target.name, "users");
        assert_eq!(ir.columns, vec!["email"]);
        assert!(!ir.unique);
        assert!(!ir.if_not_exists);
        assert!(!ir.concurrently);
        assert!(ir.method.is_none());
        assert!(ir.where_clause.is_none());
    }

    #[test]
    fn define_index_unique() {
        let ir = DefineIndexBuilder::new("idx_unique_email")
            .on("users")
            .column("email")
            .unique()
            .build();
        assert!(ir.unique);
    }

    #[test]
    fn define_index_options() {
        let ir = DefineIndexBuilder::new("idx")
            .on("users")
            .columns(&["email"])
            .if_not_exists()
            .concurrently()
            .method(IndexMethod::Hash)
            .where_clause("email IS NOT NULL")
            .build();
        assert!(ir.if_not_exists);
        assert!(ir.concurrently);
        assert!(matches!(ir.method, Some(IndexMethod::Hash)));
        assert_eq!(ir.where_clause.as_deref(), Some("email IS NOT NULL"));
    }

    #[test]
    fn define_index_with_namespace() {
        let ir = DefineIndexBuilder::new("idx")
            .on("users")
            .namespace("public")
            .columns(&["id"])
            .build();
        assert_eq!(ir.target.namespace.as_deref(), Some("public"));
    }

    // ── DropIndexBuilder tests ──────────────────────────────────────────

    #[test]
    fn drop_index_minimal() {
        let ir = DropIndexBuilder::new("idx_users_email").build();
        assert_eq!(ir.name, "idx_users_email");
        assert!(!ir.if_exists);
        assert!(!ir.concurrently);
        assert!(!ir.cascade);
    }

    #[test]
    fn drop_index_all_options() {
        let ir = DropIndexBuilder::new("idx")
            .if_exists()
            .concurrently()
            .cascade()
            .build();
        assert!(ir.if_exists);
        assert!(ir.concurrently);
        assert!(ir.cascade);
    }

    // ── GrantBuilder tests ──────────────────────────────────────────────

    #[test]
    fn grant_select() {
        let ir = GrantBuilder::new(Privilege::Select)
            .on("users")
            .to("reader")
            .build();
        assert_eq!(ir.privilege, Privilege::Select);
        assert_eq!(ir.on_target, "users");
        assert_eq!(ir.to_role, "reader");
    }

    #[test]
    fn grant_all() {
        let ir = GrantBuilder::new(Privilege::All)
            .on("users")
            .to("admin")
            .build();
        assert_eq!(ir.privilege, Privilege::All);
    }

    #[test]
    fn grant_custom_privilege() {
        let ir = GrantBuilder::new(Privilege::Custom("EXECUTE".to_string()))
            .on("my_func")
            .to("runner")
            .build();
        assert_eq!(ir.privilege, Privilege::Custom("EXECUTE".to_string()));
    }

    // ── RevokeBuilder tests ─────────────────────────────────────────────

    #[test]
    fn revoke_insert() {
        let ir = RevokeBuilder::new(Privilege::Insert)
            .on("users")
            .from("writer")
            .build();
        assert_eq!(ir.privilege, Privilege::Insert);
        assert_eq!(ir.on_target, "users");
        assert_eq!(ir.from_role, "writer");
    }

    #[test]
    fn revoke_all() {
        let ir = RevokeBuilder::new(Privilege::All)
            .on("users")
            .from("guest")
            .build();
        assert_eq!(ir.privilege, Privilege::All);
    }

    // ── TransactionBuilder tests ────────────────────────────────────────

    #[test]
    fn transaction_begin() {
        let ir = TransactionBuilder::begin();
        assert!(matches!(ir, TransactionIR::Begin));
    }

    #[test]
    fn transaction_commit() {
        let ir = TransactionBuilder::commit();
        assert!(matches!(ir, TransactionIR::Commit));
    }

    #[test]
    fn transaction_rollback() {
        let ir = TransactionBuilder::rollback();
        assert!(matches!(ir, TransactionIR::Rollback));
    }

    #[test]
    fn transaction_savepoint() {
        let ir = TransactionBuilder::savepoint("sp1");
        assert!(matches!(ir, TransactionIR::Savepoint(ref n) if n == "sp1"));
    }

    #[test]
    fn transaction_release_savepoint() {
        let ir = TransactionBuilder::release_savepoint("sp1");
        assert!(matches!(ir, TransactionIR::ReleaseSavepoint(ref n) if n == "sp1"));
    }

    #[test]
    fn transaction_rollback_to_savepoint() {
        let ir = TransactionBuilder::rollback_to_savepoint("sp1");
        assert!(matches!(ir, TransactionIR::RollbackToSavepoint(ref n) if n == "sp1"));
    }

    #[test]
    fn transaction_block() {
        let ir = TransactionBuilder::block(vec![]);
        assert!(matches!(ir, TransactionIR::Block(stmts) if stmts.is_empty()));
    }

    // ── PutObjectBuilder tests ──────────────────────────────────────────

    #[test]
    fn put_object_from_bytes() {
        let ir = PutObjectBuilder::new("images/logo.png")
            .into_bucket("assets")
            .from_bytes()
            .content_type("image/png")
            .build();
        assert_eq!(ir.key, "images/logo.png");
        assert_eq!(ir.bucket, "assets");
        assert!(matches!(ir.source, ObjectSource::FromBytes));
        assert_eq!(ir.content_type.as_deref(), Some("image/png"));
        assert!(ir.metadata.is_empty());
    }

    #[test]
    fn put_object_from_path() {
        let ir = PutObjectBuilder::new("docs/readme.md")
            .into_bucket("docs")
            .from_path("/data/readme.md")
            .build();
        assert!(matches!(ir.source, ObjectSource::FromPath(ref p) if p == "/data/readme.md"));
    }

    #[test]
    fn put_object_with_metadata() {
        let ir = PutObjectBuilder::new("file.txt")
            .into_bucket("bucket")
            .metadata("author", "test")
            .metadata("version", "1")
            .build();
        assert_eq!(ir.metadata.len(), 2);
        assert_eq!(ir.metadata[0], ("author".to_string(), "test".to_string()));
        assert_eq!(ir.metadata[1], ("version".to_string(), "1".to_string()));
    }

    // ── GetObjectBuilder tests ──────────────────────────────────────────

    #[test]
    fn get_object_basic() {
        let ir = GetObjectBuilder::new("images/logo.png")
            .from_bucket("assets")
            .build();
        assert_eq!(ir.key, "images/logo.png");
        assert_eq!(ir.bucket, "assets");
    }

    // ── ListObjectsBuilder tests ────────────────────────────────────────

    #[test]
    fn list_objects_minimal() {
        let ir = ListObjectsBuilder::new().bucket("mybucket").build();
        assert_eq!(ir.bucket, "mybucket");
        assert!(ir.prefix.is_none());
        assert!(ir.limit.is_none());
        assert!(ir.continuation_token.is_none());
    }

    #[test]
    fn list_objects_default() {
        let ir = ListObjectsBuilder::default().bucket("b").build();
        assert_eq!(ir.bucket, "b");
    }

    #[test]
    fn list_objects_full() {
        let ir = ListObjectsBuilder::new()
            .bucket("data")
            .prefix("logs/")
            .limit(100)
            .continuation_token("abc123")
            .build();
        assert_eq!(ir.prefix.as_deref(), Some("logs/"));
        assert_eq!(ir.limit, Some(100));
        assert_eq!(ir.continuation_token.as_deref(), Some("abc123"));
    }

    // ── ReadFileBuilder tests ───────────────────────────────────────────

    #[test]
    fn read_file_basic() {
        let ir = ReadFileBuilder::new("/data/config.json").build();
        assert_eq!(ir.path, "/data/config.json");
        assert!(ir.encoding.is_none());
    }

    #[test]
    fn read_file_with_encoding() {
        let ir = ReadFileBuilder::new("/data/file.csv")
            .encoding("utf-8")
            .build();
        assert_eq!(ir.encoding.as_deref(), Some("utf-8"));
    }

    // ── WriteFileBuilder tests ──────────────────────────────────────────

    #[test]
    fn write_file_from_bytes() {
        let ir = WriteFileBuilder::new("/output/result.json")
            .from_bytes()
            .build();
        assert_eq!(ir.path, "/output/result.json");
        assert!(matches!(ir.source, ObjectSource::FromBytes));
        assert!(!ir.create_dirs);
    }

    #[test]
    fn write_file_from_path() {
        let ir = WriteFileBuilder::new("/output/copy.txt")
            .from_path("/input/source.txt")
            .build();
        assert!(matches!(ir.source, ObjectSource::FromPath(ref p) if p == "/input/source.txt"));
    }

    #[test]
    fn write_file_create_dirs() {
        let ir = WriteFileBuilder::new("/deep/nested/file.txt")
            .create_dirs()
            .build();
        assert!(ir.create_dirs);
    }

    // ── MoveFileBuilder tests ───────────────────────────────────────────

    #[test]
    fn move_file_basic() {
        let ir = MoveFileBuilder::new("/old/path.txt", "/new/path.txt").build();
        assert_eq!(ir.from, "/old/path.txt");
        assert_eq!(ir.to, "/new/path.txt");
    }

    // ── Namespace propagation tests ─────────────────────────────────────

    #[test]
    fn namespace_propagates_to_get_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.get().build();
        assert_eq!(ir.source.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_insert_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.insert().columns(&["id"]).build();
        assert_eq!(ir.target.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_update_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.update().set("id").build();
        assert_eq!(ir.target.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_remove_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.remove().build();
        assert_eq!(ir.target.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_upsert_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL
            .upsert()
            .columns(&["id"])
            .on_conflict(&["id"])
            .do_nothing()
            .build();
        assert_eq!(ir.target.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_alter_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.alter().drop_field("id").build();
        assert_eq!(ir.target.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_drop_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.drop_entity().build();
        assert_eq!(ir.target.namespace.as_deref(), Some("auth"));
    }

    #[test]
    fn namespace_propagates_to_create_ir() {
        static NS_MODEL: Entity = Entity::new(
            "accounts",
            &[Field::new("id", FieldType::Uuid).primary_key()],
        )
        .with_namespace("auth");

        let ir = NS_MODEL.create().build();
        assert_eq!(ir.namespace.as_deref(), Some("auth"));
    }
}

# DOL — Data Operating Language

A universal, storage-agnostic query and schema language for Rust.

DOL provides a type-safe, composable way to build queries and schema definitions
that target multiple storage backends — SQL databases, key-value stores, and
object storage — from a single, unified API.

```rust
use dol::model::{Entity, Field, FieldType};
use dol::builder::EntityBuilderExt;
use dol::expr::{field, param};
use dol::backend::sql::dialect::Dialect;
use dol::Render;

static USERS: Entity = Entity::new("users", &[
    Field::new("id", FieldType::Uuid).primary_key(),
    Field::new("email", FieldType::Text).unique(),
    Field::new("name", FieldType::Varchar(Some(255))),
    Field::new("created_at", FieldType::Timestamp).default("now()"),
]);

// Render directly to any SQL dialect
let pg = USERS.get()
    .filter(field("email").eq(param()))
    .limit()
    .render(Some(&Dialect::postgres())).unwrap();
// → SELECT id, email, name, created_at FROM users WHERE email = $1 LIMIT $2

let sqlite = USERS.get()
    .filter(field("email").eq(param()))
    .limit()
    .render(None).unwrap(); // default dialect (SQLite)
// → SELECT id, email, name, created_at FROM users WHERE email = ? LIMIT ?
```

## Architecture

DOL follows a three-layer pipeline inspired by SQLAlchemy's Core/Engine separation:

```markdown
Builders → IR → Backends
(human API) (neutral AST) (rendering)
─────────────────────────────────────────────────────
Entity.get()       Statement::Query         SqlBackend
Entity.insert()    Statement::Insert        KvBackend
Entity.upsert()    Statement::Upsert        ObjectStorageBackend
Entity.alter()     Statement::AlterEntity   (your own)
...                ...
```

**Layer 1 — Builders** provide a fluent, method-chain API for constructing operations.

**Layer 2 — IR** is a backend-agnostic intermediate representation that captures the
intent of every operation without being tied to any specific storage engine.

**Layer 3 — Backends** render IR into target-specific output (SQL strings, KV operation
descriptors, storage commands).

Custom engine authors only need `dol-core` to implement the `Backend` trait.

## Workspace Structure

| Crate                                 | Description                                                        |
| ------------------------------------- | ------------------------------------------------------------------ |
| [`dol`](libs/dol)                     | Umbrella crate — re-exports everything under ergonomic paths       |
| [`dol-core`](libs/dol-core)           | Language layer: expressions, models, IR, builders, `Backend` trait |
| [`dol-sql`](libs/dol-sql)             | SQL backend — dialect-aware rendering for 7 databases              |
| [`dol-kv`](libs/dol-kv)               | Key-value backend — renders IR into KV operation descriptors       |
| [`dol-objects`](libs/dol-objects)     | Object storage backend — renders IR into S3-style operations       |
| [`dol-migration`](libs/dol-migration) | Type-safe schema migrations across all backends                    |
| [`dol-config`](libs/dol-config)       | Unified multi-backend configuration from TOML/YAML/JSON files      |

Internal sub-crates (managed by `dol-core`, not intended for direct use):

| Crate         | Role                                                                 |
| ------------- | -------------------------------------------------------------------- |
| `dol-expr`    | Composable expression AST — operators, functions, window expressions |
| `dol-entity`  | Schema language — `Entity`, `Field`, `FieldType`, constraints         |
| `dol-ir`      | Intermediate representation — `Statement` enum and `Backend` trait   |
| `dol-builder` | Method-chain builders that produce IR                                |

## Getting Started

Add `dol` to your project:

```toml
[dependencies]
dol = "0.1"
```

Or enable additional features:

```toml
[dependencies]
dol = { version = "0.1", features = ["full"] }  # everything
```

### Feature Flags

| Feature            | Description                                                |
| ------------------ | ---------------------------------------------------------- |
| `serde` (default)  | Serde support for dialect types                            |
| `config`           | File-based configuration (`DolConfig` from TOML/YAML/JSON) |
| `migration`        | Type-safe schema migration system                          |
| `migration-config` | Migration + config integration                             |
| `full`             | All of the above                                           |

## Usage

### Defining Models

Models are `const`-compatible and zero-cost — define them as statics:

```rust
use dol::model::{Entity, Field, FieldType, FkAction};

static POSTS: Entity = Entity::new("posts", &[
    Field::new("id", FieldType::Uuid).primary_key(),
    Field::new("title", FieldType::Varchar(Some(255))),
    Field::new("body", FieldType::Text).nullable(),
    Field::new("author_id", FieldType::Uuid)
        .references("users", "id", FkAction::Cascade, FkAction::NoAction),
    Field::new("published", FieldType::Bool).default("FALSE"),
    Field::new("created_at", FieldType::Timestamp).default("now()"),
]);
```

Fields are **NOT NULL by default** — call `.nullable()` to opt in.

DOL uses `Entity` as a universal term:

- SQL → table
- Document store → collection
- Object store → bucket schema
- Key-value → key namespace

### Queries

```rust
use dol::builder::EntityBuilderExt;
use dol::expr::{Expr, field, param, lit, raw_expr};
use dol::expr::func;
use dol::Render;

// SELECT with joins, filtering, ordering, pagination
let sql = USERS.get()
    .fields(&["id", "email", "name"])
    .left_join(&POSTS, &[("id", "author_id")])
    .filter(field("email").ilike(param()))
    .order_by_desc("created_at")
    .limit()
    .offset()
    .render(Some(&Dialect::postgres())).unwrap();

// Aggregations with GROUP BY / HAVING
let sql = POSTS.get()
    .fields(&["author_id"])
    .field(Expr::CountStar.alias("post_count"))
    .group_by(&["author_id"])
    .having(func::count(field("*")).gt(lit(5)))
    .render(Some(&Dialect::postgres())).unwrap();

// Subqueries
let active_ids = USERS.get()
    .fields(&["id"])
    .filter(field("active").eq(lit(true)))
    .build();

let sql = POSTS.get()
    .filter(field("author_id").in_subquery(active_ids))
    .render(Some(&Dialect::postgres())).unwrap();
```

### Mutations

```rust
// INSERT with RETURNING
let insert = USERS.insert()
    .fields(&["id", "email", "name"])
    .returning_all()
    .build();

// Batch INSERT (multiple rows)
let batch = USERS.insert()
    .fields(&["id", "email"])
    .rows(3)
    .build();

// UPDATE with SET and WHERE
let update = USERS.update()
    .set("name")
    .set("email")
    .filter(field("id").eq(param()))
    .returning_all()
    .build();

// UPSERT (ON CONFLICT)
let upsert = USERS.upsert()
    .fields(&["id", "email", "name"])
    .on_conflict(&["email"])
    .do_update(&["name"])
    .build();

// DELETE
let remove = USERS.remove()
    .filter(field("id").eq(param()))
    .build();
```

### Schema DDL

```rust
use dol::ir::definition::FieldDef;
use dol::EntityDefineExt;

// CREATE TABLE from model metadata
let create = USERS.create().build();

// ALTER TABLE
let alter = USERS.alter()
    .add_field(FieldDef::new("avatar_url", FieldType::Text).nullable())
    .drop_field("legacy_column")
    .build();

// Define a model programmatically
let table = Entity::define("sessions")
    .field(FieldDef::new("id", FieldType::Uuid).primary_key())
    .field(FieldDef::new("user_id", FieldType::Uuid))
    .field(FieldDef::new("expires_at", FieldType::Timestamp))
    .if_not_exists()
    .build();
```

### Transactions

```rust
use dol::builder::transaction::TransactionBuilder;
use dol::TransactionRender;
use dol::backend::sql::dialect::Dialect;
use dol::ir::Statement;

let insert_ir = USERS.insert().fields(&["id", "email"]).build();
let update_ir = POSTS.update().set("author_id").filter(field("id").eq(param())).build();

let block = TransactionBuilder::block(vec![
    Statement::Insert(insert_ir),
    Statement::Update(update_ir),
]);
let tx_sql = TransactionBuilder::render(&block, Some(&Dialect::postgres())).unwrap();
// → BEGIN;\nINSERT INTO users ...\nUPDATE posts ...\nCOMMIT
```

### Expressions

DOL has a full expression engine with operator overloads:

```rust
use dol::expr::*;

// Composable boolean expressions (& = AND, | = OR, ! = NOT)
let filter = field("age").ge(lit(18))
    & field("status").eq(lit("active"))
    & !field("banned").eq(lit(true));

// SQL functions
let expr = func::coalesce(vec![field("nickname"), field("name"), lit("Anonymous")]);

// Window functions
let ranked = func::row_number()
    .over(
        window::WindowBuilder::new()
            .partition_by(field("department"))
            .order_by(field("salary").desc()),
    )
    .alias("rank");

// CASE expressions
let label = case()
    .when(field("score").ge(lit(90)), lit("A"))
    .when(field("score").ge(lit(80)), lit("B"))
    .otherwise(lit("C"));
```

### Compound Queries

```rust
use dol::CompoundSelectBuilder;
use dol::GetBuilderSqlExt;
use dol::Render;

let active = USERS.get().fields(&["id", "name"]).filter(field("active").eq(lit(true)));
let admins = USERS.get().fields(&["id", "name"]).filter(field("role").eq(lit("admin")));

let union_sql = active.union(admins).render(Some(&Dialect::postgres())).unwrap();
```

## SQL Dialects

DOL ships with 7 built-in dialect presets:

| Dialect          | Constructor              | Param Style | Quoting          | Upsert             | RETURNING         |
| ---------------- | ------------------------ | ----------- | ---------------- | ------------------ | ----------------- |
| PostgreSQL 12+   | `Dialect::postgres()`    | `$1, $2`    | `"double"`       | `ON CONFLICT`      | `RETURNING`       |
| MySQL 8+         | `Dialect::mysql()`       | `?`         | `` `backtick` `` | `ON DUPLICATE KEY` | —                 |
| MariaDB 10.5+    | `Dialect::mariadb()`     | `?`         | `` `backtick` `` | `ON DUPLICATE KEY` | `RETURNING`       |
| SQLite 3.35+     | `Dialect::sqlite()`      | `?`         | `"double"`       | `ON CONFLICT`      | `RETURNING`       |
| SQL Server 2016+ | `Dialect::mssql()`       | `@p1, @p2`  | `[bracket]`      | `MERGE`            | `OUTPUT INSERTED` |
| Oracle 12c+      | `Dialect::oracle()`      | `:1, :2`    | `"double"`       | `MERGE`            | `RETURNING INTO`  |
| CockroachDB      | `Dialect::cockroachdb()` | `$1, $2`    | `"double"`       | `ON CONFLICT`      | `RETURNING`       |

Each dialect configures: parameter style, identifier quoting, logical-to-physical type
mappings, pagination strategy, upsert syntax, RETURNING clause support, DDL capabilities
(auto-increment, enums, IF NOT EXISTS, IF EXISTS), row-level locking, JSON access style,
array literal syntax, string concatenation, and boolean literals.

### Setting a Global Default

```rust
use dol::backend::sql::dialect::{set_default_dialect, Dialect};

// Set once at startup (defaults to SQLite if not called)
set_default_dialect(Dialect::postgres()).ok();
```

### Custom Dialects from Config

With the `config` feature, load custom dialects from TOML/YAML/JSON:

```rust
let dialect = Dialect::from_file("dialects/custom.toml").unwrap();
```

## Migrations

Migrations are written in Rust using DOL IR — not raw SQL files. This gives
type safety, backend agnosticism, and bidirectional (up/down) support.

```rust
use dol::migration::*;
use dol::model::FieldType;
use dol::ir::definition::FieldDef;
use dol::EntityDefineExt;
use dol::model::Entity;

struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn version(&self) -> u64 { 1 }
    fn description(&self) -> &str { "Create users table" }

    fn up(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::define_model(
            Entity::define("users")
                .field(FieldDef::new("id", FieldType::Uuid).primary_key())
                .field(FieldDef::new("email", FieldType::Text).unique())
                .field(FieldDef::new("name", FieldType::Text))
                .if_not_exists()
                .build(),
        )]
    }

    fn down(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::drop_model("users")]
    }
}
```

### Multi-Backend Migrations

Migration steps can span SQL, key-value, and object storage in a single migration:

```rust
fn up(&self) -> Vec<MigrationStep> {
    vec![
        // SQL: create table
        MigrationStep::define_model(/* ... */),
        // KV: create namespace
        MigrationStep::create_kv_namespace("user_sessions"),
        // Storage: create bucket
        MigrationStep::create_bucket("user-avatars"),
    ]
}
```

### Schema Diff Engine

Automatically compute the minimal set of `ALTER` actions by comparing model snapshots:

```rust
use dol::migration::schema_diff::*;

let old = EntitySnapshot::from_entity(&V1_USERS);
let new = EntitySnapshot::from_entity(&V2_USERS);

let actions = diff_entities(&old, &new);
// → [AddField("avatar_url"), DropField("legacy"), AlterFieldType { name: "count", new_type: BigInt }]

let steps = diff_to_steps("users", &old, &new);
```

### Migration Runner

```rust
use dol::migration::{MigrationRunner, InMemoryRegistry};
use dol::backend::sql::dialect::Dialect;

let mut runner = MigrationRunner::new(InMemoryRegistry::new(), Dialect::postgres());
runner.register(Box::new(CreateUsersTable));

let plan = runner.plan_forward().unwrap();
let rendered = runner.render_plan(&plan);

for step in rendered {
    // Execute against your database connection
    println!("{}", step.sql);
}
```

## Configuration

With the `config` feature, load all backend configuration from a single file:

```toml
# dol.toml

[sql]
url = "postgres://localhost:5432/mydb"
dialect = "postgresql"
max_connections = 20

[sql.pool]
min = 5
max = 20
idle_timeout_secs = 300

[kv]
provider = "redis"

[kv.redis]
url = "redis://localhost:6379"
default_ttl_secs = 3600

[storage]
provider = "s3"

[storage.s3]
bucket = "my-app-assets"
region = "us-east-1"

[migrations]
table_name = "_dol_migrations"
lock_strategy = "advisory_lock"
```

```rust
use dol::config::DolConfig;

let config = DolConfig::from_file("dol.toml").unwrap();

if let Some(sql) = &config.sql {
    println!("Database: {}", sql.primary.url);
}
```

Supports TOML, YAML, and JSON. Enterprise features include read replicas,
named instances for isolated workloads, and per-backend migration filtering.

## Custom Backends

Implement the `Backend` trait to add support for any storage system:

```rust
use dol_core::op::{Backend, BackendError, RenderedOutput, Statement};

struct MyCustomBackend;

impl Backend for MyCustomBackend {
    fn render(&self, stmt: &Statement) -> Result<RenderedOutput, BackendError> {
        match stmt {
            Statement::Query(q) => {
                // Render to your custom format
                todo!()
            }
            _ => Err(BackendError::Unsupported(
                "Only queries are supported".into()
            )),
        }
    }
}
```

Only `dol-core` is needed as a dependency — no coupling to `dol-sql` or any other backend.

## Building

```bash
# Check the entire workspace
cargo check --workspace

# Run all tests
cargo test --workspace

# Run tests with all features
cargo test --workspace --all-features

# Format
cargo fmt --all

# Lint
cargo clippy --workspace --all-features
```

**Requirements:** Rust 1.85+ (edition 2024)

## License

BSD 3-Clause License. See [LICENSE](LICENSE) for details.

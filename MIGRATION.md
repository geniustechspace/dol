# Migrating from `dol-ir` v1 to v2

The v2 IR introduces:

- [`Operation`](docs/IR.md) as the universal operation enum (replaces
  `Statement`, which is retained as a deprecated compat alias under
  `dol_ir::compat::statement`).
- [`Target`](docs/IR.md) / [`Locator`](docs/IR.md) /
  [`SchemaBinding`](docs/IR.md) as the universal addressing primitive.
- A schema catalog: operations carry a `SchemaRef`, not inline bodies.
- Open [`CapabilityTag`](docs/IR.md) / [`CapabilitySet`](docs/IR.md)
  vocabulary alongside the existing `BackendCapabilities` bitset.
- A versioned [`Program`](docs/IR.md) envelope (`IR_SCHEMA_VERSION = 2`)
  and a borrowed [`ProgramRef<'_>`](docs/IR.md) view that backends
  consume.
- Structured [`BackendError`](docs/IR.md) carrying
  `dol_core::Diagnostic` + optional `Span`, and `#[non_exhaustive]`.

This file shows what changes for callers. Old API on the left, new API
on the right.

## Compatibility

The `compat::statement` module re-exports every v1 type and provides

```rust
fn statement_to_operation(stmt: &Statement) -> Operation;
impl From<Statement> for Operation { /* … */ }
impl From<&Statement> for Operation { /* … */ }
```

so existing code that emits `Statement` is auto-converted into v2 when
it lands in a `Program`. `Program::from_stmt` and `Program::stmt` keep
working — the v2 `Operation` equivalent is mirrored on
`Program::operations` automatically.

## Mapping table

| v1 (Statement)                                   | v2 (Operation)                                                     |
| ------------------------------------------------ | ------------------------------------------------------------------ |
| `Statement::Query(_)`                            | `Operation::Query` against `TargetKind::Relation`                  |
| `Statement::Insert(_)`                           | `Operation::Insert`                                                |
| `Statement::Update(_)`                           | `Operation::Update` (partial / SET semantics)                      |
| `Statement::Delete(_)`                           | `Operation::Delete`                                                |
| `Statement::Upsert(_)`                           | `Operation::Upsert`                                                |
| `Statement::DefineEntity(_)`                     | `Operation::Schema { verb: Create, body: Entity{…}, target }`     |
| `Statement::AlterEntity(_)`                      | `Operation::Schema { verb: Alter, … }` and/or `Operation::Field`  |
| `Statement::DropEntity(_)`                       | `Operation::Schema { verb: Drop, … }`                              |
| `Statement::DefineLookup(_)`                     | `Operation::Lookup { verb: Create, … }`                            |
| `Statement::DropLookup(_)`                       | `Operation::Lookup { verb: Drop, … }`                              |
| `Statement::DefineType(_)`                       | `Operation::Schema { verb: Create, body: Type{…}, target }`       |
| `Statement::DropType(_)`                         | `Operation::Schema { verb: Drop, … }`                              |
| `Statement::Grant(_)` / `Statement::Revoke(_)`   | `Operation::Grant` / `Operation::Revoke`                           |
| `Statement::DefinePolicy(_)`                     | `Operation::Policy { verb: Create, … }`                            |
| `Statement::Transaction(_)`                      | `Operation::Tx(TxOp::*)`                                           |
| `Statement::PutObject(_)`                        | `Operation::Insert` against `TargetKind::Blob`                     |
| `Statement::GetObject(_)`                        | `Operation::Query` against `TargetKind::Blob`                      |
| `Statement::ListObjects(_)`                      | `Operation::Query` against `TargetKind::Blob` (or `Probe`)         |
| `Statement::ReadFile(_)`                         | `Operation::Query` against `TargetKind::FileTree`                  |
| `Statement::WriteFile(_)`                        | `Operation::Replace` against `TargetKind::FileTree`                |
| `Statement::MoveFile(_)`                         | `Operation::Update` against `TargetKind::FileTree`                 |
| `Statement::Raw(_)`                              | `Operation::Raw` *(feature-gated `raw`)*                           |
| `Statement::Extension(_)`                        | `Operation::Extension`                                             |

## Side-by-side examples

### Schema definition

| Old                                                         | New                                                                                    |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `Statement::DefineEntity(DefineEntity { name, fields, … })` | `SchemaOp::create_entity(target, schema_ref, if_not_exists).into()`                    |

### Alter entity

| Old                                                                 | New                                                                              |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `Statement::AlterEntity(AlterEntity { actions: vec![AddField(…)] })`| `Operation::Field(FieldOp { verb: Create, target, field, def, new_name: None })` |

### Query

| Old                                            | New                                                                       |
| ---------------------------------------------- | ------------------------------------------------------------------------- |
| `Statement::Query(Box::new(QueryNode { … }))`  | `Query { target, node: Some(arena_node) }.into()`                          |

### Upsert

| Old                                            | New                                                                      |
| ---------------------------------------------- | ------------------------------------------------------------------------ |
| `Statement::Upsert(_)`                         | `Upsert { target, source, conflict_keys, on_conflict, returning }.into()`|

### Policy

| Old                                                              | New                                                                                              |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `Statement::DefinePolicy(DefinePolicy { name, action, using_expr, … })` | `PolicyOp { verb: Create, target, name, scope, using_expr, check_expr }.into()`           |

### Grant

| Old                                                        | New                                                                                  |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `Statement::Grant(Grant { privilege, on_target, to_role })`| `GrantV2 { privileges: smallvec![priv], target, roles: smallvec![role], with_grant_option: false }.into()` |

### Transaction

| Old                                                         | New                                                                            |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `Statement::Transaction(Transaction::Block(stmts))`         | `Operation::Tx(TxOp::Atomic { ops, opts: TxOptions::default() })`              |

### File write

| Old                                            | New                                                                  |
| ---------------------------------------------- | -------------------------------------------------------------------- |
| `Statement::WriteFile(WriteFile { path, source, create_dirs })` | `Replace { target: filetree(path), body, filter: None }.into()` |

### Object put

| Old                                            | New                                                                       |
| ---------------------------------------------- | ------------------------------------------------------------------------- |
| `Statement::PutObject(PutObject { bucket, key, source, … })`| `Insert { target: blob(bucket, key), source: InsertSource::Bindings, returning: None }.into()` |

## Backend trait change

```rust
// v1
fn compile(&self, program: &Program) -> Result<Self::Output, BackendError>;

// v2
fn compile(&self, program: ProgramRef<'_>) -> Result<Self::Output, BackendError>;
```

Use `program.as_ref()` (or the blanket `impl From<&Program> for ProgramRef<'_>`)
to obtain a `ProgramRef` from a `Program`.

## `BackendError`

```rust
// v1
match err {
    BackendError::Unsupported(msg) => …,
    BackendError::MissingValue(msg) => …,
    BackendError::Render(msg) => …,
}

// v2 — `#[non_exhaustive]`
match err {
    BackendError::Unsupported { diag, span } => …,
    BackendError::MissingValue { diag, span } => …,
    BackendError::Render { diag, span } => …,
    BackendError::Capability { check, diag, span } => …,
    BackendError::Extension { diag, span } => …,
    BackendError::AclDenied { diag, span } => …,
    _ => …,
}
```

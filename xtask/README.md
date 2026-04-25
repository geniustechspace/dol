# `xtask`

Workspace task runner. Run via:

```bash
cargo run -p xtask -- <subcommand>
```

## Subcommands

| Command  | Purpose                                                        |
| -------- | -------------------------------------------------------------- |
| `size`   | print `size_of` for the public size-budgeted IR types; fails CI on regression |
| `nostd`  | run `cargo check --no-default-features` on the no_std crates (`dol-arena`, `dol-span`, `dol-diag`) |
| `doc`    | build workspace docs with all features                         |
| `readme` | verify every workspace member has a non-empty `README.md`      |
| `help`   | print this list                                                |

## Size budgets enforced by `xtask size`

```text
size_of::<dol_types::Value>()            ≤ 24
size_of::<dol_types::Literal<'static>>() ≤ 32
size_of::<dol_expr::ExprNode>()          ≤ 32
size_of::<dol_ir::Statement>()           ≤ 64   (currently 24)
```

See [`justfile`](../justfile) for higher-level recipes that wrap these.

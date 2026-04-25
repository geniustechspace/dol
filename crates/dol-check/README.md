# `dol-check`

Static validator for DOL programs. Every backend that consumes a
`dol_ir::Program` is expected to call these checks first. They are pure
functions that produce a list of `dol_diag::Diagnostic`s; an empty list
means the program is well-formed.

The checker is split into independent passes:

- **`type_check`** — operand types are consistent.
- **`schema_check`** — entity / field references exist and are valid.
- **`capability_check`** — the program does not use features the chosen
  backend lacks.
- **`lint`** — style / best-practice findings.

All passes are append-only on the diagnostic list: callers may run only the
passes they need. `check_all` runs `type_check`, `schema_check`, and `lint`
in order.

## Features

| feature | default | effect                                  |
| ------- | :-----: | --------------------------------------- |
| `serde` |         | `Serialize` derives across diagnostics  |

## Example

```rust,ignore
let diags = dol_check::check_all(&program);
if diags.iter().any(|d| d.severity.is_error()) { /* … */ }
```

See the rustdoc for the full API.

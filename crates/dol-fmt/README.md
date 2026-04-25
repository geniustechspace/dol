# `dol-fmt`

Canonical pretty-printer. Renders an IR `dol_ir::Program` into a stable,
human-readable text form. The output is **not** a parser surface; it exists
for debugging, `insta`-style golden tests, and round-trip checks against
`dol-wire`.

The text form is loosely S-expression-flavoured:

```text
(program
  (transaction (begin))
  (set "search_path" "public")
  (transaction (commit)))
```

The exact shape is governed by snapshot tests; downstream tooling should
treat any change as a breaking change.

## Features

| feature | default | effect                                  |
| ------- | :-----: | --------------------------------------- |
| `serde` |         | forwards `serde` to `dol-ir`/`dol-expr` |

## Example

```rust,ignore
let text = dol_fmt::print(&program);
println!("{text}");
```

See the rustdoc for the full API.

# `dol`

Umbrella facade crate. Re-exports the individual `dol-*` crates behind cargo
feature flags so downstream consumers can pick the slice of the stack they
need without pulling in the full dependency tree.

## Layered surface

| Feature      | Pulls in       | Purpose                                                    |
| ------------ | -------------- | ---------------------------------------------------------- |
| *(default)*  | `dol-core`     | Spans, diagnostics, and the value/type system (`Value`, `Literal`, `DataType`, …). |
| `expr`       | `dol-expr`     | Expression arena + tree DSL.                               |
| `schema`     | `dol-schema`   | Entities, fields, constraints, relations, lookups.         |
| `command`    | `dol-command`  | `Operation`, `Program`, `Backend`, `BackendCapabilities`, plus the DDL/ACL/Tx/storage builders. |
| `wire`       | `dol-wire`     | Canonical wire envelope + postcard / JSON codec helpers.   |
| `check`      | `dol-check`    | Static validator (type / schema / capability / lint).      |
| `fmt`        | `dol-fmt`      | Canonical pretty-printer.                                  |
| `query`      | `dol-query`    | Fluent query builder DSL, plus the streaming / pipeline / IoT IR (formerly the separate `dol-stream` and `dol-pipeline` crates). |

## Curated presets

| Preset    | Layers                                            | Use case                                        |
| --------- | ------------------------------------------------- | ----------------------------------------------- |
| `core`    | `expr + schema + command + query`                 | "Full programs" build, no codecs / streaming    |
| `full`    | every layer DOL ships                             | Library / tooling consumers                     |
| `iot-min` | `expr + schema + command + query + wire/postcard` | Minimal IoT-edge slice (fits `thumbv7em` budget) |

The `iot-min` preset and the `no_std` leaves are verified in CI:

- `cargo check -p dol --no-default-features --features iot-min` runs on every
  build (host compile-only).
- `cargo check -p dol-core -p dol-expr --no-default-features
  --target thumbv7em-none-eabihf` runs in a dedicated cross-compile job.

## Universal `serde`

Adding the `serde` feature turns on `serde` on every active sub-crate via
`?` activations, so it is a no-op on layers you have not enabled.

Note: the leaf crates `dol-core` and `dol-schema` already enable `serde` by
default, because the cost is negligible and they're almost always used with
serialization. Disable them with `default-features = false` on each crate
individually if you need the no-op build.

## Example

```toml
# Cargo.toml
[dependencies]
dol = { version = "0.1", features = ["core", "wire", "serde"] }
```

```rust,ignore
use dol::query::Query;
let program = Query::from("users").get().build();
```

See the rustdoc and the per-crate READMEs for the full surface.

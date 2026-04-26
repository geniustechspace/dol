# `backends/`

Reserved bucket for concrete `dol-ir::Backend` implementations. Each crate
here ships an executor or SQL-emitter for one storage technology and is
named `dol-backend-<store>` (folder: `backends/<store>/`).

## Invariants

A crate under `backends/` may depend only on `dol-ir` (always) and
`dol-wire` (optional). It must **not** depend on any other backend, on the
`tools/*` crates, or on the umbrella `dol`.

## Adding a backend

1. Create `backends/<store>/` with a `Cargo.toml` whose `[package] name =
   "dol-backend-<store>"`.
2. Add `"backends/*"` back into the root `Cargo.toml`'s
   `[workspace] members` array (it is currently commented out because cargo
   workspace globs error when they match zero crates).
3. Implement `dol_ir::Backend` in `src/lib.rs`.

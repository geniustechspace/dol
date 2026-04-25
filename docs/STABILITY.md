# DOL Stability Policy

This document describes the stability guarantees of the DOL public Rust API
and the on-disk / on-the-wire serialized representation produced by the
`serde` feature on `dol-types` and `dol-expr`.

The policy applies to the language layer (`dol-types` and `dol-expr`).
Higher-level crates inherit the same conventions, but their own stability
contracts are documented in their respective READMEs.

> While DOL is in `0.x`, all guarantees below are best-effort. The policy
> takes effect at `1.0`. We document it now so the implementation evolves
> in a stability-friendly direction.

## Versioning

DOL follows [SemVer 2.0](https://semver.org/) at the *crate* granularity.
Breaking changes are batched into a single major release wherever
practical.

A change is **breaking** if any of the following hold:

* A public type, function, or trait is renamed or removed.
* A public function's signature changes in a non-source-compatible way.
* A `#[non_exhaustive]` enum gains a variant that callers were already
  matching against using `..` (allowed) — but **not** if they used a
  wildcard (also allowed). In short: `#[non_exhaustive]` makes adding
  variants non-breaking by design.
* The MSRV (`rust-version` in `Cargo.toml`) increases. We will increase
  MSRV only in a minor release after at least one Rust toolchain release
  cycle.

## Public surface

* All public enums and growable public structs that can plausibly grow
  new variants/fields are marked `#[non_exhaustive]`. Adding a variant
  or field is then a non-breaking change.
* Public structs whose field set is conceptually closed (e.g. `Date {
  year, month, day }`, `MacAddr([u8; 6])`) are *not* `#[non_exhaustive]`.
  Adding fields to them is a breaking change.
* New public items may be added in a minor release. Removing or renaming
  one is a breaking change.

## Wire format

The serde representation produced by

* `serde_json` — the canonical human-readable form, and
* `postcard` — the canonical binary form

is part of the public stability contract.

### Variant naming

The variant names you see in JSON match the Rust enum variant names. We
will not silently rename variants. Renames are batched into a major
release alongside a deprecation notice.

### Field ordering

Field order in serialized output is the order declared in source. We
won't reorder fields without bumping the major version, because some
binary formats (e.g. CBOR with `bytes`-mode keys) are sensitive to it.

### Non-exhaustive enums

Adding a new variant to a `#[non_exhaustive]` enum is *not* breaking at
the source level, but it is a *forward-compatible* change at the wire
level: older deserializers will fail to decode messages containing the
new variant. We treat this as a minor-release-allowed change. Producers
that need to interoperate with old consumers must avoid emitting new
variants until the consumer base has updated.

### Interner determinism

`dol_expr::Interner`'s serde codec writes the canonical
`Vec<Arc<str>>` in insertion order. Two `Interner` instances populated
with the same string sequence in the same order produce byte-identical
output across `postcard` and `serde_json`. Map iteration order is *not*
serialized; the map is rebuilt on deserialization.

### Sizes are not part of the contract

The `xtask size` gate (`size_of::<ExprNode> == 32`) is an internal
performance guarantee, not part of the public ABI. We may relax or
tighten it across minor releases. Downstream code must treat all
arena/IR types as opaque-by-size.

## Feature flags

* `std` and `serde` features may grow more capability in minor releases.
* Removing a feature flag is a breaking change; renaming one is breaking
  unless the old name continues to work as a no-op forwarder for at
  least one minor release.
* Enabling a feature must never change observable behaviour of code that
  did not opt into that feature (the *additive* feature rule).

## Deprecation

Deprecations use `#[deprecated]` with a `since` version and a `note`. A
deprecated item must keep working for at least one minor release before
removal in the next major release.

## Unsafe code

The language layer carries `#![deny(unsafe_code)]`. Introducing `unsafe`
in either crate requires a code review by at least two maintainers and a
public rationale in the commit message.

## Reporting

If you believe DOL has broken any of the above guarantees, open an issue
or a PR; we treat stability regressions as bugs.

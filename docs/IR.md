# DOL IR

The DOL IR is the universal intermediate representation that every backend
ingests. It is a single Rust enum, [`Operation`], plus a small ecosystem of
addressing primitives ([`Target`], [`Locator`], [`SchemaRef`],
[`SchemaCatalog`]) and a capability vocabulary ([`CapabilityTag`],
[`CapabilitySet`], [`CapabilityCheck`]).

This document captures the **rules** that govern the IR's shape. RFC
[`docs/rfcs/0001-ir.md`](rfcs/0001-ir.md) tells the design story; this
file is the reference.

## The noun-vs-verb rule

Every variant of [`Operation`] obeys exactly one of three categories:

1. **Structural / governance** variants are **nouns**:
   `Schema`, `Field`, `Index`, `Lookup`, `Policy`, `Mask`, `Quota`, `Audit`.

   Each noun payload carries a [`StructuralVerb`] (`Create`, `Drop`,
   `Alter`, `Rename`, `Truncate`). The variant name describes *what kind of
   thing* the operation acts on; the embedded `verb` describes *what is
   being done to it*.

2. **Data / query / authorization** variants are **verbs**:
   `Insert`, `Update`, `Replace`, `Delete`, `Upsert`, `Append`, `Query`,
   `Probe`, `Describe`, `Grant`, `Revoke`.

   Each verb payload carries the data it acts on directly. There is no
   embedded structural verb; the variant name *is* the verb.

3. **Meta** variants: `Tx` (transaction control), `Extension` (typed
   open-ended payload), and the feature-gated `Raw` (pre-built dialect
   bodies).

The rule has two practical consequences:

- The same verb (`Create`, `Drop`, …) is reused across all noun variants
  via [`StructuralVerb`], so backends that handle generic structural
  changes can match once on the verb.
- The same payload shape (`Insert`, `Query`, …) is reused across all
  [`TargetKind`]s, so storage / file / stream / API operations *are not*
  separate variants. They are an `Insert`/`Query`/`Replace`/`Update`
  against `TargetKind::Blob` / `FileTree` / `StreamTopic` / `ApiResource`.

## Targets, locators, and schemas

Every operation that has a primary subject carries a [`Target`]:

```text
Target { kind: TargetKind, locator: Locator, alias: Option<Symbol>,
         schema: SchemaBinding }
```

- [`TargetKind`] declares *what kind of thing* is being addressed
  (`Relation`, `Document`, `KeyValue`, `Blob`, `FileTree`, `StreamTopic`,
  `ApiResource`, `Virtual`).
- [`Locator`] is structured data, not a string:
  `Locator { namespace, name, path }`. `path` covers nested document
  paths, S3 key segments, file-tree segments, and topic partitions.
- [`SchemaBinding`] is one of `Declared(SchemaRef)`, `Inferred`, or
  `Opaque`. Schemas live in a catalog ([`SchemaCatalog`]); operations
  carry a `SchemaRef`, never an inline schema body.

## Expression slots

Every expression slot in every payload is an arena
[`NodeId`](dol_expr::ids::NodeId) — never a `String`. This is enforced by
review: any `String`-typed expression slot in a payload is a bug. See RFC
§3.

## Symbols

All `String`-typed names (locator names, aliases, role names,
savepoints, …) are wrapped in [`Symbol`] (a newtype around a `u32`
interner index). This makes equality cheap and eliminates per-operation
allocations on the hot path.

## Capabilities

Two complementary capability surfaces:

- The closed [`BackendCapabilities`] `u64` bitset for the hot path.
- The open [`CapabilityTag`] vocabulary, accumulated into a
  [`CapabilitySet`], used by [`Operation::required_capabilities`] and by
  `dol-check` for forward-compatible diagnostics.

`dol-check` reports each missing tag with a structured
[`CapabilityCheck`] key:

```text
operation Append on TargetKind::StreamTopic requires capability
`STREAM_TARGETS` not provided by backend
```

## Static invariants

- `size_of::<Operation>() ≤ 64` — enforced by `xtask size` and
  `const _: () = assert!(...);` in `lib/ir/src/operation/mod.rs`.
- `Operation: Send + Sync + 'static` — enforced by a static assertion.
- `#![deny(unsafe_code)]` is retained throughout the crate.
- Every payload derives `Debug, Clone, PartialEq` and is
  `Serialize/Deserialize` under the `serde` feature.

## Wire format

Persistent serialisation lives in [`dol-wire`](../lib/wire), which carries
its own header / framing version independently of the in-memory IR. The IR
itself is a single, current shape with no embedded schema-version envelope.

[`Operation`]: ../lib/ir/src/operation/mod.rs
[`Target`]: ../lib/ir/src/target.rs
[`Locator`]: ../lib/ir/src/target.rs
[`TargetKind`]: ../lib/ir/src/target.rs
[`SchemaBinding`]: ../lib/ir/src/target.rs
[`SchemaRef`]: ../lib/ir/src/schema_ref.rs
[`SchemaCatalog`]: ../lib/ir/src/schema_catalog.rs
[`Symbol`]: ../lib/ir/src/target.rs
[`StructuralVerb`]: ../lib/ir/src/operation/schema.rs
[`CapabilityTag`]: ../lib/ir/src/capabilities.rs
[`CapabilitySet`]: ../lib/ir/src/capabilities.rs
[`CapabilityCheck`]: ../lib/ir/src/capabilities.rs
[`BackendCapabilities`]: ../lib/ir/src/capabilities.rs
[`Operation::required_capabilities`]: ../lib/ir/src/capabilities.rs
[`IR_SCHEMA_VERSION`]: ../lib/ir/src/version.rs
[`VersionedProgram`]: ../lib/ir/src/version.rs

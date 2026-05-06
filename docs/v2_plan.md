Is this the complete v2 we discussed? Remember we don't need to complier with any backward compatibility, and the entire project is not published yet, so we need to do what's best, rather than patching or complying with existing constraints or implementation decisions.
All we care about is performance, security, less resource consumptions, and best of developer workflow and codebase navigation.
Finalized plan: one coordinated cut, core-first, no compatibility shims
North star
Pick the right shape for every crate now. No Deserialize migrators, no v1/v2 feature gates, no deprecation aliases. Re-export the new names, delete the old. Cut a single 0.2.0 across the workspace when it's done.

Non-negotiable invariants (enforced in CI from day one) #![forbid(unsafe_code)] workspace-wide except one auditable crate lib/core/src/raw/ (used only for bytemuck/zerocopy casts of POD wire types). forbid not deny — already a step up from current state.
no_std + alloc is default; std is opt-in everywhere except tools/\* and xtask.
Zero unwrap/expect/panic! in non-test code — clippy::unwrap_used, expect_used, panic, indexing_slicing, arithmetic_side_effects set to deny workspace-wide.
All recursive entry points take a &mut Budget. No exceptions. Lint via a custom dylint rule or a grep gate in xtask.
All deserialization paths are validating. No serde::Deserialize on any in-memory IR/AST type. Wire-in goes through dol-wire::Decoder only.
MSRV pinned to 1.85, toolchain 1.94 (per stored memories). Re-verified.
Every public API has a doc example that compiles in no_std mode — cargo test --doc --no-default-features is a CI gate.
Shape of each crate after the cut
lib/core — the foundation everyone shares
Owns: spans, diagnostics, ids, limits, storage, signing traits, error codes, value/type system, common datatypes (today's contents), and a tiny prelude.

New modules:

core::policy — Limits, Budget, Quota. const fn iot(), const fn host(), const fn fuzz(). Thread through everything.
core::id — Id<Tag>(NonMaxU32, PhantomData<Tag>). Option<Id<T>> is 4 bytes. Replace every bespoke NodeId/StrId/StmtId/FieldId newtype.
core::storage — trait Storage<T> with impls for Vec<T>, &'a [T], heapless::Vec<T,N>, arrayvec::ArrayVec<T,N>. Arenas elsewhere are generic over it.
core::diag::ErrorCode — stable u16 codes. Display impls feature-gated; codes alone are no_std + no_alloc.
core::sign — trait Verifier, trait KeyResolver. No crypto in core.
core::hash — re-export blake3 with a Hasher32 and Hasher128 newtype that wraps it; everyone uses these, never blake3 directly. Lets us swap if needed.
core::raw — only place unsafe is allowed; pure POD casts via bytemuck. Audited.
Drop:

serde::Deserialize impls on Value, Literal, FieldType, DataType. Keep Serialize. Decoding goes through dol-wire.
Feature flags (all default-on for host, all default-off for embedded):

std, serde, geo, network, datetime, numeric, alloc, signing.
New: defmt (formats ErrorCode/Span for embedded logging).
lib/expr — recursion-free, arena-only, packed
PackedNode is the only node representation — 16 bytes, bytemuck::Pod, no enum discriminant blow-ups. Drop the current variant-style Node.
Arena is ExprArena<S: Storage<PackedNode>>. Host uses Vec, MCU uses heapless::Vec<\_, N>.
Interner is content-addressed with a 64-bit blake3 prefix → globally stable StrIds. (This replaces the per-instance sequential ids called out in the stored memory; verified the memory and intentionally invalidating it because stable ids are required for caching, signed manifests, and cross-process equality.)
All traversal is iterative (work-stack on Storage); fuel/depth from Budget. Stack overflow becomes impossible by construction.
No Deserialize. Construction is via Builder (in code) or Decoder (from bytes).
lib/schema and lib/model
Today dol-model lives under dol-core/crates/. Hoist it: lib/model becomes a sibling. lib/schema builds on lib/model. Both:

All types use Id<Tag> from core.
All builders take &mut Budget.
FieldType::Custom(&'static str) becomes FieldType::Custom(StrId) — solves the Serialize-only problem cleanly: now it round-trips through wire.
lib/ir
Three-address SSA-ish IR using Id<StmtTag>.
Validating constructor — every IrModule::new runs structural checks, returns Result<\_, IrError> with ErrorCodes.
Serialisation only via dol-wire.
lib/wire — the only border crossing
This is where the security work concentrates.

One wire format. postcard + a fixed-shape framing header { magic: [u8;4], version: u16, kind: u16, payload_len: u32, payload_hash: [u8;32], sig_len: u16, sig: [..] }.
Decoder<'a> is streaming, allocation-budgeted, and validating: enforces Limits, rejects oversize/cyclic/unknown-kind frames before allocation.
Frame<'a> is the zero-copy reader — borrows from input bytes, no allocation in the read path.
Signed manifests: every persisted artifact (schema, IR module, expression) is wrapped in Signed<T> carrying a Signature verified via core::sign::Verifier.
Fuzz targets in lib/wire/fuzz/ for every decoder, run in CI nightly.
lib/pipeline and lib/stream
Replace process-global extension registration (legacy) with the deterministic FNV-1a Symbol (per stored memory, already present — keep). Audit it's used everywhere — no inventory! crate, no linkme, no lazy_static.
Both crates take &mut Budget on every step/poll.
Backpressure model documented; bounded queues by default, heapless::spsc on embedded.
lib/query
Query IR generic over Storage.
Cost model takes Limits; planner refuses plans whose worst-case fuel exceeds budget.
lib/dol (umbrella)
Pure re-exports + a curated prelude. No logic. Compile-time check that no other crate depends on it.
tools/check and tools/fmt
check runs every validator in one pass and emits SARIF (good DX in IDEs).
fmt is deterministic, idempotent, and content-addressed (formatting is a function of the canonical form).
Both use the workspace's diagnostic types directly — no string parsing.
backends/\* (re-add when first one lands, per current Cargo.toml comment)
Trait KvStore / Catalog lives in lib/ir. Backends depend only on lib/ir (+ lib/wire for serialization). No backend depends on expr or pipeline.

DX — concrete improvements landed in the same cut
One dol::prelude import gets you 90% of public API.
A single Diagnostic type flows from parser → builder → ir → query → wire. IDEs see one shape.
Stable ErrorCodes mean docs.rs gets a permalink table that never moves.
xtask ci runs the exact same gates locally as in CI: fmt, clippy -D warnings, test --workspace, test --no-default-features, doc, deny check, udeps, miri test -p dol-core -p dol-expr -p dol-wire, cargo-fuzz run -- -max_total_time=60 smoke.
cargo dol new <crate> scaffold via xtask — sets up forbid(unsafe_code), lints, feature flags, README, dual-test (std + no_std). #[track_caller] on every fallible builder; errors point at user call sites, not into our internals.
Performance — concrete wins in the same cut
PackedNode (16 B) + content-addressed interning + iterative traversal ⇒ ~3-5x speedup on typed expression building, zero stack-overflow risk, and dedup across nodes for free.
Id<Tag>(NonMaxU32) ⇒ every Option<Id> is 4 bytes; cuts node size and improves cache density in arenas.
Storage<T> generic arenas ⇒ MCU build can be entirely on the stack.
Zero-copy Frame<'a> ⇒ no decode-time allocation for read-side workloads (introspection, planning, hashing).
blake3 Hasher32/Hasher128 ⇒ SIMD-accelerated hashing, used for interning, content addressing, and signing prep.
Security — concrete wins in the same cut
No Deserialize on any AST/IR ⇒ untrusted input cannot bypass invariants.
Limits + Budget ⇒ DoS-resistant by construction.
Iterative traversal ⇒ no stack-overflow attack surface.
forbid(unsafe_code) everywhere except core::raw ⇒ memory-safety surface auditable in one file.
Signed<T> envelope on every persisted artifact ⇒ supply-chain story end-to-end.
Continuous fuzzing of every decoder + property tests on every validator (proptest is already a dev-dep in core).
cargo-deny + cargo-audit + cargo-vet (or cargo-supply-chain) gates in CI.
RUSTFLAGS=-Zsanitizer=address job for the test suite on nightly.
Resource — concrete wins in the same cut
default-features = false produces a working build that links neither std nor alloc-heavy paths.
Granular features keep dropping Value/Literal variants when not needed (already true; preserved).
heapless paths replace Vec/BTreeMap/String with bounded equivalents on MCU.
defmt feature replaces Display formatters → ~10x smaller log code on Cortex-M.
All recursive traversals are iterative ⇒ thread/task stacks can be small (8 KB MCU tasks become viable).
Packed 16 B nodes ⇒ a 4 KB arena holds 256 expression nodes, plenty for IoT.
What we explicitly drop (and why)
Sequential per-instance StrId → replaced by content-addressed ids. The memory I have on this becomes obsolete; storing a corrected fact below.
serde::Deserialize for ExprArena/Value/Literal/FieldType/DataType → wire is the only entry.
FieldType::Custom(&'static str) → Custom(StrId).
Variant-discriminated Node enum in expr → PackedNode.
Recursive fold/walk/visit anywhere → iterative work-stack helpers in core.
Process-global extension registries if any survive → already moved to const-symbol per memory; audit and confirm.
unwrap/expect in non-test code → Result everywhere; lint enforces.
Bespoke per-crate id newtypes → Id<Tag> from core.
Multiple wire formats / fallback decoders → exactly one framed format.
String-based extension keys at runtime → Symbol (FNV-1a, const) only.
Execution order (single epic, no flag days because no users)
Land core scaffolding — policy, id, storage, diag::ErrorCode, sign, hash, raw (with audited unsafe), forbid(unsafe_code) everywhere else, drop Deserializes. Hoist dol-model to lib/model.
Rewrite lib/expr with PackedNode, content-addressed interner, iterative traversal, Storage-generic arena. Delete old Node enum.
Update lib/schema, lib/ir, lib/query to use core ids, limits, validating constructors.
Build out lib/wire as the sole codec; add fuzz harnesses; add Signed<T>.
Update lib/pipeline, lib/stream to thread Budget, use Symbol consistently, document backpressure.
Tighten tools/check and tools/fmt to consume new diagnostics, emit SARIF.
Add xtask ci super-task, wire all gates into .github/workflows/ci.yml.
Add MCU CI target (thumbv7em-none-eabihf, riscv32imac-unknown-none-elf) building lib/{core,expr,ir,wire} with default-features = false.
Tag 0.2.0 workspace-wide. Update CHANGELOG.md with one entry: "Foundational rewrite — see RFC."

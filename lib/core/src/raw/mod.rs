//! Audited POD wire types — the planned `unsafe` boundary for the
//! workspace.
//!
//! # Why this module exists
//!
//! The DOL workspace declares `unsafe_code = "forbid"` at the root and
//! `#![forbid(unsafe_code)]` on every crate. This module is reserved as
//! the single planned exception: a tightly scoped sandbox for plain-old-
//! data wire structs that need [`bytemuck`](https://docs.rs/bytemuck) /
//! `zerocopy`-style casts to cross a byte boundary without going through
//! a serialiser.
//!
//! # Audit boundary
//!
//! Everything declared here is subject to these invariants:
//!
//! 1. **POD only.** Every public type must be `#[repr(C)]` (or `#[repr(transparent)]`),
//!    have no padding, contain only primitive integer / float fields,
//!    and be `Pod + Zeroable` (verified by `bytemuck` derives).
//! 2. **No raw pointers, no `UnsafeCell`, no aliasing tricks.** Casts
//!    are limited to byte-slice ↔ POD-struct via `bytemuck::from_bytes`
//!    / `bytemuck::cast_slice`. Nothing else is permitted.
//! 3. **No allocation.** All operations are in-place on caller-supplied
//!    storage.
//! 4. **No `unsafe { … }` blocks** unless they are wrapping a `bytemuck`
//!    / `zerocopy` cast that the surrounding type's `Pod` derive proves
//!    sound; each such block carries a `// SAFETY:` comment pointing at
//!    the proof.
//!
//! See `lib/core/src/raw/README.md` for the full review checklist.
//!
//! # Current contents
//!
//! The module is intentionally empty for the 0.2.0 cut — no `unsafe`
//! code lives in the workspace today. POD wire types are added here as
//! the wire layer grows; when the first such type lands the surrounding
//! crate's lint level is downgraded from `forbid` to `deny` and this
//! module gets a scoped `#![allow(unsafe_code)]`.


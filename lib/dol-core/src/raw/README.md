# `dol-core::raw` — audited `unsafe` boundary

This module is the **only** place in the DOL workspace where `unsafe` code
is permitted. The workspace `Cargo.toml` declares `unsafe_code = "forbid"`
and every crate carries `#![forbid(unsafe_code)]`; this module opts back
in via a scoped `#![allow(unsafe_code)]` at the module root.

## Charter

`raw` exists for **plain-old-data wire structs** that need `bytemuck` /
`zerocopy`-style casts to cross a byte boundary without going through a
serialiser. Its primary consumers are:

- The wire layer (`dol-wire`) for fixed-width records.
- Backends that mmap on-disk segments and want zero-copy struct views.

If you find yourself wanting to put `unsafe` *anywhere else*, the answer
is almost certainly "use a safe abstraction". If you have a genuine need
that the rest of the workspace cannot express safely, raise a discussion
issue before adding to `raw/`.

## Invariants — non-negotiable

Every public item declared here must satisfy **all** of the following:

1. **POD layout.** `#[repr(C)]` or `#[repr(transparent)]`, no padding,
   only primitive integer / float fields. `bytemuck::Pod` and
   `bytemuck::Zeroable` derives are required.
2. **No raw pointers.** No `*const T`, `*mut T`, `NonNull<T>`,
   `UnsafeCell<T>`, or other aliasing tricks. Casts are limited to
   byte-slice ↔ POD-struct.
3. **No allocation.** All operations are in-place on caller-supplied
   storage.
4. **`SAFETY:` comments.** Every `unsafe { … }` block is preceded by a
   `// SAFETY:` comment naming the invariant that makes the cast sound
   (typically: "`T: Pod` ⇒ every bit pattern is a valid `T`, and the
   slice length and alignment are checked above").

## Review checklist

Reviewers of any change touching this module should verify:

- [ ] No new types added without `Pod + Zeroable` derives.
- [ ] No new `unsafe` blocks without a matching `SAFETY:` comment.
- [ ] No raw pointers introduced.
- [ ] No allocation paths introduced.
- [ ] No new public function takes user-controlled length/offset values
      without bounds-checking them before the cast.

Failing any of the above is a hard block; this module's correctness is
load-bearing for the workspace's `forbid(unsafe_code)` posture.

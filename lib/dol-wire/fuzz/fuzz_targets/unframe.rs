//! Fuzz target: `dol_wire::unframe`.
//!
//! Contract verified: for any byte slice, `unframe` returns either
//! `Ok(_)` or a structured `WireError` — never panics, never aborts.
//! This is the lightest-weight DOL decoder reachable from untrusted
//! bytes, so it is the natural smoke entry point per
//! `docs/v2_plan.md` §57.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = dol_wire::unframe(data);
});

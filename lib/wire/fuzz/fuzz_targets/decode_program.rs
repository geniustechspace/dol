//! Fuzz target: `dol_wire::program::decode`.
//!
//! Contract verified: arbitrary bytes never panic the budget-threaded
//! `Program` decoder (`Decode`-based, no `serde::Deserialize`). Any
//! malformed input must surface as a clean `DecodeError`. Per
//! `docs/v2_plan.md` §57: "Fuzz targets in `lib/wire/fuzz/` for every
//! decoder, run in CI nightly."

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = dol_wire::program::decode(data);
});

//! # `dol-wire` — Wire Protocol
//!
//! M0 scaffold. The real content (envelope, `Decode` trait, postcard/JSON
//! adapters, fuzz harnesses) lands in M4 per `dol-rewrite-plan-v2.md` §9.
//!
//! The previous v0.1 implementation is preserved on disk in `_legacy_src/`
//! and `_legacy_tests/` as a reference to mine from.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

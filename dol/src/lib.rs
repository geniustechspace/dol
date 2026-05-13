//! # DOL — umbrella facade
//!
//! M0 scaffold per `dol-rewrite-plan-v2.md` §3.6 / §11.1. Only `dol-core`
//! is currently wired through; the `cas`, `ir`, `wire`, `query`, `check`,
//! and `fmt` features pull empty scaffold crates that gain content during
//! M1–M6.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use dol_core;

#[cfg(feature = "cas")]
pub use dol_cas;

#[cfg(feature = "ir")]
pub use dol_ir;

#[cfg(feature = "wire")]
pub use dol_wire;

#[cfg(feature = "query")]
pub use dol_query;

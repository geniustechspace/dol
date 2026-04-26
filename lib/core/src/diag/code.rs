//! Stable diagnostic codes.
//!
//! A [`Code`] is a `&'static str` newtype so that built-in codes are
//! zero-cost and downstream crates can mint their own without coordinating
//! with `dol-core`. Built-in codes follow the pattern `DOLNNNN`.
//!
//! Adding a new code requires (a) a constant here, (b) an entry in the docs.

#![allow(missing_docs)]

/// Stable diagnostic identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Code(pub &'static str);

impl Code {
    /// Borrow the underlying static string.
    #[inline]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl core::fmt::Display for Code {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0)
    }
}

// ─── Catalogue ────────────────────────────────────────────────────────────────
// Codes are grouped by layer:
//
//   DOL0xxx — generic / cross-cutting
//   DOL1xxx — types
//   DOL2xxx — expr
//   DOL3xxx — schema
//   DOL4xxx — ir / capabilities
//   DOL5xxx — wire
//   DOL6xxx — pipeline / stream
//   DOL7xxx — check (lints)

// Generic
pub const INTERNAL_ERROR: Code = Code("DOL0001");
pub const NOT_IMPLEMENTED: Code = Code("DOL0002");

// Types
pub const TYPE_MISMATCH: Code = Code("DOL1001");
pub const VALUE_OUT_OF_RANGE: Code = Code("DOL1002");
pub const UNKNOWN_TYPE: Code = Code("DOL1003");

// Expr
pub const UNDEFINED_FIELD: Code = Code("DOL2001");
pub const ARITY_MISMATCH: Code = Code("DOL2002");
pub const UNKNOWN_FUNCTION: Code = Code("DOL2003");

// Schema
pub const DUPLICATE_ENTITY: Code = Code("DOL3001");
pub const DUPLICATE_FIELD: Code = Code("DOL3002");
pub const DANGLING_RELATION: Code = Code("DOL3003");
pub const INVALID_CONSTRAINT: Code = Code("DOL3004");

// IR / capabilities
pub const UNSUPPORTED_BY_BACKEND: Code = Code("DOL4001");
pub const MISSING_CAPABILITY: Code = Code("DOL4002");
pub const UNKNOWN_EXTENSION: Code = Code("DOL4003");

// Wire
pub const WIRE_VERSION_MISMATCH: Code = Code("DOL5001");
pub const WIRE_DECODE_FAILURE: Code = Code("DOL5002");

// Pipeline / stream
pub const PIPELINE_CYCLE: Code = Code("DOL6001");
pub const SCHEMA_INFERENCE: Code = Code("DOL6002");
pub const WATERMARK_INVALID: Code = Code("DOL6003");

// Check / lint
pub const SUSPICIOUS_PREDICATE: Code = Code("DOL7001");
pub const REDUNDANT_CAST: Code = Code("DOL7002");

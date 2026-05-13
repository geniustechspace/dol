//! Stable diagnostic error codes.
//!
//! An [`ErrorCode`] is a 16-bit numeric identifier, layered as
//! `layer * 1000 + serial`. It is `Copy`, `no_std`, and `no_alloc` — the
//! type itself imposes no runtime cost and links cleanly into a Cortex-M
//! firmware image. Display rendering writes the canonical `DOLNNNN`
//! string into a [`core::fmt::Formatter`] without allocating.
//!
//! ## Layering
//!
//! Codes are grouped by layer. The "layer" is the leading digit of the
//! 4-digit serial form (`DOLLNNN`) and lets external tooling reason
//! about which subsystem emitted the diagnostic without parsing.
//!
//! | layer | range          | subsystem                  |
//! | ----- | -------------- | -------------------------- |
//! | 0     | `   1..1000`   | generic / cross-cutting    |
//! | 1     | `1000..2000`   | types                      |
//! | 2     | `2000..3000`   | expr                       |
//! | 3     | `3000..4000`   | schema                     |
//! | 4     | `4000..5000`   | ir / capabilities          |
//! | 5     | `5000..6000`   | wire                       |
//! | 6     | `6000..7000`   | pipeline / stream          |
//! | 7     | `7000..8000`   | check (lints)              |
//!
//! New codes are added by appending a `pub const FOO: ErrorCode = ErrorCode(N);`
//! constant in the relevant section below and documenting it in the
//! diagnostics index.

#![allow(missing_docs)]

/// Stable 16-bit diagnostic identifier.
///
/// The wire form is the raw `u16`; the human form is the
/// [`core::fmt::Display`] rendering `DOL{NNNN:04}`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ErrorCode(pub u16);

impl ErrorCode {
    /// The numeric identifier, exactly as emitted on the wire.
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// The layer this code belongs to (`0..=7` for built-in codes).
    /// Computed as `code / 1000`.
    #[inline]
    pub const fn layer(self) -> u16 {
        // `self.0 / 1000` cannot overflow: `u16::MAX / 1000 == 65`.
        #[allow(clippy::arithmetic_side_effects)]
        let l = self.0 / 1000;
        l
    }

    /// The serial within the layer (`code % 1000`).
    #[inline]
    pub const fn serial(self) -> u16 {
        // `self.0 % 1000` is always in `0..1000`; never overflows.
        #[allow(clippy::arithmetic_side_effects)]
        let s = self.0 % 1000;
        s
    }
}

impl core::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DOL{:04}", self.0)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for ErrorCode {
    fn format(&self, fmt: defmt::Formatter<'_>) {
        defmt::write!(fmt, "DOL{=u16:04}", self.0);
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
pub const INTERNAL_ERROR: ErrorCode = ErrorCode(1);
pub const NOT_IMPLEMENTED: ErrorCode = ErrorCode(2);

// Types
pub const TYPE_MISMATCH: ErrorCode = ErrorCode(1001);
pub const VALUE_OUT_OF_RANGE: ErrorCode = ErrorCode(1002);
pub const UNKNOWN_TYPE: ErrorCode = ErrorCode(1003);

// Expr
pub const UNDEFINED_FIELD: ErrorCode = ErrorCode(2001);
pub const ARITY_MISMATCH: ErrorCode = ErrorCode(2002);
pub const UNKNOWN_FUNCTION: ErrorCode = ErrorCode(2003);

// Schema
pub const DUPLICATE_ENTITY: ErrorCode = ErrorCode(3001);
pub const DUPLICATE_FIELD: ErrorCode = ErrorCode(3002);
pub const DANGLING_RELATION: ErrorCode = ErrorCode(3003);
pub const INVALID_CONSTRAINT: ErrorCode = ErrorCode(3004);

// IR / capabilities
pub const UNSUPPORTED_BY_BACKEND: ErrorCode = ErrorCode(4001);
pub const MISSING_CAPABILITY: ErrorCode = ErrorCode(4002);
pub const UNKNOWN_EXTENSION: ErrorCode = ErrorCode(4003);

// Wire
pub const WIRE_VERSION_MISMATCH: ErrorCode = ErrorCode(5001);
pub const WIRE_DECODE_FAILURE: ErrorCode = ErrorCode(5002);

// Pipeline / stream
pub const PIPELINE_CYCLE: ErrorCode = ErrorCode(6001);
pub const SCHEMA_INFERENCE: ErrorCode = ErrorCode(6002);
pub const WATERMARK_INVALID: ErrorCode = ErrorCode(6003);

// Check / lint
pub const SUSPICIOUS_PREDICATE: ErrorCode = ErrorCode(7001);
pub const REDUNDANT_CAST: ErrorCode = ErrorCode(7002);

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;

    #[test]
    fn display_writes_canonical_form() {
        assert_eq!(alloc::format!("{}", INTERNAL_ERROR), "DOL0001");
        assert_eq!(alloc::format!("{}", TYPE_MISMATCH), "DOL1001");
        assert_eq!(alloc::format!("{}", REDUNDANT_CAST), "DOL7002");
    }

    #[test]
    fn layer_and_serial() {
        assert_eq!(TYPE_MISMATCH.layer(), 1);
        assert_eq!(TYPE_MISMATCH.serial(), 1);
        assert_eq!(WIRE_DECODE_FAILURE.layer(), 5);
        assert_eq!(WIRE_DECODE_FAILURE.serial(), 2);
        assert_eq!(REDUNDANT_CAST.layer(), 7);
        assert_eq!(REDUNDANT_CAST.serial(), 2);
    }

    #[test]
    fn niche_size() {
        // `ErrorCode` is `repr(transparent)` over `u16` — it must be 2 B
        // and `Option<ErrorCode>` must remain 4 B (no smaller niche available
        // since every `u16` value is a valid code).
        assert_eq!(core::mem::size_of::<ErrorCode>(), 2);
    }
}

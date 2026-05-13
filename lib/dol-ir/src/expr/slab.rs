//! `OperandSlab` — side pool for variadic-arity `ExprNode` operands.
//!
//! Per `dol-rewrite-plan-v2.md` §3.3 ("side pools") and §8.1 (the
//! 16-byte `ExprNode` budget rule on line 1080–1082 + the risk-register
//! line 1994 "any new opcode needing >3 u32 operands gets a side pool").
//!
//! ## Role in the lowering pipeline
//!
//! [`ExprNode`](super::node::ExprNode) is fixed at 16 bytes:
//! `op:u8 | flags:u8 | aux:u16 | a:u32 | b:u32 | c:u32`. Operations
//! with **variadic** arity (the tree-DSL variants `Expr::Seq`,
//! `Expr::Map`, `Expr::Call`, `Expr::Match`) cannot inline their
//! operand list. The recursive lowerer in §8.4 will park each
//! variable-length operand sequence here and store a `(offset, len)`
//! handle in two of the node's `u32` slots.
//!
//! ## What this slice ships (M3c-δ₂b prereq #3)
//!
//! - The flat `u32` storage + an [`OperandSpan`] handle paired
//!   `(offset, len)` accessor. Operands are uninterpreted `u32`s; the
//!   surrounding [`ExprNode`](super::node::ExprNode) opcode determines
//!   how each slot is read (NodeId, LiteralId, FieldId, paired StrId+
//!   NodeId for `Map`, etc.).
//! - Append-only [`OperandSlab::push_span`]: copies a `&[u32]` to the
//!   tail and returns the [`OperandSpan`] that addresses it.
//! - Zero-copy [`OperandSlab::get`] returning `&[u32]`.
//! - **No interior dedup.** A `Seq([1, 2])` lowered twice produces two
//!   distinct spans. Span-level structural dedup belongs in the
//!   surrounding `ExprArena::intern_node` (which already keys on the
//!   16 raw bytes of `ExprNode` — two `Seq` nodes whose spans hold
//!   identical contents at different offsets will *not* dedup; that
//!   is a deliberate trade-off for this slice, mirroring the
//!   push-only [`crate::expr::literals::LiteralPool`] policy).
//!
//! ## Capacity bound
//!
//! Span offset and length both occupy a single `u32` (so they can fit
//! in `ExprNode::b` / `ExprNode::c`). The slab therefore caps at
//! [`u32::MAX`] total operand slots — overflow returns
//! [`OperandSlabError::CapacityExceeded`].

extern crate alloc;

use alloc::vec::Vec;

/// Handle into an [`OperandSlab`]: a `(offset, len)` pair into the
/// flat `u32` storage.
///
/// Encoded as two `u32`s so it fits in the free `b` / `c` slots of an
/// [`ExprNode`](super::node::ExprNode). The handle is **opaque** —
/// the inner numbers carry no other meaning. A handle from one slab
/// must never be used against another.
///
/// Empty spans are represented as `OperandSpan { offset: 0, len: 0 }`
/// and resolve to `&[]` via [`OperandSlab::get`] regardless of the
/// slab's contents — they require no storage allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperandSpan {
    offset: u32,
    len: u32,
}

impl OperandSpan {
    /// The empty span (`offset = 0`, `len = 0`). Resolves to `&[]`
    /// in every [`OperandSlab`].
    pub const EMPTY: Self = Self { offset: 0, len: 0 };

    /// Construct a span from raw `(offset, len)` pair. Used by
    /// [`ExprNode`](super::node::ExprNode) accessors that re-hydrate
    /// the handle from the node's `b`/`c` slots; callers outside the
    /// node module should obtain a span via
    /// [`OperandSlab::push_span`] instead.
    #[must_use]
    #[inline]
    pub const fn from_parts(offset: u32, len: u32) -> Self {
        Self { offset, len }
    }

    /// The offset into the slab's `u32` storage.
    #[must_use]
    #[inline]
    pub const fn offset(self) -> u32 {
        self.offset
    }

    /// The number of `u32` slots covered by this span.
    #[must_use]
    #[inline]
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Whether the span has zero length.
    #[must_use]
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Errors returned by [`OperandSlab::push_span`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperandSlabError {
    /// Appending the requested slice would push the slab beyond
    /// [`u32::MAX`] total slots. Mirrors the
    /// [`crate::expr::literals::LiteralPoolError::CapacityExceeded`]
    /// and [`crate::expr::funcs::FuncRegistryError::CapacityExceeded`]
    /// shapes so the recursive lowerer can compose all three with `?`.
    CapacityExceeded,
}

impl core::fmt::Display for OperandSlabError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("operand slab: capacity exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OperandSlabError {}

/// Flat side pool of variadic `ExprNode` operands.
///
/// All operands are stored back-to-back in a single `Vec<u32>`.
/// Spans are addressed by `(offset, len)` and are stable for the
/// life of the slab (the storage is append-only — no removal, no
/// shuffling).
///
/// `Send + Sync`. `Default` constructs an empty slab.
#[derive(Debug, Clone, Default)]
pub struct OperandSlab {
    data: Vec<u32>,
}

impl OperandSlab {
    /// Construct an empty slab.
    #[must_use]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Pre-allocate room for `cap` total operand slots.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: Vec::with_capacity(cap),
        }
    }

    /// Append `operands` to the slab and return the [`OperandSpan`]
    /// that addresses them.
    ///
    /// Empty input yields [`OperandSpan::EMPTY`] without growing
    /// storage — empty spans are a sentinel, not a slab address.
    ///
    /// # Errors
    ///
    /// Returns [`OperandSlabError::CapacityExceeded`] when the
    /// resulting slab would exceed [`u32::MAX`] slots (the upper
    /// bound of `OperandSpan::offset` and `OperandSpan::len`).
    pub fn push_span(&mut self, operands: &[u32]) -> Result<OperandSpan, OperandSlabError> {
        if operands.is_empty() {
            return Ok(OperandSpan::EMPTY);
        }
        // Reject individually-overflowing lengths before any
        // mutation, so a failed call leaves the slab unchanged.
        let len = u32::try_from(operands.len()).map_err(|_| OperandSlabError::CapacityExceeded)?;
        let offset =
            u32::try_from(self.data.len()).map_err(|_| OperandSlabError::CapacityExceeded)?;
        // Guard against `offset + len > u32::MAX` so spans are
        // always representable. `checked_add` keeps us inside the
        // `clippy::arithmetic_side_effects` lint that the workspace
        // enables for `dol-ir`.
        let end = u64::from(offset).checked_add(u64::from(len));
        match end {
            Some(end) if end <= u64::from(u32::MAX) => {}
            _ => return Err(OperandSlabError::CapacityExceeded),
        }
        self.data.extend_from_slice(operands);
        Ok(OperandSpan { offset, len })
    }

    /// Borrow the operands addressed by `span`.
    ///
    /// Returns `None` when the span's `offset..offset+len` range
    /// falls outside the slab's current storage — that is, the span
    /// came from a different slab (or was constructed by some other
    /// route).
    #[must_use]
    pub fn get(&self, span: OperandSpan) -> Option<&[u32]> {
        let start = span.offset as usize;
        let end = start.checked_add(span.len as usize)?;
        self.data.get(start..end)
    }

    /// Total number of `u32` slots currently in the slab.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the slab has any operand slots in use.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_slab_round_trip() {
        let slab = OperandSlab::new();
        assert!(slab.is_empty());
        assert_eq!(slab.len(), 0);
        assert_eq!(slab.get(OperandSpan::EMPTY), Some(&[][..]));
    }

    #[test]
    fn push_span_returns_correct_view() {
        let mut slab = OperandSlab::new();
        let span = slab.push_span(&[10, 20, 30]).unwrap();
        assert_eq!(span.offset(), 0);
        assert_eq!(span.len(), 3);
        assert_eq!(slab.get(span), Some(&[10, 20, 30][..]));
        assert_eq!(slab.len(), 3);
    }

    #[test]
    fn multiple_spans_are_independent() {
        let mut slab = OperandSlab::new();
        let s1 = slab.push_span(&[1, 2]).unwrap();
        let s2 = slab.push_span(&[7, 8, 9]).unwrap();
        let s3 = slab.push_span(&[42]).unwrap();
        assert_eq!(slab.get(s1), Some(&[1, 2][..]));
        assert_eq!(slab.get(s2), Some(&[7, 8, 9][..]));
        assert_eq!(slab.get(s3), Some(&[42][..]));
        assert_eq!(slab.len(), 6);
    }

    #[test]
    fn push_span_does_not_dedup_identical_runs() {
        // Mirrors LiteralPool's push-only policy. Slab-level
        // structural dedup is a future slice; for now identical
        // operand lists produce distinct OperandSpans.
        let mut slab = OperandSlab::new();
        let a = slab.push_span(&[1, 2, 3]).unwrap();
        let b = slab.push_span(&[1, 2, 3]).unwrap();
        assert_ne!(a, b);
        assert_eq!(slab.get(a), slab.get(b));
        assert_eq!(slab.len(), 6);
    }

    #[test]
    fn empty_input_returns_sentinel_without_growth() {
        let mut slab = OperandSlab::new();
        let _ = slab.push_span(&[1, 2]).unwrap();
        let len_before = slab.len();
        let span = slab.push_span(&[]).unwrap();
        assert_eq!(span, OperandSpan::EMPTY);
        assert_eq!(slab.len(), len_before);
        assert_eq!(slab.get(span), Some(&[][..]));
    }

    #[test]
    fn empty_sentinel_resolves_in_any_slab() {
        let slab = OperandSlab::new();
        assert_eq!(slab.get(OperandSpan::EMPTY), Some(&[][..]));
        // And in a non-empty slab too.
        let mut slab2 = OperandSlab::new();
        slab2.push_span(&[99]).unwrap();
        assert_eq!(slab2.get(OperandSpan::EMPTY), Some(&[][..]));
    }

    #[test]
    fn get_with_unknown_span_returns_none() {
        let slab = OperandSlab::new();
        let bogus = OperandSpan {
            offset: 5,
            len: 2,
        };
        assert!(slab.get(bogus).is_none());
    }

    #[test]
    fn span_addressing_is_stable_across_pushes() {
        // The append-only contract: pushing more data must not
        // shift existing spans. (Vec growth re-allocates the buffer
        // address but indices stay valid.)
        let mut slab = OperandSlab::new();
        let first = slab.push_span(&[10, 11]).unwrap();
        for v in 0..256u32 {
            slab.push_span(&[v, v + 1, v + 2]).unwrap();
        }
        assert_eq!(slab.get(first), Some(&[10, 11][..]));
    }

    #[test]
    fn with_capacity_does_not_change_addressing() {
        let mut slab = OperandSlab::with_capacity(64);
        let s = slab.push_span(&[1, 2, 3]).unwrap();
        assert_eq!(s.offset(), 0);
        assert_eq!(s.len(), 3);
        assert_eq!(slab.get(s), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn span_helpers() {
        let s = OperandSpan {
            offset: 12,
            len: 5,
        };
        assert_eq!(s.offset(), 12);
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());
        assert!(OperandSpan::EMPTY.is_empty());
    }

    #[test]
    fn error_display_is_human_readable() {
        let s = alloc::format!("{}", OperandSlabError::CapacityExceeded);
        assert!(s.contains("capacity"));
    }
}

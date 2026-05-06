//! # `span` — source spans for diagnostics
//!
//! A [`Span`] is a compact `(FileId, start, length)` triple packed into 8
//! bytes (`u16 + u32 + u24`). Spans are kept out of the hot AST/IR path; they
//! are stored in a side [`SpanTable`] keyed by the AST node's id.

use alloc::vec::Vec;

/// Identifier for a source file in the [`SpanTable`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileId(pub u16);

impl FileId {
    /// Sentinel "no file".
    pub const NONE: FileId = FileId(u16::MAX);
}

/// A `(file, start, length)` span. Length is capped at 16 MiB which is
/// enough for any source line and most files.
///
/// Encoded as `u64`:
/// - bits  0..16 : `file: u16`
/// - bits 16..40 : `start: u24`
/// - bits 40..64 : `length: u24`
///
/// This keeps the type 8 bytes and trivially `Copy`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span(u64);

impl Span {
    /// Sentinel "no span".
    pub const NONE: Span = Span(0);

    /// Build a span. `start` and `length` are clamped to 24 bits.
    #[inline]
    pub const fn new(file: FileId, start: u32, length: u32) -> Self {
        let s = (start as u64) & 0x00FF_FFFF;
        let l = (length as u64) & 0x00FF_FFFF;
        Self((file.0 as u64) | (s << 16) | (l << 40))
    }

    /// File this span lives in.
    #[inline]
    pub const fn file(self) -> FileId {
        FileId(self.0 as u16)
    }

    /// Byte offset (within the file) at which this span starts.
    #[inline]
    pub const fn start(self) -> u32 {
        ((self.0 >> 16) & 0x00FF_FFFF) as u32
    }

    /// Length of the span in bytes.
    #[inline]
    pub const fn length(self) -> u32 {
        ((self.0 >> 40) & 0x00FF_FFFF) as u32
    }

    /// Byte offset one past the end of the span.
    #[inline]
    pub const fn end(self) -> u32 {
        self.start() + self.length()
    }

    /// `true` if this is the [`Self::NONE`] sentinel.
    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// Reconstruct a [`Span`] from its packed `u64` representation.
    ///
    /// This is the inverse of [`Span::to_raw_u64`] and exists so that
    /// codecs (e.g. [`dol-wire`](https://docs.rs/dol-wire)) can rebuild a
    /// `Span` from bytes without going through the field-decomposed
    /// constructor. Any 64-bit value is accepted; the `(start, length)`
    /// halves are masked to 24 bits on read by [`Span::start`] and
    /// [`Span::length`], so an arbitrary `u64` is at worst ill-formed,
    /// never undefined.
    #[inline]
    pub const fn from_raw_u64(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the packed `u64` representation of this [`Span`].
    ///
    /// Pairs with [`Span::from_raw_u64`]. The returned bits encode
    /// `file` (low 16 bits), `start` (next 24 bits), and `length`
    /// (top 24 bits) per the layout documented on [`Span`].
    #[inline]
    pub const fn to_raw_u64(self) -> u64 {
        self.0
    }
}

/// Side table mapping AST/IR node indices to [`Span`]s.
///
/// The table is grow-only and unsorted; callers either index in parallel with
/// the AST node arena or look up by raw index. Spans default to [`Span::NONE`].
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanTable {
    spans: Vec<Span>,
}

impl SpanTable {
    /// Create an empty table.
    #[inline]
    pub const fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Reserve room for `cap` spans.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            spans: Vec::with_capacity(cap),
        }
    }

    /// Append a span; the index is the table's previous length.
    pub fn push(&mut self, span: Span) -> u32 {
        let id = self.spans.len() as u32;
        self.spans.push(span);
        id
    }

    /// Look up by index. Returns `Span::NONE` for out-of-bounds ids.
    pub fn get(&self, index: u32) -> Span {
        self.spans
            .get(index as usize)
            .copied()
            .unwrap_or(Span::NONE)
    }

    /// Number of spans in the table.
    #[inline]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// `true` if the table holds no spans.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn span_is_8_bytes() {
        assert_eq!(size_of::<Span>(), 8);
        assert_eq!(size_of::<FileId>(), 2);
    }

    #[test]
    fn span_roundtrip() {
        let s = Span::new(FileId(7), 100, 25);
        assert_eq!(s.file(), FileId(7));
        assert_eq!(s.start(), 100);
        assert_eq!(s.length(), 25);
        assert_eq!(s.end(), 125);
    }

    #[test]
    fn span_table_basic() {
        let mut t = SpanTable::new();
        let i = t.push(Span::new(FileId(0), 0, 10));
        assert_eq!(t.get(i).length(), 10);
        assert!(t.get(999).is_none());
    }

    #[test]
    fn span_raw_u64_round_trip() {
        // Every accessor must be preserved across `to_raw_u64` /
        // `from_raw_u64` so that wire codecs can round-trip a Span
        // through its packed byte form.
        let s = Span::new(FileId(42), 1234, 5678);
        let raw = s.to_raw_u64();
        let back = Span::from_raw_u64(raw);
        assert_eq!(back.file(), FileId(42));
        assert_eq!(back.start(), 1234);
        assert_eq!(back.length(), 5678);
        // NONE sentinel preserved.
        assert!(Span::from_raw_u64(Span::NONE.to_raw_u64()).is_none());
    }
}

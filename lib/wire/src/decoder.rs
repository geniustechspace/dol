//! Validating, budget-aware wire decoder — the v2 single wire-in entry point.
//!
//! `dol-wire::decoder` defines [`Decode`], the trait every in-memory IR / AST
//! type implements when v2 is fully cut over. Unlike `serde::Deserialize`,
//! every [`Decode`] impl:
//!
//! - takes a [`&mut Budget`](dol_core::policy::Budget) so recursive payloads
//!   are bounded at the wire layer (a malicious peer cannot blow the call
//!   stack with a million-deep `Vec<Vec<…>>`),
//! - validates discriminants, ranges, and length-prefixes explicitly
//!   (`Decode::decode` returns [`DecodeError`] rather than panicking on
//!   malformed input), and
//! - shares postcard's byte format with the existing `encode_postcard` helper
//!   for postcard-compatible types; types that require stable discriminants
//!   (see [`crate::decode_core`]) use a custom but stable encoding.
//!
//! # Wire format (postcard-compatible)
//!
//! `Decode` reads the same bytes that `postcard::to_allocvec(&value)` would
//! emit for the equivalent `Serialize` derivation. The supported leaf
//! encodings are:
//!
//! | type            | encoding                                        |
//! | --------------- | ----------------------------------------------- |
//! | `bool`          | one byte; `0 = false`, `1 = true`               |
//! | `u8` / `i8`     | one byte raw                                    |
//! | `u16/u32/u64`   | unsigned varint, little-endian shifts of 7 bits |
//! | `i16/i32/i64`   | zig-zag varint                                  |
//! | `str`           | varint length + UTF-8 payload                   |
//! | `&[u8]`         | varint length + raw bytes                       |
//! | `Option<T>`     | one byte (`0 = None`, `1 = Some`) + payload     |
//! | `Vec<T>` / seq  | varint length + payload                         |
//! | enum            | varint discriminant + payload                   |
//! | struct          | fields concatenated in declaration order        |
//!
//! Only varint widths up to 5 bytes (`u32`) and 10 bytes (`u64`) are accepted;
//! a longer varint is a [`DecodeError::LengthOverflow`].
//!
//! # Cut-over status — v2 complete
//!
//! All `dol-core` leaf types have `Decode` impls in [`crate::decode_core`]:
//! primitives, datetime, numeric, geo, network, `DataType`, `StructField`,
//! `Value`, `ValueRange`, `Literal<'static>`, and `LiteralRange<'static>`.
//! The `decode_core_roundtrip` integration test asserts byte-for-byte parity
//! with `postcard::to_allocvec(&v)` for postcard-compatible types and
//! self-roundtrip for types with stable custom discriminants.
//!
//! # Example
//!
//! ```
//! use dol_core::policy::{Budget, Limits};
//! use dol_wire::decoder::{Decode, Reader};
//!
//! // A round-trippable plain-old-data wrapper.
//! let bytes = [
//!     0x05, b'h', b'e', b'l', b'l', b'o', // varint(5) "hello"
//!     0x01,                                  // Some
//!     0x2a,                                  // varint(42)
//! ];
//! let mut reader = Reader::new(&bytes);
//! let mut budget = Budget::new(Limits::host());
//!
//! let s: String = Decode::decode(&mut reader, &mut budget).unwrap();
//! let n: Option<u32> = Decode::decode(&mut reader, &mut budget).unwrap();
//! assert_eq!(s, "hello");
//! assert_eq!(n, Some(42));
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use dol_core::policy::{Budget, BudgetError};

// ─── Cursor ──────────────────────────────────────────────────────────────────

/// A byte cursor backing every [`Decode`] impl.
///
/// `Reader` owns a `&[u8]` slice and a position. It exposes only
/// bounds-checked reads — no `unwrap`, no panics on truncated input. All
/// errors funnel through [`DecodeError`].
#[derive(Debug)]
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Wrap `bytes` in a fresh cursor positioned at byte 0.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { buf: bytes, pos: 0 }
    }

    /// How many bytes remain to be consumed.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Whether every byte of the backing slice has been consumed. `Decode`
    /// users should assert this after reading a top-level message to detect
    /// trailing-byte attacks.
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Read exactly one byte. Returns [`DecodeError::Eof`] on truncation.
    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        let byte = *self
            .buf
            .get(self.pos)
            .ok_or(DecodeError::Eof { needed: 1, had: 0 })?;
        // Cursor advance is bounded by buf.len() and cannot wrap on any
        // realistic platform where buf.len() ≤ usize::MAX.
        self.pos = self.pos.saturating_add(1);
        Ok(byte)
    }

    /// Borrow exactly `n` bytes from the cursor and advance.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::LengthOverflow)?;
        let slice = self.buf.get(self.pos..end).ok_or(DecodeError::Eof {
            needed: n,
            had: self.remaining(),
        })?;
        self.pos = end;
        Ok(slice)
    }

    /// Read a postcard-style unsigned varint, max 5 bytes (`u32`).
    pub fn read_varint_u32(&mut self) -> Result<u32, DecodeError> {
        let mut result: u32 = 0;
        let mut shift: u32 = 0;
        for _ in 0..5 {
            let byte = self.read_u8()?;
            let chunk = (byte & 0x7F) as u32;
            // The shift is always < 35 because we cap at 5 iterations.
            result |= chunk
                .checked_shl(shift)
                .ok_or(DecodeError::LengthOverflow)?;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift = shift.saturating_add(7);
        }
        Err(DecodeError::LengthOverflow)
    }

    /// Read a postcard-style unsigned varint, max 10 bytes (`u64`).
    pub fn read_varint_u64(&mut self) -> Result<u64, DecodeError> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        for _ in 0..10 {
            let byte = self.read_u8()?;
            let chunk = (byte & 0x7F) as u64;
            result |= chunk
                .checked_shl(shift)
                .ok_or(DecodeError::LengthOverflow)?;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift = shift.saturating_add(7);
        }
        Err(DecodeError::LengthOverflow)
    }

    /// Read a postcard-style unsigned varint, max 19 bytes (`u128`).
    pub fn read_varint_u128(&mut self) -> Result<u128, DecodeError> {
        let mut result: u128 = 0;
        let mut shift: u32 = 0;
        for _ in 0..19 {
            let byte = self.read_u8()?;
            let chunk = (byte & 0x7F) as u128;
            result |= chunk
                .checked_shl(shift)
                .ok_or(DecodeError::LengthOverflow)?;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift = shift.saturating_add(7);
        }
        Err(DecodeError::LengthOverflow)
    }

    /// Read `N` raw bytes as a fixed-length array. Postcard encodes
    /// `[u8; N]` (and any fixed-array `[T; N]`) without a length prefix —
    /// `N` is part of the type contract, not the wire.
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let slice = self.read_bytes(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    /// Read a varint length and use it to slice the next `len` bytes,
    /// charging the budget for the implied allocation. The returned slice
    /// borrows from the underlying buffer.
    pub fn read_seq_bytes(
        &mut self,
        budget: &mut Budget,
        max: usize,
    ) -> Result<&'a [u8], DecodeError> {
        let len = self.read_varint_u32()? as usize;
        if len > max {
            return Err(DecodeError::LengthOverflow);
        }
        // Charge one budget unit per length-prefixed read so a 4 G length
        // cannot drain host memory before the bounds check fires below.
        budget.tick(1)?;
        self.read_bytes(len)
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Reasons a [`Decode`] impl may reject its input.
#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
    /// Reader hit the end of the buffer mid-read.
    Eof {
        /// Bytes the impl wanted.
        needed: usize,
        /// Bytes that remained.
        had: usize,
    },
    /// Discriminant value did not match any variant of the target enum.
    InvalidVariant {
        /// Type whose discriminant was being decoded.
        type_name: &'static str,
        /// Out-of-range discriminant value seen on the wire.
        seen: u32,
    },
    /// Encoded length exceeded what the receiving impl is willing to allocate.
    LengthOverflow,
    /// Byte sequence claimed to be UTF-8 was not.
    Utf8,
    /// Recursive [`Decode`] call exhausted the [`Budget`].
    Budget(BudgetError),
    /// Custom payload-specific failure.
    Custom(&'static str),
}

impl From<BudgetError> for DecodeError {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof { needed, had } => write!(
                f,
                "dol-wire decode: end of input (needed {needed} bytes, had {had})"
            ),
            Self::InvalidVariant { type_name, seen } => write!(
                f,
                "dol-wire decode: invalid discriminant {seen} for {type_name}"
            ),
            Self::LengthOverflow => f.write_str("dol-wire decode: length-prefix overflow"),
            Self::Utf8 => f.write_str("dol-wire decode: invalid UTF-8 in str payload"),
            Self::Budget(e) => write!(f, "dol-wire decode: budget exhausted: {e}"),
            Self::Custom(s) => write!(f, "dol-wire decode: {s}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

// ─── Trait + impls for the primitive leaves ─────────────────────────────────

/// Validating, budget-aware byte-slice reader for an in-memory IR / AST type.
///
/// See the module docs for the wire format and the cut-over status of v2.
pub trait Decode: Sized {
    /// Read `Self` from `reader`, charging recursion against `budget`.
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError>;
}

impl Decode for u8 {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        reader.read_u8()
    }
}

impl Decode for i8 {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        Ok(reader.read_u8()? as i8)
    }
}

impl Decode for bool {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "bool",
                seen: seen as u32,
            }),
        }
    }
}

macro_rules! impl_decode_uvarint {
    ($($t:ty => $reader:ident),* $(,)?) => {
        $(
            impl Decode for $t {
                #[inline]
                fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
                    let v = reader.$reader()?;
                    if v as u64 > <$t>::MAX as u64 {
                        return Err(DecodeError::LengthOverflow);
                    }
                    Ok(v as $t)
                }
            }
        )*
    };
}
impl_decode_uvarint!(
    u16 => read_varint_u32,
    u32 => read_varint_u32,
    u64 => read_varint_u64,
    usize => read_varint_u64,
);

macro_rules! impl_decode_ivarint {
    ($($t:ty => $reader:ident),* $(,)?) => {
        $(
            impl Decode for $t {
                #[inline]
                fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
                    // Postcard signed varint = zig-zag over the unsigned varint.
                    let raw = reader.$reader()? as u64;
                    // Zig-zag decode: (raw >> 1) ^ -(raw & 1)
                    let decoded = ((raw >> 1) as i64) ^ -((raw & 1) as i64);
                    if decoded < <$t>::MIN as i64 || decoded > <$t>::MAX as i64 {
                        return Err(DecodeError::LengthOverflow);
                    }
                    Ok(decoded as $t)
                }
            }
        )*
    };
}
impl_decode_ivarint!(
    i16 => read_varint_u32,
    i32 => read_varint_u32,
    i64 => read_varint_u64,
    isize => read_varint_u64,
);

impl Decode for String {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        // Cap str length at 16 MiB by default. Callers that legitimately
        // need larger strings should compose their own `Decode` impl that
        // checks application-specific limits before calling `read_seq_bytes`.
        let bytes = reader.read_seq_bytes(budget, 16 * 1024 * 1024)?;
        let s = core::str::from_utf8(bytes).map_err(|_| DecodeError::Utf8)?;
        Ok(String::from(s))
    }
}

impl Decode for alloc::boxed::Box<str> {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        Ok(String::decode(reader, budget)?.into_boxed_str())
    }
}

impl Decode for alloc::boxed::Box<[u8]> {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let bytes = reader.read_seq_bytes(budget, 16 * 1024 * 1024)?;
        Ok(Vec::from(bytes).into_boxed_slice())
    }
}

impl Decode for f32 {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        // Postcard encodes f32 as 4 raw little-endian bytes.
        Ok(f32::from_le_bytes(reader.read_array::<4>()?))
    }
}

impl Decode for f64 {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        // Postcard encodes f64 as 8 raw little-endian bytes.
        Ok(f64::from_le_bytes(reader.read_array::<8>()?))
    }
}

impl Decode for u128 {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        reader.read_varint_u128()
    }
}

impl Decode for i128 {
    #[inline]
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        // Postcard signed varint = zig-zag over the unsigned varint.
        // Zig-zag decode for i128: (raw >> 1) ^ -(raw & 1).
        let raw = reader.read_varint_u128()?;
        Ok(((raw >> 1) as i128) ^ -((raw & 1) as i128))
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(T::decode(reader, budget)?)),
            seen => Err(DecodeError::InvalidVariant {
                type_name: "Option",
                seen: seen as u32,
            }),
        }
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let len = reader.read_varint_u32()? as usize;
        // Cap a single sequence at 1 M elements to bound the worst-case
        // allocation regardless of how many bytes the budget has left.
        if len > 1 << 20 {
            return Err(DecodeError::LengthOverflow);
        }
        let mut out = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            // Each element charges one descent so deep nested Vec<Vec<…>>
            // cannot escape the recursion bound. The outer `?` surfaces
            // `BudgetError::Depth`; the inner `?` surfaces a per-element
            // `DecodeError`.
            let item = budget.descend(|b| T::decode(reader, b))??;
            out.push(item);
        }
        Ok(out)
    }
}

impl<A: smallvec::Array> Decode for smallvec::SmallVec<A>
where
    A::Item: Decode,
{
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let len = reader.read_varint_u32()? as usize;
        if len > 1 << 20 {
            return Err(DecodeError::LengthOverflow);
        }
        let mut out = smallvec::SmallVec::new();
        for _ in 0..len {
            let item = budget.descend(|b| A::Item::decode(reader, b))??;
            out.push(item);
        }
        Ok(out)
    }
}

impl<Tag: ?Sized> Decode for dol_core::id::Id<Tag> {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        let v = reader.read_varint_u32()?;
        dol_core::id::Id::from_u32(v).ok_or(DecodeError::Custom("Id cannot be zero"))
    }
}

impl Decode for alloc::sync::Arc<str> {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let s = String::decode(reader, budget)?;
        Ok(alloc::sync::Arc::from(s.as_str()))
    }
}

impl<A: Decode, B: Decode> Decode for (A, B) {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let a = budget.descend(|b| A::decode(reader, b))??;
        let b = budget.descend(|b| B::decode(reader, b))??;
        Ok((a, b))
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::policy::Limits;

    fn fuzz_budget() -> Budget {
        Budget::new(Limits::fuzz())
    }

    #[test]
    fn primitives_round_trip_postcard_byte_layout() {
        // String "hello": varint(5) 'h' 'e' 'l' 'l' 'o'
        let bytes = [0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let s = String::decode(&mut r, &mut b).unwrap();
        assert_eq!(s, "hello");
        assert!(r.is_exhausted());
    }

    #[test]
    fn option_some_some() {
        let bytes = [0x01, 0x2a]; // Some(42)
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let v: Option<u32> = Decode::decode(&mut r, &mut b).unwrap();
        assert_eq!(v, Some(42));
    }

    #[test]
    fn option_none() {
        let bytes = [0x00];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let v: Option<u32> = Decode::decode(&mut r, &mut b).unwrap();
        assert_eq!(v, None);
    }

    #[test]
    fn option_invalid_discriminant() {
        let bytes = [0x05];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let res: Result<Option<u32>, _> = Decode::decode(&mut r, &mut b);
        assert!(matches!(res, Err(DecodeError::InvalidVariant { .. })));
    }

    #[test]
    fn vec_bounded() {
        // Vec<u32> of len=3, [1, 2, 3]
        let bytes = [0x03, 0x01, 0x02, 0x03];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let v: Vec<u32> = Decode::decode(&mut r, &mut b).unwrap();
        assert_eq!(v, alloc::vec![1u32, 2, 3]);
    }

    #[test]
    fn signed_zigzag_round_trips() {
        // i32::MIN encodes as varint(2*|i32::MIN| - 1) = max u32 unsigned.
        // Just verify a couple of small values + one negative.
        let bytes = [0x00]; // 0
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let n: i32 = Decode::decode(&mut r, &mut b).unwrap();
        assert_eq!(n, 0);

        let bytes = [0x01]; // -1 (zig-zag(1) -> -1)
        let mut r = Reader::new(&bytes);
        let n: i32 = Decode::decode(&mut r, &mut b).unwrap();
        assert_eq!(n, -1);

        let bytes = [0x02]; // 1 (zig-zag(2) -> 1)
        let mut r = Reader::new(&bytes);
        let n: i32 = Decode::decode(&mut r, &mut b).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn truncated_input_eofs_cleanly() {
        let bytes = [0x05, b'h', b'i']; // claims len=5 but only 2 bytes follow
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let res: Result<String, _> = Decode::decode(&mut r, &mut b);
        assert!(matches!(res, Err(DecodeError::Eof { .. })));
    }

    #[test]
    fn varint_overflow_rejected() {
        // Six bytes all with continuation set → overflows the u32 varint cap.
        let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let res: Result<u32, _> = Decode::decode(&mut r, &mut b);
        assert!(matches!(res, Err(DecodeError::LengthOverflow)));
    }

    #[test]
    fn invalid_utf8_rejected() {
        // varint(2), 0xC3, 0x28  — invalid UTF-8 (lone start byte).
        let bytes = [0x02, 0xC3, 0x28];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();
        let res: Result<String, _> = Decode::decode(&mut r, &mut b);
        assert!(matches!(res, Err(DecodeError::Utf8)));
    }

    #[test]
    fn compound_decodes_fields_in_declaration_order() {
        // Manually compose bytes for: String "hello", Option<u32> Some(42),
        // Vec<bool> [false, true, false] — the same sequence that would
        // appear if they were fields of a struct decoded left-to-right.
        let bytes = [
            0x05, b'h', b'e', b'l', b'l', b'o', // label = "hello"
            0x01, 0x2a, // count = Some(42)
            0x03, 0x00, 0x01, 0x00, // flags = [false, true, false]
        ];
        let mut r = Reader::new(&bytes);
        let mut b = fuzz_budget();

        let label = b.descend(|b| String::decode(&mut r, b)).unwrap().unwrap();
        let count = b
            .descend(|b| Option::<u32>::decode(&mut r, b))
            .unwrap()
            .unwrap();
        let flags = b
            .descend(|b| Vec::<bool>::decode(&mut r, b))
            .unwrap()
            .unwrap();

        assert_eq!(label, "hello");
        assert_eq!(count, Some(42));
        assert_eq!(flags, alloc::vec![false, true, false]);
        assert!(r.is_exhausted());
    }

    #[test]
    fn budget_exhaustion_surfaces_as_decode_error() {
        // Build a deeply-nested Vec<Vec<Vec<u8>>> that would exceed the
        // budget if recursion charged one descent per level. A `max_depth`
        // of 1 lets the outermost descent succeed but the next one fails.
        let mut budget = Budget::new(Limits {
            max_depth: 1,
            ..Limits::fuzz()
        });
        let bytes = [0x01, 0x01, 0x01, 0x01]; // varint(1), varint(1), varint(1), 0x01 byte
        let mut r = Reader::new(&bytes);
        let res: Result<Vec<Vec<Vec<u8>>>, _> = Decode::decode(&mut r, &mut budget);
        assert!(matches!(res, Err(DecodeError::Budget(_))));
    }
}

//! Validating, budget-aware wire encoder — the v2 single wire-out entry point.
//!
//! `dol-wire::encoder` defines [`Encode`], the symmetric counterpart to
//! [`Decode`](crate::decoder::Decode). Every [`Encode`] impl:
//!
//! - takes a [`&mut Budget`](dol_core::policy::Budget) so deeply nested
//!   payloads charge per-descent against the same recursion cap as
//!   [`Decode`] (a producer cannot accidentally emit a `Vec<Vec<…>>`
//!   tower the peer can't decode), and
//! - shares postcard's byte format with the existing
//!   [`encode_postcard`](crate::postcard::encode_postcard) helper so the
//!   cut-over from `Serialize`-derived encoding is byte-for-byte
//!   transparent. The `encode_core_roundtrip` integration test asserts
//!   parity against `postcard::to_allocvec(&value)` for every type that
//!   has both an [`Encode`] and a `Serialize` derivation.
//!
//! # Wire format
//!
//! See [`Decode`](crate::decoder::Decode) for the full table — `Encode`
//! emits the exact same bytes that `Decode` reads. The supported leaf
//! encodings are postcard-compatible:
//!
//! | type            | encoding                                        |
//! | --------------- | ----------------------------------------------- |
//! | `bool`          | one byte; `0 = false`, `1 = true`               |
//! | `u8` / `i8`     | one byte raw                                    |
//! | `u16/u32/u64`   | unsigned varint, little-endian shifts of 7 bits |
//! | `i16/i32/i64`   | zig-zag varint                                  |
//! | `&str`          | varint length + UTF-8 payload                   |
//! | `&[u8]`         | varint length + raw bytes                       |
//! | `Option<T>`     | one byte (`0 = None`, `1 = Some`) + payload     |
//! | `Vec<T>` / seq  | varint length + payload                         |
//! | enum            | varint discriminant + payload                   |
//! | struct          | fields concatenated in declaration order        |
//!
//! # Cut-over status
//!
//! v2 (0.2.0) lands the trait, the [`Writer`] sink, the helpers, and
//! `Encode` impls for every primitive plus the same dol-core leaves that
//! [`crate::decode_core`] covers
//! (`BitString`, `FileId`, `Date`, `Time`, `DateTime`, `Offset`,
//! `TimestampTz`, `Interval`, `Decimal`, `Point`, `Line`, `Segment`,
//! `Rect`, `Circle`). Recursive enums and the `Deserialize`-derive strip
//! follow in the next focused PR (matching the [`Decode`] follow-up).
//!
//! # Example
//!
//! ```
//! use dol_core::policy::{Budget, Limits};
//! use dol_wire::encoder::{Encode, Writer, encode_to_vec};
//!
//! let mut budget = Budget::new(Limits::host());
//! let bytes = encode_to_vec(&"hello".to_string(), &mut budget).unwrap();
//! assert_eq!(&bytes, &[0x05, b'h', b'e', b'l', b'l', b'o']);
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use dol_core::policy::{Budget, BudgetError};

// ─── Sink ────────────────────────────────────────────────────────────────────

/// Append-only byte sink backing every [`Encode`] impl.
///
/// `Writer` borrows a mutable [`Vec<u8>`] and exposes only append operations
/// so the wire-side equivalent of [`Reader`](crate::decoder::Reader) is
/// equally allocator-disciplined: no panics, no implicit reallocation
/// fan-out, no out-of-bounds writes.
#[derive(Debug)]
pub struct Writer<'a> {
    buf: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    /// Wrap `buf`; subsequent writes append to it.
    #[inline]
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { buf }
    }

    /// Number of bytes appended so far. Equal to `buf.len()` when the
    /// `Writer` was constructed against an empty buffer.
    #[inline]
    pub fn bytes_written(&self) -> usize {
        self.buf.len()
    }

    /// Emit a single byte.
    #[inline]
    pub fn write_u8(&mut self, byte: u8) -> Result<(), EncodeError> {
        self.buf.push(byte);
        Ok(())
    }

    /// Emit a raw byte slice (no length prefix).
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }

    /// Emit a fixed-length array (no length prefix). Mirrors
    /// [`Reader::read_array`](crate::decoder::Reader::read_array).
    #[inline]
    pub fn write_array<const N: usize>(&mut self, arr: &[u8; N]) -> Result<(), EncodeError> {
        self.buf.extend_from_slice(arr);
        Ok(())
    }

    /// Emit a postcard-style unsigned varint (max 5 bytes for `u32`).
    pub fn write_varint_u32(&mut self, mut value: u32) -> Result<(), EncodeError> {
        // Postcard varint: 7-bit chunks little-endian, MSB set on every
        // non-final byte. Max five chunks for `u32`.
        for _ in 0..4 {
            let chunk = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.buf.push(chunk);
                return Ok(());
            }
            self.buf.push(chunk | 0x80);
        }
        // Fifth (terminal) chunk for values that need the full 5 bytes.
        // The remaining `value` here is already < 1 << 4 by construction.
        self.buf.push((value & 0x7F) as u8);
        Ok(())
    }

    /// Emit a postcard-style unsigned varint (max 10 bytes for `u64`).
    pub fn write_varint_u64(&mut self, mut value: u64) -> Result<(), EncodeError> {
        for _ in 0..9 {
            let chunk = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.buf.push(chunk);
                return Ok(());
            }
            self.buf.push(chunk | 0x80);
        }
        self.buf.push((value & 0x7F) as u8);
        Ok(())
    }

    /// Emit a postcard-style unsigned varint (max 19 bytes for `u128`).
    pub fn write_varint_u128(&mut self, mut value: u128) -> Result<(), EncodeError> {
        for _ in 0..18 {
            let chunk = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.buf.push(chunk);
                return Ok(());
            }
            self.buf.push(chunk | 0x80);
        }
        self.buf.push((value & 0x7F) as u8);
        Ok(())
    }

    /// Emit a varint length followed by `bytes`. Charges one budget unit
    /// per length-prefixed write so symmetry with
    /// [`Reader::read_seq_bytes`](crate::decoder::Reader::read_seq_bytes)
    /// is preserved on both halves of the wire.
    pub fn write_seq_bytes(
        &mut self,
        budget: &mut Budget,
        bytes: &[u8],
    ) -> Result<(), EncodeError> {
        // Match `read_seq_bytes`: charge one tick per length-prefixed write
        // so the same recursion cap applies in both directions.
        budget.tick(1)?;
        // `usize` may exceed `u32` on 64-bit hosts; reject anything that
        // wouldn't round-trip through `read_varint_u32`.
        let len: u32 = bytes
            .len()
            .try_into()
            .map_err(|_| EncodeError::LengthOverflow)?;
        self.write_varint_u32(len)?;
        self.buf.extend_from_slice(bytes);
        Ok(())
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Reasons an [`Encode`] impl may reject its input.
#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError {
    /// A length-prefixed payload exceeded `u32::MAX` bytes (the maximum
    /// the postcard varint width supports).
    LengthOverflow,
    /// Recursive [`Encode`] call exhausted the [`Budget`].
    Budget(BudgetError),
    /// Custom payload-specific failure.
    Custom(&'static str),
}

impl From<BudgetError> for EncodeError {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthOverflow => f.write_str("dol-wire encode: length-prefix overflow"),
            Self::Budget(e) => write!(f, "dol-wire encode: budget exhausted: {e}"),
            Self::Custom(s) => write!(f, "dol-wire encode: {s}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

// ─── Trait + impls for the primitive leaves ─────────────────────────────────

/// Validating, budget-aware byte-vector writer for an in-memory IR / AST type.
///
/// See the module docs for the wire format and the cut-over status of v2.
pub trait Encode {
    /// Append `self` to `writer`, charging recursion against `budget`.
    fn encode(&self, writer: &mut Writer<'_>, budget: &mut Budget) -> Result<(), EncodeError>;
}

/// Convenience helper: encode `value` into a fresh `Vec<u8>`.
pub fn encode_to_vec<T: Encode + ?Sized>(
    value: &T,
    budget: &mut Budget,
) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::new();
    {
        let mut w = Writer::new(&mut buf);
        value.encode(&mut w, budget)?;
    }
    Ok(buf)
}

impl Encode for u8 {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_u8(*self)
    }
}

impl Encode for i8 {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_u8(*self as u8)
    }
}

impl Encode for bool {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_u8(u8::from(*self))
    }
}

macro_rules! impl_encode_uvarint {
    ($($t:ty => $writer:ident),* $(,)?) => {
        $(
            impl Encode for $t {
                #[inline]
                fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
                    w.$writer(*self as _)
                }
            }
        )*
    };
}
impl_encode_uvarint!(
    u16 => write_varint_u32,
    u32 => write_varint_u32,
    u64 => write_varint_u64,
    usize => write_varint_u64,
);

macro_rules! impl_encode_ivarint {
    ($($t:ty as $u:ty => $writer:ident),* $(,)?) => {
        $(
            impl Encode for $t {
                #[inline]
                fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
                    // Postcard signed varint = zig-zag over the unsigned varint.
                    // For an N-bit signed integer, zig-zag(v) = ((v << 1) ^ (v >> (N-1))).
                    // The `<< 1` must wrap (e.g. `i32::MIN << 1` overflows in debug);
                    // the `>> (N-1)` is arithmetic so the resulting xor produces the
                    // correct unsigned encoding for negative values.
                    let bits = (core::mem::size_of::<$t>() as u32).saturating_mul(8);
                    let shift = bits.saturating_sub(1);
                    let zz = (self.wrapping_shl(1) ^ (*self >> shift)) as $u;
                    w.$writer(zz as _)
                }
            }
        )*
    };
}
impl_encode_ivarint!(
    i16 as u16 => write_varint_u32,
    i32 as u32 => write_varint_u32,
    i64 as u64 => write_varint_u64,
    isize as usize => write_varint_u64,
);

impl Encode for u128 {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_varint_u128(*self)
    }
}

impl Encode for i128 {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        // Zig-zag for i128: ((v << 1) ^ (v >> 127)) cast to u128. Wrapping
        // shift-left so `i128::MIN` doesn't overflow in debug builds; the
        // arithmetic `>> 127` produces all-ones for negatives.
        let zz = (self.wrapping_shl(1) ^ (*self >> 127)) as u128;
        w.write_varint_u128(zz)
    }
}

impl Encode for f32 {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        // Postcard encodes f32 as 4 raw little-endian bytes.
        w.write_array(&self.to_le_bytes())
    }
}

impl Encode for f64 {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        // Postcard encodes f64 as 8 raw little-endian bytes.
        w.write_array(&self.to_le_bytes())
    }
}

impl Encode for str {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        w.write_seq_bytes(b, self.as_bytes())
    }
}

impl Encode for String {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        <str as Encode>::encode(self.as_str(), w, b)
    }
}

impl Encode for Box<str> {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        <str as Encode>::encode(self, w, b)
    }
}

impl Encode for [u8] {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        w.write_seq_bytes(b, self)
    }
}

impl Encode for Box<[u8]> {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        <[u8] as Encode>::encode(self, w, b)
    }
}

impl<T: Encode + ?Sized> Encode for &T {
    #[inline]
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        (**self).encode(w, b)
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            None => w.write_u8(0),
            Some(v) => {
                w.write_u8(1)?;
                v.encode(w, b)
            }
        }
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        let len: u32 = self
            .len()
            .try_into()
            .map_err(|_| EncodeError::LengthOverflow)?;
        w.write_varint_u32(len)?;
        for item in self {
            // Each element charges one descent so deep nested Vec<Vec<…>>
            // cannot escape the recursion bound — symmetric with
            // `Decode for Vec<T>`. Outer `?` surfaces `BudgetError::Depth`;
            // inner `?` surfaces a per-element `EncodeError`.
            b.descend(|b| item.encode(w, b))??;
        }
        Ok(())
    }
}

// ─── Reference compound: architectural proof ────────────────────────────────

/// Encode-side mirror of [`crate::decoder::DecodeWrap`]. Exists to exercise
/// the trait shape from integration tests while the recursive enums are
/// being filled in by follow-up PRs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeWrap {
    /// String payload — exercises [`Encode for String`].
    pub label: String,
    /// Optional unsigned — exercises [`Encode for Option<u32>`].
    pub count: Option<u32>,
    /// Sequence — exercises [`Encode for Vec<T>`] plus budget descent.
    pub flags: Vec<bool>,
}

impl Encode for EncodeWrap {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        // A struct encode walks fields in declaration order, charging one
        // budget unit per descent so a deeply nested wrapper graph cannot
        // exceed the configured recursion limit. Mirrors `DecodeWrap`.
        b.descend(|b| self.label.encode(w, b))??;
        b.descend(|b| self.count.encode(w, b))??;
        b.descend(|b| self.flags.encode(w, b))??;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::policy::Limits;

    fn fuzz_budget() -> Budget {
        Budget::new(Limits::fuzz())
    }

    #[test]
    fn primitives_emit_postcard_byte_layout() {
        let mut b = fuzz_budget();
        // String "hello": varint(5) 'h' 'e' 'l' 'l' 'o'
        let bytes = encode_to_vec(&String::from("hello"), &mut b).unwrap();
        assert_eq!(&bytes, &[0x05, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn option_some_emits_tag_and_payload() {
        let mut b = fuzz_budget();
        let bytes = encode_to_vec(&Some(42u32), &mut b).unwrap();
        assert_eq!(&bytes, &[0x01, 0x2a]);
    }

    #[test]
    fn option_none_emits_zero() {
        let mut b = fuzz_budget();
        let none: Option<u32> = None;
        let bytes = encode_to_vec(&none, &mut b).unwrap();
        assert_eq!(&bytes, &[0x00]);
    }

    #[test]
    fn vec_emits_length_prefix() {
        let mut b = fuzz_budget();
        let bytes = encode_to_vec(&alloc::vec![1u32, 2, 3], &mut b).unwrap();
        assert_eq!(&bytes, &[0x03, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn signed_zigzag_matches_decoder() {
        let mut b = fuzz_budget();
        assert_eq!(encode_to_vec(&0i32, &mut b).unwrap(), alloc::vec![0x00]);
        assert_eq!(encode_to_vec(&-1i32, &mut b).unwrap(), alloc::vec![0x01]);
        assert_eq!(encode_to_vec(&1i32, &mut b).unwrap(), alloc::vec![0x02]);
        // -2 zig-zag → 3.
        assert_eq!(encode_to_vec(&-2i32, &mut b).unwrap(), alloc::vec![0x03]);
    }

    #[test]
    fn round_trip_decode_wrap_via_encode_wrap() {
        // EncodeWrap → bytes → DecodeWrap: the architectural compound proof.
        use crate::decoder::{Decode, DecodeWrap, Reader};
        let v = EncodeWrap {
            label: "hi".into(),
            count: Some(7),
            flags: alloc::vec![true, false, true],
        };
        let mut b = fuzz_budget();
        let bytes = encode_to_vec(&v, &mut b).unwrap();
        let mut r = Reader::new(&bytes);
        let mut b2 = fuzz_budget();
        let dec = DecodeWrap::decode(&mut r, &mut b2).unwrap();
        assert_eq!(dec.label, v.label);
        assert_eq!(dec.count, v.count);
        assert_eq!(dec.flags, v.flags);
        assert!(r.is_exhausted());
    }
}

//! Bit-string type.

use super::error::TypeError;
use alloc::boxed::Box;
use alloc::vec;
use core::fmt;

/// A packed bit string with an explicit bit count.
///
/// Maps to SQL `BIT(n)` / `VARBIT(n)`, PostgreSQL `bit` / `varbit`, and
/// binary protocol flags. The byte buffer always satisfies
/// `bytes.len() == len.div_ceil(8)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BitString {
    /// Bit count.
    pub len: u32,
    /// Packed bytes; high bit of `bytes[0]` is bit 0.
    pub bytes: Box<[u8]>,
}

impl BitString {
    /// Validates that `bytes.len() == len.div_ceil(8)`.
    pub fn try_new(len: u32, bytes: Box<[u8]>) -> Result<Self, TypeError> {
        let required = len.div_ceil(8) as usize;
        if bytes.len() != required {
            return Err(TypeError::BitLengthMismatch {
                declared: len,
                byte_count: bytes.len(),
            });
        }
        Ok(Self { len, bytes })
    }

    pub fn new_unchecked(len: u32, bytes: Box<[u8]>) -> Self {
        Self { len, bytes }
    }

    /// Creates a zero-filled bit string.
    pub fn zeroes(len: u32) -> Self {
        let byte_count = len.div_ceil(8) as usize;
        Self {
            len,
            bytes: vec![0u8; byte_count].into_boxed_slice(),
        }
    }
}

impl fmt::Display for BitString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("b'")?;
        for i in 0..self.len {
            let byte = self.bytes[(i / 8) as usize];
            let bit = (byte >> (7 - (i % 8))) & 1;
            write!(f, "{bit}")?;
        }
        f.write_str("'")
    }
}

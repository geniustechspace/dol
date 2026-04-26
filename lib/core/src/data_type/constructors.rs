//! Convenience constructors for [`DataType`].
//!
//! Store-neutral helpers that keep call sites concise without referencing
//! any specific backend's spelling.

use super::DataType;

impl DataType {
    /// A bounded variable-length character string (`max_len` characters).
    pub const fn varying_string(max_len: u32) -> Self {
        Self::String {
            max_len: Some(max_len),
            fixed: false,
        }
    }

    /// A fixed-length character string (`len` characters).
    pub const fn fixed_string(len: u32) -> Self {
        Self::String {
            max_len: Some(len),
            fixed: true,
        }
    }

    /// An unbounded character string.
    pub const fn unbounded_string() -> Self {
        Self::String {
            max_len: None,
            fixed: false,
        }
    }

    /// A bounded variable-length byte string.
    pub const fn varying_bytes(max_len: u32) -> Self {
        Self::Bytes {
            max_len: Some(max_len),
            fixed: false,
        }
    }

    /// A fixed-length byte string.
    pub const fn fixed_bytes(len: u32) -> Self {
        Self::Bytes {
            max_len: Some(len),
            fixed: true,
        }
    }

    /// An unbounded byte string.
    pub const fn unbounded_bytes() -> Self {
        Self::Bytes {
            max_len: None,
            fixed: false,
        }
    }

    /// A bounded variable-length bit string.
    pub const fn varying_bits(max_len: u32) -> Self {
        Self::BitString {
            max_len: Some(max_len),
            fixed: false,
        }
    }

    /// A fixed-length bit string.
    pub const fn fixed_bits(len: u32) -> Self {
        Self::BitString {
            max_len: Some(len),
            fixed: true,
        }
    }
}

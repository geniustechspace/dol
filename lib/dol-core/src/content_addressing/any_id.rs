use core::fmt;

use super::Kind;

/// Type-erased content identifier for storage, manifests, and generic CAS
/// plumbing. Prefer typed `ContentId<BITS, Tag>` in normal APIs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnyContentId {
    Bits64 { kind: Kind, bytes: [u8; 8] },
    Bits128 { kind: Kind, bytes: [u8; 16] },
    Bits160 { kind: Kind, bytes: [u8; 20] },
    Bits256 { kind: Kind, bytes: [u8; 32] },
}

impl AnyContentId {
    pub const fn kind(&self) -> Kind {
        match self {
            Self::Bits64 { kind, .. }
            | Self::Bits128 { kind, .. }
            | Self::Bits160 { kind, .. }
            | Self::Bits256 { kind, .. } => *kind,
        }
    }

    pub const fn bit_len(&self) -> usize {
        match self {
            Self::Bits64 { .. } => 64,
            Self::Bits128 { .. } => 128,
            Self::Bits160 { .. } => 160,
            Self::Bits256 { .. } => 256,
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bits64 { bytes, .. } => bytes,
            Self::Bits128 { bytes, .. } => bytes,
            Self::Bits160 { bytes, .. } => bytes,
            Self::Bits256 { bytes, .. } => bytes,
        }
    }
}

impl fmt::Debug for AnyContentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyContentId")
            .field("kind", &self.kind())
            .field("bit_len", &self.bit_len())
            .field("hex", &HexDebug(self.as_bytes()))
            .finish()
    }
}

struct HexDebug<'a>(&'a [u8]);

impl fmt::Debug for HexDebug<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("\"")?;
        for byte in self.0 {
            formatter.write_fmt(format_args!("{byte:02x}"))?;
        }
        formatter.write_str("\"")
    }
}

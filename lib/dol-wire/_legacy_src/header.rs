//! Wire format header.

/// 4-byte magic prefix for any DOL wire payload: `b"DOL\0"`.
pub const MAGIC: [u8; 4] = *b"DOL\0";

/// Major.minor schema version. Additive changes bump `minor`; renames or
/// removals bump `major` and require a migration shim in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WireSchemaVersion {
    /// Breaking changes.
    pub major: u16,
    /// Additive changes.
    pub minor: u16,
}

impl WireSchemaVersion {
    /// Build a version literal.
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// `true` if `self` is forward-compatible with `other` (same major, our
    /// minor ≥ other's minor).
    pub const fn accepts(self, other: Self) -> bool {
        self.major == other.major && self.minor >= other.minor
    }

    /// Encode into 4 bytes (little-endian).
    pub fn to_bytes(self) -> [u8; 4] {
        let mut out = [0u8; 4];
        out[0..2].copy_from_slice(&self.major.to_le_bytes());
        out[2..4].copy_from_slice(&self.minor.to_le_bytes());
        out
    }

    /// Decode from 4 bytes (little-endian).
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            major: u16::from_le_bytes([bytes[0], bytes[1]]),
            minor: u16::from_le_bytes([bytes[2], bytes[3]]),
        }
    }
}

/// The version this build of `dol-wire` understands.
pub const CURRENT_VERSION: WireSchemaVersion = WireSchemaVersion::new(0, 1);

/// 8-byte wire header: 4-byte magic + 4-byte [`WireSchemaVersion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireHeader {
    /// Schema version.
    pub version: WireSchemaVersion,
}

impl WireHeader {
    /// Build a header for the current version.
    pub const fn current() -> Self {
        Self {
            version: CURRENT_VERSION,
        }
    }

    /// Encode to 8 bytes.
    pub fn to_bytes(self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..4].copy_from_slice(&MAGIC);
        out[4..8].copy_from_slice(&self.version.to_bytes());
        out
    }

    /// Decode an 8-byte header.
    pub fn from_bytes(bytes: [u8; 8]) -> Result<Self, crate::WireError> {
        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != MAGIC {
            return Err(crate::WireError::BadMagic);
        }
        let version = WireSchemaVersion::from_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if !CURRENT_VERSION.accepts(version) {
            return Err(crate::WireError::VersionMismatch {
                found: version,
                expected: CURRENT_VERSION,
            });
        }
        Ok(Self { version })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let h = WireHeader::current();
        let b = h.to_bytes();
        let g = WireHeader::from_bytes(b).unwrap();
        assert_eq!(h, g);
    }

    #[test]
    fn header_rejects_bad_magic() {
        let mut b = WireHeader::current().to_bytes();
        b[0] = 0;
        assert!(matches!(
            WireHeader::from_bytes(b),
            Err(crate::WireError::BadMagic)
        ));
    }
}

use core::fmt;
use core::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
enum MacAddrErrorKind {
    /// The input did not parse as a MAC address.
    Invalid,
    /// The value was a valid `MacAddr`, but not the EUI-48 width requested
    /// by a [`TryFrom`] conversion.
    ExpectedEui48,
    /// The value was a valid `MacAddr`, but not the EUI-64 width requested
    /// by a [`TryFrom`] conversion.
    ExpectedEui64,
}

/// An error which can be returned when parsing a [`MacAddr`] or converting
/// one to a fixed-width byte array.
///
/// This error is used as the error type for the [`FromStr`] implementation
/// for [`MacAddr`] and for the [`TryFrom<MacAddr>`] conversions to
/// `[u8; 6]` and `[u8; 8]`.
///
/// # Potential causes
///
/// `ParseMacAddrError` may be returned because the provided string does not
/// parse as a MAC address, or because a valid `MacAddr` was the wrong width
/// for the requested conversion (e.g. converting an EUI-64 to `[u8; 6]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMacAddrError(MacAddrErrorKind);

impl ParseMacAddrError {
    const fn invalid() -> Self {
        Self(MacAddrErrorKind::Invalid)
    }

    const fn expected_eui48() -> Self {
        Self(MacAddrErrorKind::ExpectedEui48)
    }

    const fn expected_eui64() -> Self {
        Self(MacAddrErrorKind::ExpectedEui64)
    }
}

impl fmt::Display for ParseMacAddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self.0 {
            MacAddrErrorKind::Invalid => "invalid mac address",
            MacAddrErrorKind::ExpectedEui48 => "mac address is not EUI-48",
            MacAddrErrorKind::ExpectedEui64 => "mac address is not EUI-64",
        };
        f.write_str(msg)
    }
}

impl core::error::Error for ParseMacAddrError {}

/// An Ethernet MAC address.
///
/// This stores either EUI-48 (6 bytes) or EUI-64 (8 bytes) for parity with
/// [`super::IpAddr`], which models multiple wire widths as one enum.
///
/// # Serde representation
///
/// With the `serde` feature, `MacAddr` serialises **untagged** as the bare
/// octet array of its variant: a 6-byte array for [`MacAddr::Eui48`] and an
/// 8-byte array for [`MacAddr::Eui64`]. The two widths are unambiguous on
/// the wire, so no enum tag is emitted.
///
/// ```json
/// // EUI-48
/// [0, 26, 43, 60, 77, 94]
/// // EUI-64
/// [0, 26, 43, 60, 77, 94, 111, 128]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum MacAddr {
    Eui48([u8; 6]),
    Eui64([u8; 8]),
}

impl MacAddr {
    pub const fn eui48(b: [u8; 6]) -> Self {
        Self::Eui48(b)
    }

    pub const fn eui64(b: [u8; 8]) -> Self {
        Self::Eui64(b)
    }

    pub const fn is_eui48(&self) -> bool {
        matches!(self, Self::Eui48(_))
    }

    pub const fn is_eui64(&self) -> bool {
        matches!(self, Self::Eui64(_))
    }

    pub const fn octets_48(self) -> Option<[u8; 6]> {
        if let Self::Eui48(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub const fn octets_64(self) -> Option<[u8; 8]> {
        if let Self::Eui64(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_bytes(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for (i, b) in bytes.iter().enumerate() {
                if i > 0 {
                    f.write_str(":")?;
                }
                write!(f, "{b:02x}")?;
            }
            Ok(())
        }

        match self {
            Self::Eui48(bytes) => write_bytes(bytes, f),
            Self::Eui64(bytes) => write_bytes(bytes, f),
        }
    }
}

impl From<[u8; 6]> for MacAddr {
    fn from(value: [u8; 6]) -> Self {
        Self::Eui48(value)
    }
}

impl From<[u8; 8]> for MacAddr {
    fn from(value: [u8; 8]) -> Self {
        Self::Eui64(value)
    }
}

impl TryFrom<MacAddr> for [u8; 6] {
    type Error = ParseMacAddrError;

    fn try_from(value: MacAddr) -> Result<Self, Self::Error> {
        value
            .octets_48()
            .ok_or_else(ParseMacAddrError::expected_eui48)
    }
}

impl TryFrom<MacAddr> for [u8; 8] {
    type Error = ParseMacAddrError;

    fn try_from(value: MacAddr) -> Result<Self, Self::Error> {
        value
            .octets_64()
            .ok_or_else(ParseMacAddrError::expected_eui64)
    }
}

fn parse_hex_byte(part: &str) -> Option<u8> {
    if part.len() != 2 {
        return None;
    }

    let mut chars = part.chars();
    let hi = chars.next()?.to_digit(16)? as u8;
    let lo = chars.next()?.to_digit(16)? as u8;
    Some((hi << 4) | lo)
}

impl FromStr for MacAddr {
    type Err = ParseMacAddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let separator = if s.contains(':') {
            ':'
        } else if s.contains('-') {
            '-'
        } else {
            return Err(ParseMacAddrError::invalid());
        };

        let mut bytes = [0u8; 8];
        let mut count = 0usize;

        for part in s.split(separator) {
            if count == bytes.len() {
                return Err(ParseMacAddrError::invalid());
            }
            // `count < bytes.len()` is enforced by the guard above; the
            // increment cannot overflow because it is capped at 8.
            #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
            {
                bytes[count] = parse_hex_byte(part).ok_or_else(ParseMacAddrError::invalid)?;
                count += 1;
            }
        }

        match count {
            6 => Ok(Self::Eui48([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
            ])),
            8 => Ok(Self::Eui64(bytes)),
            _ => Err(ParseMacAddrError::invalid()),
        }
    }
}

impl TryFrom<&str> for MacAddr {
    type Error = ParseMacAddrError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<alloc::string::String> for MacAddr {
    type Error = ParseMacAddrError;

    fn try_from(value: alloc::string::String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<alloc::boxed::Box<str>> for MacAddr {
    type Error = ParseMacAddrError;

    fn try_from(value: alloc::boxed::Box<str>) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::{MacAddr, ParseMacAddrError};
    use crate::alloc::string::ToString;

    #[test]
    fn display_eui48() {
        let mac = MacAddr::eui48([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);
        assert_eq!(mac.to_string(), "00:1a:2b:3c:4d:5e");
    }

    #[test]
    fn display_eui64() {
        let mac = MacAddr::eui64([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x70]);
        assert_eq!(mac.to_string(), "00:1a:2b:3c:4d:5e:6f:70");
    }

    #[test]
    fn parse_eui48_and_eui64() {
        let e48: MacAddr = "00:1a:2b:3c:4d:5e".parse().expect("valid 48-bit mac");
        let e64: MacAddr = "00-1a-2b-3c-4d-5e-6f-70".parse().expect("valid 64-bit mac");

        assert_eq!(e48, MacAddr::eui48([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]));
        assert_eq!(
            e64,
            MacAddr::eui64([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x70])
        );
    }

    #[test]
    fn parse_rejects_invalid() {
        assert!("00:11:22:33:44".parse::<MacAddr>().is_err());
        assert!("zz:11:22:33:44:55".parse::<MacAddr>().is_err());
    }

    #[test]
    fn try_from_distinguishes_width_mismatch_from_parse_error() {
        let e64 = MacAddr::eui64([0u8; 8]);
        let e48 = MacAddr::eui48([0u8; 6]);

        let bad_parse = "not-a-mac".parse::<MacAddr>().unwrap_err();
        let bad_48: ParseMacAddrError = <[u8; 6]>::try_from(e64).unwrap_err();
        let bad_64: ParseMacAddrError = <[u8; 8]>::try_from(e48).unwrap_err();

        assert_ne!(bad_parse, bad_48);
        assert_ne!(bad_parse, bad_64);
        assert_ne!(bad_48, bad_64);

        assert_eq!(bad_parse.to_string(), "invalid mac address");
        assert_eq!(bad_48.to_string(), "mac address is not EUI-48");
        assert_eq!(bad_64.to_string(), "mac address is not EUI-64");
    }
}

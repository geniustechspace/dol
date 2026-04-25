use core::fmt;
use core::str::FromStr;

/// Parse error for [`MacAddr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseMacAddrError;

impl fmt::Display for ParseMacAddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid mac address")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseMacAddrError {}

/// An Ethernet MAC address.
///
/// This stores either EUI-48 (6 bytes) or EUI-64 (8 bytes) for parity with
/// [`super::IpAddr`], which models multiple wire widths as one enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Backward-compatible alias for constructing a 6-byte MAC.
    pub const fn new(b: [u8; 6]) -> Self {
        Self::eui48(b)
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
        value.octets_48().ok_or(ParseMacAddrError)
    }
}

impl TryFrom<MacAddr> for [u8; 8] {
    type Error = ParseMacAddrError;

    fn try_from(value: MacAddr) -> Result<Self, Self::Error> {
        value.octets_64().ok_or(ParseMacAddrError)
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
            return Err(ParseMacAddrError);
        };

        let mut bytes = [0u8; 8];
        let mut count = 0usize;

        for part in s.split(separator) {
            if count == bytes.len() {
                return Err(ParseMacAddrError);
            }
            bytes[count] = parse_hex_byte(part).ok_or(ParseMacAddrError)?;
            count += 1;
        }

        match count {
            6 => Ok(Self::Eui48([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
            ])),
            8 => Ok(Self::Eui64(bytes)),
            _ => Err(ParseMacAddrError),
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
    use super::MacAddr;

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
}

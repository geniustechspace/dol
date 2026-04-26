use core::fmt;
use core::net::AddrParseError;
use core::str::FromStr;

/// An IP address literal.
///
/// Stored as a self-contained enum (not `core::net::IpAddr`) for consistent
/// size, alignment, and serde behaviour. `From` impls cover the stdlib types.
///
/// # Serde representation
///
/// With the `serde` feature, `IpAddr` serialises **untagged** as the bare
/// octet array of its variant: a 4-byte array for [`IpAddr::V4`] and a
/// 16-byte array for [`IpAddr::V6`]. The two widths are unambiguous on the
/// wire, so no enum tag is emitted.
///
/// ```json
/// // V4
/// [192, 168, 0, 1]
/// // V6
/// [32, 1, 13, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum IpAddr {
    V4([u8; 4]),
    V6([u8; 16]),
}

impl IpAddr {
    pub const fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self::V4([a, b, c, d])
    }
    pub const fn v6(bytes: [u8; 16]) -> Self {
        Self::V6(bytes)
    }
    pub const fn is_v4(&self) -> bool {
        matches!(self, Self::V4(_))
    }
    pub const fn is_v6(&self) -> bool {
        matches!(self, Self::V6(_))
    }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V4([a, b, c, d]) => write!(f, "{a}.{b}.{c}.{d}"),
            Self::V6(b) => {
                let w = |i: usize| u16::from_be_bytes([b[i * 2], b[i * 2 + 1]]);
                write!(
                    f,
                    "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                    w(0),
                    w(1),
                    w(2),
                    w(3),
                    w(4),
                    w(5),
                    w(6),
                    w(7)
                )
            }
        }
    }
}

impl From<[u8; 4]> for IpAddr {
    fn from(b: [u8; 4]) -> Self {
        Self::V4(b)
    }
}
impl From<[u8; 16]> for IpAddr {
    fn from(b: [u8; 16]) -> Self {
        Self::V6(b)
    }
}
impl From<core::net::Ipv4Addr> for IpAddr {
    fn from(a: core::net::Ipv4Addr) -> Self {
        Self::V4(a.octets())
    }
}
impl From<core::net::Ipv6Addr> for IpAddr {
    fn from(a: core::net::Ipv6Addr) -> Self {
        Self::V6(a.octets())
    }
}
impl From<core::net::IpAddr> for IpAddr {
    fn from(a: core::net::IpAddr) -> Self {
        match a {
            core::net::IpAddr::V4(v) => v.into(),
            core::net::IpAddr::V6(v) => v.into(),
        }
    }
}

impl From<IpAddr> for core::net::IpAddr {
    fn from(a: IpAddr) -> Self {
        match a {
            IpAddr::V4(octets) => core::net::IpAddr::V4(core::net::Ipv4Addr::from(octets)),
            IpAddr::V6(octets) => core::net::IpAddr::V6(core::net::Ipv6Addr::from(octets)),
        }
    }
}

impl FromStr for IpAddr {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<core::net::IpAddr>().map(Into::into)
    }
}

impl TryFrom<&str> for IpAddr {
    type Error = AddrParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<alloc::string::String> for IpAddr {
    type Error = AddrParseError;

    fn try_from(value: alloc::string::String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<alloc::boxed::Box<str>> for IpAddr {
    type Error = AddrParseError;

    fn try_from(value: alloc::boxed::Box<str>) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::IpAddr;

    #[test]
    fn parses_ipv4_from_str() {
        let ip: IpAddr = "192.168.0.1".parse().expect("valid IPv4");
        assert_eq!(ip, IpAddr::V4([192, 168, 0, 1]));
    }

    #[test]
    fn parses_ipv6_from_str() {
        let ip: IpAddr = "2001:db8::1".parse().expect("valid IPv6");
        let expected: IpAddr = core::net::Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1).into();
        assert_eq!(ip, expected);
    }

    #[test]
    fn parses_owned_string_variants() {
        let from_string =
            IpAddr::try_from(alloc::string::String::from("10.0.0.42")).expect("valid string ip");
        let from_boxed =
            IpAddr::try_from(alloc::boxed::Box::<str>::from("::1")).expect("valid boxed str ip");

        assert_eq!(from_string, IpAddr::V4([10, 0, 0, 42]));
        assert_eq!(
            from_boxed,
            IpAddr::V6(core::net::Ipv6Addr::LOCALHOST.octets())
        );
    }

    #[test]
    fn rejects_invalid_string_ip() {
        assert!("not-an-ip".parse::<IpAddr>().is_err());
    }
}

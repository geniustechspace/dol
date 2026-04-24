use core::fmt;

/// An IP address literal.
///
/// Stored as a self-contained enum (not `std::net::IpAddr`) for consistent
/// size, alignment, and serde behaviour. `From` impls cover the stdlib types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
impl From<std::net::Ipv4Addr> for IpAddr {
    fn from(a: std::net::Ipv4Addr) -> Self {
        Self::V4(a.octets())
    }
}
impl From<std::net::Ipv6Addr> for IpAddr {
    fn from(a: std::net::Ipv6Addr) -> Self {
        Self::V6(a.octets())
    }
}
impl From<std::net::IpAddr> for IpAddr {
    fn from(a: std::net::IpAddr) -> Self {
        match a {
            std::net::IpAddr::V4(v) => v.into(),
            std::net::IpAddr::V6(v) => v.into(),
        }
    }
}

use core::fmt;

/// An Ethernet MAC address (EUI-48).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MacAddr(pub [u8; 6]);

impl MacAddr {
    pub const fn new(b: [u8; 6]) -> Self { Self(b) }
    pub const fn octets(self) -> [u8; 6] { self.0 }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g] = self.0;
        write!(f, "{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{g:02x}")
    }
}

/// An Ethernet MAC address (EUI-64 / MAC-48 extended).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MacAddr8(pub [u8; 8]);

impl MacAddr8 {
    pub const fn new(b: [u8; 8]) -> Self { Self(b) }
    pub const fn octets(self) -> [u8; 8] { self.0 }
}

impl fmt::Display for MacAddr8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g, h, i] = self.0;
        write!(f, "{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{g:02x}:{h:02x}:{i:02x}")
    }
}

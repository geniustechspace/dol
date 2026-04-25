//! Network address types: IP addresses and MAC addresses.
//!
//! # Ergonomic factories
//!
//! ```rust,no_run
//! use dol_types::network;
//!
//! let ip4  = network::ipv4(192, 168, 1, 1);             // Value::Inet
//! let mac  = network::mac([0xde, 0xad, 0xbe, 0xef, 0x00, 0x01]); // Value::MacAddr
//! ```

pub mod ip;
pub mod mac;

pub use ip::IpAddr;
pub use mac::{MacAddr, MacAddr8};

use super::value::Value;

// ─── Factory functions ────────────────────────────────────────────────────────

/// Constructs a `Value::Inet` from four IPv4 octets.
pub const fn ipv4(a: u8, b: u8, c: u8, d: u8) -> Value {
    Value::Inet(IpAddr::v4(a, b, c, d))
}

/// Constructs a `Value::Inet` from a 16-byte IPv6 address.
pub const fn ipv6(bytes: [u8; 16]) -> Value {
    Value::Inet(IpAddr::v6(bytes))
}

/// Constructs a `Value::MacAddr` from a 6-byte EUI-48 address.
pub const fn mac(bytes: [u8; 6]) -> Value {
    Value::MacAddr(MacAddr::new(bytes))
}

/// Constructs a `Value::MacAddr8` from an 8-byte EUI-64 address.
pub const fn mac8(bytes: [u8; 8]) -> Value {
    Value::MacAddr8(MacAddr8::new(bytes))
}

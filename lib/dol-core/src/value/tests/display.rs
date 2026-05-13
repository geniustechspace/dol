//! Display tests for [`Value`] and embedded payload types.

use crate::binary::BitString;
use crate::value::Value;
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec;

#[test]
fn bitstring_construction_and_display() {
    let bs = BitString::try_new(8, vec![0b1010_1010].into_boxed_slice()).unwrap();
    let v = Value::BitString(Box::new(bs));
    assert_eq!(v.to_string(), "b'10101010'");
}

#[test]
fn bitstring_length_mismatch_rejected() {
    let err = BitString::try_new(9, vec![0u8].into_boxed_slice()).unwrap_err();
    assert!(matches!(
        err,
        crate::TypeError::BitLengthMismatch {
            declared: 9,
            byte_count: 1
        }
    ));
}

#[cfg(feature = "network")]
#[test]
fn macaddr_display() {
    use crate::network::MacAddr;
    let mac = MacAddr::eui48([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]);
    assert_eq!(mac.to_string(), "00:1a:2b:3c:4d:5e");
}

#[cfg(feature = "network")]
#[test]
fn macaddr_eui64_display() {
    use crate::network::MacAddr;
    let mac = MacAddr::eui64([0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x70]);
    assert_eq!(mac.to_string(), "00:1a:2b:3c:4d:5e:6f:70");
}

#[cfg(feature = "geo")]
#[test]
fn point_display() {
    use crate::geo::Point;
    let p = Point::new_unchecked(1.5, 2.5);
    assert_eq!(p.to_string(), "(1.5,2.5)");
}

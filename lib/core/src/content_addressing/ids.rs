use core::fmt;
use core::marker::PhantomData;

/// A typed deterministic content identifier.
///
/// `BITS` is the digest width in bits. It must be divisible by 8.
/// `Tag` separates identifiers for different DOL domains at compile time.
#[repr(transparent)]
pub struct ContentId<const BITS: usize, Tag: ?Sized> {
    bytes: [u8; BITS / 8],
    marker: PhantomData<fn() -> Tag>,
}

pub type ContentId64<Tag> = ContentId<64, Tag>;
pub type ContentId128<Tag> = ContentId<128, Tag>;
pub type ContentId160<Tag> = ContentId<160, Tag>;
pub type ContentId256<Tag> = ContentId<256, Tag>;

impl<const BITS: usize, Tag: ?Sized> ContentId<BITS, Tag> {
    pub const BYTE_LEN: usize = BITS / 8;

    pub const fn from_bytes(bytes: [u8; BITS / 8]) -> Self {
        Self {
            bytes,
            marker: PhantomData,
        }
    }

    pub const fn as_bytes(&self) -> &[u8; BITS / 8] {
        &self.bytes
    }

    pub const fn into_bytes(self) -> [u8; BITS / 8] {
        self.bytes
    }

    pub const fn bit_len(&self) -> usize {
        BITS
    }

    pub const fn byte_len(&self) -> usize {
        BITS / 8
    }
}

impl<const BITS: usize, Tag: ?Sized> Clone for ContentId<BITS, Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const BITS: usize, Tag: ?Sized> Copy for ContentId<BITS, Tag> {}

impl<const BITS: usize, Tag: ?Sized> PartialEq for ContentId<BITS, Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<const BITS: usize, Tag: ?Sized> Eq for ContentId<BITS, Tag> {}

impl<const BITS: usize, Tag: ?Sized> PartialOrd for ContentId<BITS, Tag> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const BITS: usize, Tag: ?Sized> Ord for ContentId<BITS, Tag> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl<const BITS: usize, Tag: ?Sized> core::hash::Hash for ContentId<BITS, Tag> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<const BITS: usize, Tag: ?Sized> fmt::Debug for ContentId<BITS, Tag> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ContentId<")?;
        fmt::Display::fmt(&BITS, formatter)?;
        formatter.write_str(">(")?;
        write_hex(&self.bytes, formatter)?;
        formatter.write_str(")")
    }
}

impl<const BITS: usize, Tag: ?Sized> fmt::Display for ContentId<BITS, Tag> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(&self.bytes, formatter)
    }
}

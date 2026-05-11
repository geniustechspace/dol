use super::{ContentId, EncodeError, Kind};

/// Destination for canonical bytes.
///
/// Implementations may hash, count, store, or forward bytes. Encoders should
/// stream into this trait instead of allocating a `Vec<u8>`.
pub trait CanonicalSink {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError>;

    fn write_u8(&mut self, value: u8) -> Result<(), EncodeError> {
        self.write_bytes(&[value])
    }

    fn write_u16(&mut self, value: u16) -> Result<(), EncodeError> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn write_u32(&mut self, value: u32) -> Result<(), EncodeError> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn write_u64(&mut self, value: u64) -> Result<(), EncodeError> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn write_usize(&mut self, value: usize) -> Result<(), EncodeError> {
        let value = u64::try_from(value).map_err(|_| EncodeError::ObjectTooLarge)?;
        self.write_u64(value)
    }

    fn write_len(&mut self, len: usize) -> Result<(), EncodeError> {
        self.write_usize(len)
    }

    fn write_str(&mut self, value: &str) -> Result<(), EncodeError> {
        self.write_len(value.len())?;
        self.write_bytes(value.as_bytes())
    }

    fn write_blob(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.write_len(bytes.len())?;
        self.write_bytes(bytes)
    }
}

pub struct CountingSink {
    byte_count: u64,
}

impl CountingSink {
    pub const fn new() -> Self {
        Self { byte_count: 0 }
    }

    pub const fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

impl Default for CountingSink {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalSink for CountingSink {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let len = u64::try_from(bytes.len()).map_err(|_| EncodeError::ObjectTooLarge)?;
        self.byte_count = self
            .byte_count
            .checked_add(len)
            .ok_or(EncodeError::ObjectTooLarge)?;
        Ok(())
    }
}

pub struct HashSink<const BITS: usize> {
    hasher: crate::hash::DomainSeparatedHasher,
}

impl<const BITS: usize> HashSink<BITS> {
    pub fn new(kind: Kind, version: u16) -> Result<Self, EncodeError> {
        let mut hasher = crate::hash::DomainSeparatedHasher::new(b"DOL-CONTENT-ADDRESS-V1");
        hasher.update_u16(kind.stable_u16());
        hasher.update_u16(version);
        hasher.update_u16(u16::try_from(BITS).map_err(|_| EncodeError::ObjectTooLarge)?);
        Ok(Self { hasher })
    }
}

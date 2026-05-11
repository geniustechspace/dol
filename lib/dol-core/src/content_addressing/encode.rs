use super::{CanonicalSink, ContentId, EncodeError, HashSink, Kind};
use crate::budget::Budget;

/// Deterministic byte encoding for content identity.
///
/// This is intentionally separate from serde and wire encoding. Wire encoding
/// may include transport concerns; canonical encoding represents identity.
pub trait CanonicalEncode {
    const KIND: Kind;
    const VERSION: u16;

    fn encode<S: CanonicalSink>(
        &self,
        sink: &mut S,
        budget: &mut Budget,
    ) -> Result<(), EncodeError>;
}

/// Convenience trait for values that can produce typed content identifiers.
pub trait Addressable: CanonicalEncode {
    type Tag: ?Sized;

    fn content_id<const BITS: usize>(
        &self,
        budget: &mut Budget,
    ) -> Result<ContentId<BITS, Self::Tag>, EncodeError> {
        let mut sink = HashSink::<BITS>::new(Self::KIND, Self::VERSION)?;
        self.encode(&mut sink, budget)?;
        sink.finish()
    }

    fn content_id_128(
        &self,
        budget: &mut Budget,
    ) -> Result<ContentId<128, Self::Tag>, EncodeError> {
        self.content_id::<128>(budget)
    }

    fn content_id_256(
        &self,
        budget: &mut Budget,
    ) -> Result<ContentId<256, Self::Tag>, EncodeError> {
        self.content_id::<256>(budget)
    }
}

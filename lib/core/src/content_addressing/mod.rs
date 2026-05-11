//! Deterministic content addressing primitives for DOL.
//!
//! Public names are explicit where they cross module boundaries, but compact
//! where the module already provides context.

mod any_id;
mod encode;
mod error;
mod ids;
mod kind;
mod sink;
mod tags;

pub use any_id::AnyContentId;
pub use encode::{Addressable, CanonicalEncode};
pub use error::{ContentAddressError, EncodeError};
pub use id::{ContentId, ContentId64, ContentId128, ContentId160, ContentId256};
pub use kind::Kind;
pub use sink::{CanonicalSink, CountingSink, HashSink};
pub use tags::{
    BlobTag, ExpressionArenaTag, ExpressionTag, ProgramTag, SchemaTag, StreamTag, WireEnvelopeTag,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Budget;

    struct TestTag;

    struct TestValue<'a> {
        text: &'a str,
    }

    impl CanonicalEncode for TestValue<'_> {
        const KIND: Kind = Kind::Utf8;
        const VERSION: u16 = 1;

        fn encode<S: CanonicalSink>(
            &self,
            sink: &mut S,
            _budget: &mut Budget,
        ) -> Result<(), EncodeError> {
            sink.write_str(self.text)
        }
    }

    impl Addressable for TestValue<'_> {
        type Tag = TestTag;
    }

    #[test]
    fn same_content_produces_same_content_id() {
        let mut left_budget = Budget::default();
        let mut right_budget = Budget::default();

        let left = TestValue { text: "hello" };
        let right = TestValue { text: "hello" };

        let left_id = left.content_id_128(&mut left_budget).unwrap();
        let right_id = right.content_id_128(&mut right_budget).unwrap();

        assert_eq!(left_id, right_id);
    }

    #[test]
    fn different_content_produces_different_content_id() {
        let mut left_budget = Budget::default();
        let mut right_budget = Budget::default();

        let left = TestValue { text: "hello" };
        let right = TestValue { text: "world" };

        let left_id = left.content_id_128(&mut left_budget).unwrap();
        let right_id = right.content_id_128(&mut right_budget).unwrap();

        assert_ne!(left_id, right_id);
    }
}

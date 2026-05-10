//! [`Internable`](dol_core::intern::Internable) impls for the
//! `dol-expr` arena payload types.
//!
//! Mirrors `dol_core::intern_literal` for arena-side payloads. Each
//! impl owns the canonical byte form of one type so the arena's
//! `intern_*` deduplication paths share a single, reviewer-friendly
//! specification rather than re-implementing structural hashing per
//! pool.
//!
//! See [`dol_core::intern`] for the underlying contract and the
//! per-`feed` canonicalisation rules every impl must uphold.

use dol_core::hash::Hasher;
use dol_core::intern::Internable;

use crate::arena::{FieldNode, FieldStep};
use crate::ids::StrId;

// FieldStep variant tag bytes.
const STEP_KEY: u8 = 0x01;
const STEP_INDEX: u8 = 0x02;

// `Option<StrId>` discriminator bytes for `FieldNode::namespace`.
const NS_NONE: u8 = 0x00;
const NS_SOME: u8 = 0x01;

#[inline]
fn feed_str_id(h: &mut Hasher, id: StrId) {
    h.update(&id.get().to_le_bytes());
}

#[inline]
fn feed_len(h: &mut Hasher, len: usize) {
    h.update(&(len as u64).to_le_bytes());
}

impl Internable for FieldStep {
    fn feed(&self, h: &mut Hasher) {
        match self {
            FieldStep::Key(s) => {
                h.update(&[STEP_KEY]);
                feed_str_id(h, *s);
            }
            FieldStep::Index(i) => {
                h.update(&[STEP_INDEX]);
                h.update(&i.to_le_bytes());
            }
        }
    }
}

impl Internable for FieldNode {
    fn feed(&self, h: &mut Hasher) {
        // Namespace presence is part of the canonical form so a leaf
        // with `Some(ns)` cannot collide with one whose name is
        // pre-encoded in the leaf id.
        match self.namespace {
            None => {
                h.update(&[NS_NONE]);
            }
            Some(ns) => {
                h.update(&[NS_SOME]);
                feed_str_id(h, ns);
            }
        }
        feed_str_id(h, self.name);
        feed_len(h, self.steps.len());
        for step in &self.steps {
            step.feed(h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::StrTag;
    use dol_core::id::Id;
    use smallvec::smallvec;

    enum FieldTag {}

    fn sid(raw: u32) -> Id<StrTag> {
        Id::from_u32(raw).unwrap()
    }

    #[test]
    fn identical_unanchored_leaves_share_id() {
        let a: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(42),
            steps: smallvec![],
        }
        .content_id();
        let b: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(42),
            steps: smallvec![],
        }
        .content_id();
        assert_eq!(a, b);
    }

    #[test]
    fn namespace_changes_id() {
        let a: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(42),
            steps: smallvec![],
        }
        .content_id();
        let b: Id<FieldTag> = FieldNode {
            namespace: Some(sid(7)),
            name: sid(42),
            steps: smallvec![],
        }
        .content_id();
        assert_ne!(a, b);
    }

    #[test]
    fn step_order_changes_id() {
        let a: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(1),
            steps: smallvec![FieldStep::Key(sid(2)), FieldStep::Index(0)],
        }
        .content_id();
        let b: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(1),
            steps: smallvec![FieldStep::Index(0), FieldStep::Key(sid(2))],
        }
        .content_id();
        assert_ne!(a, b);
    }

    #[test]
    fn step_kind_disambiguates_same_payload() {
        // `Key(StrId(5))` and `Index(5)` must not collide even though
        // both carry the bit-pattern 5.
        let key = FieldStep::Key(sid(5));
        let index = FieldStep::Index(5);
        let ka: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(1),
            steps: smallvec![key],
        }
        .content_id();
        let kb: Id<FieldTag> = FieldNode {
            namespace: None,
            name: sid(1),
            steps: smallvec![index],
        }
        .content_id();
        assert_ne!(ka, kb);
    }
}

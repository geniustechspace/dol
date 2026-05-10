//! Unified content-addressed interning contract.
//!
//! `dol-core` defines the [`Internable`] trait so every consumer crate
//! (`dol-expr`, `dol-schema`, …) deduplicates its arena pools against a
//! single, shared specification rather than re-inventing per-type logic.
//!
//! # Why a single contract
//!
//! Historically only [`crate::hash`]-derived [`crate::id::Id`]s for
//! interned strings were content-addressed; every other side-pool
//! (literals, field references, composite literals, …) handed out
//! sequential `Vec` indices, so two structurally-equal inputs received
//! distinct ids. That broke plan-cache hits, defeated `==` on derived
//! handles, and forced every dedup site to grow its own ad-hoc hash
//! table.
//!
//! `Internable` formalises a one-method contract:
//!
//! 1. The implementor describes its **canonical byte form** by feeding
//!    bytes into a [`crate::hash::Hasher`].
//! 2. The provided [`Internable::content_id`] turns the digest into a
//!    typed [`crate::id::Id<Tag>`] via the niche-folding helper
//!    [`id_from_digest32`], producing the same id for any two values
//!    whose canonical bytes match — across processes, machines, and
//!    language bindings, exactly like `dol_expr::Interner`'s
//!    `StrId`s.
//!
//! # Canonicalisation responsibilities
//!
//! Implementors own the canonical-byte spec for their type. The trait
//! only requires *determinism*; meaningful invariants (NaN normalised
//! to a single bit pattern, `-0.0 == 0.0`, decimal scale collapsed,
//! key ordering for objects) live in each `feed` body and are
//! reviewed there, not at the call-site.
//!
//! Children that are themselves [`Internable`] feed themselves via
//! `child.feed(hasher)` so the per-type canonical form stays in one
//! place per type. (An optimisation that hashes the child's
//! pre-computed [`Internable::content_id`] instead — keeping `feed`
//! linear in the node and constant per child — is correct whenever
//! the parent's identity already encodes the child's identity, but
//! it is not enforced by the trait; pick whichever your pool's
//! lookup pattern needs.)
//!
//! # Feature gating
//!
//! Requires the `hash` cargo feature (the only place in the workspace
//! that pulls in `blake3`).

use crate::hash::{Digest32, Hasher};
use crate::id::Id;
use core::num::NonZeroU32;

/// Build an [`Id<Tag>`] from a 32-bit BLAKE3 prefix.
///
/// The all-zero digest is folded onto `1` so the result fits the
/// [`NonZeroU32`] niche backing [`Id`]. This introduces a single
/// artificial collision (the empty hash and `1` map to the same id) at
/// a one-in-2³² rate, mirroring the existing `dol_expr::Interner` rule
/// so every content-addressed id family in the workspace shares one
/// niche convention.
#[inline]
#[must_use]
pub fn id_from_digest32<Tag: ?Sized>(digest: Digest32) -> Id<Tag> {
    let raw = u32::from_le_bytes(digest);
    // SAFETY-equivalent: the `if h == 0 { 1 }` fold guarantees the
    // value is non-zero; `NonZeroU32::new` therefore always returns
    // `Some`. We surface the impossibility as a `match` rather than an
    // `expect` so the workspace `expect_used = "deny"` lint stays
    // satisfied without a per-call `#[allow]`.
    let nz = match NonZeroU32::new(if raw == 0 { 1 } else { raw }) {
        Some(nz) => nz,
        // The `if raw == 0 { 1 }` arm makes this branch unreachable;
        // we still pick a deterministic fallback (`NonZeroU32::MIN`)
        // rather than panic so the trait stays panic-free.
        None => NonZeroU32::MIN,
    };
    Id::new(nz)
}

/// Types whose identity is their content.
///
/// Implementors describe their canonical byte form in [`Internable::feed`];
/// the provided [`Internable::content_id`] turns that into a typed,
/// niche-folded [`Id<Tag>`]. Two values whose `feed` writes the same
/// bytes are guaranteed to receive the same id (modulo the documented
/// 1-in-2³² folded-zero collision).
///
/// See the module-level docs for the canonicalisation contract that
/// each `feed` body must uphold.
pub trait Internable {
    /// Feed the canonical bytes of `self` into `hasher`.
    ///
    /// Implementations must be deterministic for any two values that
    /// the type's owner considers structurally equal. Variants that
    /// share a layout MUST be disambiguated with a leading tag byte so
    /// `Foo::A(0)` and `Foo::B(0)` cannot collide.
    fn feed(&self, hasher: &mut Hasher);

    /// Compute the content-addressed id for `self`, tagged with `Tag`.
    ///
    /// The default implementation finalises a fresh hasher fed by
    /// [`feed`](Self::feed). Implementors typically do **not** override
    /// this; the indirection lets the workspace migrate to a different
    /// digest size (e.g. 64-bit ids) in one place if the 32-bit
    /// collision surface ever becomes uncomfortable.
    #[must_use]
    fn content_id<Tag: ?Sized>(&self) -> Id<Tag> {
        let mut h = Hasher::new();
        self.feed(&mut h);
        id_from_digest32(h.finalize32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Demo(&'static [u8]);

    impl Internable for Demo {
        fn feed(&self, h: &mut Hasher) {
            h.update(self.0);
        }
    }

    enum DemoTag {}

    #[test]
    fn equal_inputs_yield_equal_ids() {
        let a: Id<DemoTag> = Demo(b"hello").content_id();
        let b: Id<DemoTag> = Demo(b"hello").content_id();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_inputs_yield_distinct_ids() {
        let a: Id<DemoTag> = Demo(b"hello").content_id();
        let b: Id<DemoTag> = Demo(b"world").content_id();
        assert_ne!(a, b);
    }

    #[test]
    fn id_is_non_zero_for_zero_digest() {
        // The folded-zero rule guarantees the niche is always live.
        let id = id_from_digest32::<DemoTag>([0, 0, 0, 0]);
        assert_eq!(id.get(), 1);
    }
}

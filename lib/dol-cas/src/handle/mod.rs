//! Handle types — `Lid`, `Cid`, `Gid` plus the canonical tag and alias set.
//!
//! Per `dol-rewrite-plan-v2.md` §7.1–§7.2.
//!
//! - [`Lid<Tag>`] is the local (in-process) identifier — a 4-byte
//!   one-based handle into a pool. Re-exported from
//!   [`dol_core::ids::Id`] because the planned shape is structurally
//!   identical; aliasing avoids two copies of the same niche-optimised
//!   primitive.
//! - [`Cid<Tag>`] is the BLAKE3-128 content address (16 bytes,
//!   cross-process stable).
//! - [`Gid<Tag>`] is the BLAKE3-256 global address (32 bytes,
//!   cross-process stable, suitable for signed manifests).

pub mod tags;

mod cid;
mod gid;

pub use cid::Cid;
pub use gid::Gid;

/// Local (in-process) identifier. Alias for [`dol_core::ids::Id`].
///
/// `Lid` is the M2 name for the existing dol-core `Id<Tag>` primitive.
/// It is a 4-byte [`NonZeroU32`](core::num::NonZeroU32)-backed handle;
/// `Option<Lid<Tag>>` is also 4 bytes thanks to the niche.
///
/// Different `Tag` types produce structurally distinct `Lid` types, so
/// a `StrId` cannot be passed where a `NodeId` is expected.
pub type Lid<Tag> = dol_core::ids::Id<Tag>;

// ─── Canonical type aliases ──────────────────────────────────────────────────
//
// Each alias pairs a [`Lid`] / [`Cid`] / [`Gid`] with the zero-sized tag
// from [`tags`] that identifies which pool / arena it belongs to.

/// Interned-string handle. Issued by [`crate::string_pool::StringPool`].
pub type StrId = Lid<tags::StrTag>;
/// Expression-node handle. Issued by `dol_ir::ExprArena` (lands in M3).
pub type NodeId = Lid<tags::NodeTag>;
/// Field-chain handle.
pub type FieldId = Lid<tags::FieldTag>;
/// Entity-table handle.
pub type EntityId = Lid<tags::EntityTag>;
/// Interned-literal handle.
pub type LiteralId = Lid<tags::LiteralTag>;
/// Function / opcode handle.
pub type FuncId = Lid<tags::FuncTag>;

/// BLAKE3-128 content address of an interned string.
pub type StrCid = Cid<tags::StrTag>;
/// BLAKE3-128 content address of a schema.
pub type SchemaCid = Cid<tags::SchemaTag>;
/// BLAKE3-128 content address of a program.
pub type ProgramCid = Cid<tags::ProgramTag>;
/// BLAKE3-256 global identity of a program (for signed manifests).
pub type ProgramGid = Gid<tags::ProgramTag>;

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn lid_option_is_4_bytes() {
        // The whole point of the NonZeroU32 niche.
        assert_eq!(size_of::<Option<StrId>>(), 4);
        assert_eq!(size_of::<Option<NodeId>>(), 4);
    }

    #[test]
    fn cid_is_16_bytes() {
        assert_eq!(size_of::<StrCid>(), 16);
        assert_eq!(size_of::<Cid<tags::SchemaTag>>(), 16);
    }

    #[test]
    fn gid_is_32_bytes() {
        assert_eq!(size_of::<ProgramGid>(), 32);
    }

    #[test]
    fn cid_from_bytes_round_trip() {
        let raw = [0x11; 16];
        let cid: StrCid = Cid::from_bytes(raw);
        assert_eq!(cid.as_bytes(), &raw);
        assert_eq!(cid.into_bytes(), raw);
    }

    #[test]
    fn gid_from_bytes_round_trip() {
        let raw = [0x22; 32];
        let gid: ProgramGid = Gid::from_bytes(raw);
        assert_eq!(gid.as_bytes(), &raw);
        assert_eq!(gid.into_bytes(), raw);
    }

    #[test]
    fn cid_equality_and_ordering() {
        let a: StrCid = Cid::from_bytes([0; 16]);
        let b: StrCid = Cid::from_bytes([1; 16]);
        assert!(a < b);
        assert_eq!(a, Cid::<tags::StrTag>::from_bytes([0; 16]));
    }
}

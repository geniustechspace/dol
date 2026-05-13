//! [`StringResolver`] for [`StringPool`] — closes the two-mode path bridge.
//!
//! Per `dol-rewrite-plan-v2.md` §7.5. The plan calls for
//! `impl PathSegment for Lid<StrTag>` with `Resolver = StringPool`,
//! but the orphan rule forbids that impl from living in `dol-cas`
//! (both `PathSegment` and `Id` are foreign here). Instead,
//! `dol-core` ships `impl PathSegment for StrId` with
//! `Resolver = dyn StringResolver`, and this file plugs `StringPool`
//! into that resolver slot.
//!
//! The end-to-end shape the plan calls for is preserved: a
//! `Path<StrId>` resolves zero-copy against any `&StringPool`, since
//! [`StringPool::get`] borrows directly into the slot's stable
//! per-allocation byte storage.

use dol_core::path::StringResolver;
use dol_core::strings::StrId;

use crate::string_pool::StringPool;

impl StringResolver for StringPool {
    #[inline]
    fn resolve_str(&self, id: StrId) -> Option<&str> {
        self.get(id)
    }

    #[inline]
    fn content_id_for(&self, id: StrId) -> Option<[u8; 16]> {
        self.to_cid(id).map(|cid| *cid.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use dol_core::path::{Path, PathSegment};

    #[test]
    fn resolve_str_matches_pool_get() {
        let pool = StringPool::standard();
        let id = pool.intern("users").unwrap();
        let via_trait = StringResolver::resolve_str(&pool, id);
        assert_eq!(via_trait, Some("users"));
        assert_eq!(via_trait, pool.get(id));
    }

    #[test]
    fn unknown_handle_via_path_segment_resolves_to_empty_string() {
        let pool = StringPool::standard();
        // Synthesise a handle that points past every interned slot.
        let bogus: StrId = crate::handle::Lid::from_index(9999).unwrap();
        let resolver: &(dyn StringResolver + 'static) = &pool;
        // Via the PathSegment trait directly — exercises the dyn
        // dispatch through the resolver slot.
        assert_eq!(<StrId as PathSegment>::resolve(&bogus, resolver), "");
    }

    #[test]
    fn content_id_for_matches_to_cid() {
        let pool = StringPool::standard();
        let id = pool.intern("payload").unwrap();
        let via_trait = StringResolver::content_id_for(&pool, id).unwrap();
        let from_pool = *pool.to_cid(id).unwrap().as_bytes();
        assert_eq!(via_trait, from_pool);
    }

    #[test]
    fn path_of_strid_resolves_target_and_fields_against_pool() {
        // The whole point of closing the bridge: `Path<StrId>` walks
        // through the same `PathSegment` machinery as `Path<Name>`,
        // resolving zero-copy against the pool that issued its IDs.
        let pool = StringPool::standard();
        let users = pool.intern("users").unwrap();
        let profile = pool.intern("profile").unwrap();
        let email = pool.intern("email").unwrap();

        let path: Path<StrId> = Path::from_segment(users).get(profile).get(email);
        let resolver: &(dyn StringResolver + 'static) = &pool;

        assert_eq!(path.resolve_target(resolver), "users");
        let fields: alloc::vec::Vec<&str> = path.resolve_fields(resolver).collect();
        assert_eq!(fields, alloc::vec!["profile", "email"]);
    }

    #[test]
    fn path_of_strid_namespace_resolves_against_pool() {
        let pool = StringPool::standard();
        let api = pool.intern("api").unwrap();
        let target = pool.intern("orders").unwrap();
        let path: Path<StrId> = Path::from_segment(target).namespace(api);
        let resolver: &(dyn StringResolver + 'static) = &pool;
        assert_eq!(path.resolve_namespace(resolver), Some("api"));
        assert_eq!(path.resolve_target(resolver), "orders");
    }

    #[test]
    fn dol_cas_strid_is_dol_core_strid() {
        // The `StrTag` re-export means `dol_cas::handle::StrId` and
        // `dol_core::strings::StrId` are the same type, so handles
        // issued by `StringPool` flow into anywhere a `dol-core`
        // `StrId` is expected.
        let pool = StringPool::standard();
        let issued: crate::handle::StrId = pool.intern("x").unwrap();
        let _as_core: dol_core::strings::StrId = issued;
    }
}

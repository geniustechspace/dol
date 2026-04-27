//! Small helpers shared across `dol-query` builders for building
//! [`Target`](dol_ir::Target)s, [`Symbol`](dol_ir::Symbol)s, and
//! [`Locator`](dol_ir::Locator)s from owned name/namespace strings.

use dol_expr::Interner;
use dol_ir::{Locator, SchemaBinding, Symbol, Target, TargetKind};
use smallvec::smallvec;

/// Convert an owned `&str` into an interned [`Symbol`].
#[inline]
pub fn intern_symbol(interner: &mut Interner, s: &str) -> Symbol {
    Symbol::new(interner.intern(s))
}

/// Build a [`Locator`] from a name and an optional namespace.
///
/// `name` is interned into [`Locator::name`]. When provided, `namespace`
/// is interned as a single [`Symbol`] and stored in [`Locator::namespace`]
/// — this preserves the dotted form (e.g. `"public.audit"`) verbatim
/// rather than splitting it into [`Locator::path`] segments. Builders that
/// need structured paths (S3 keys, file-tree segments, topic partitions)
/// should populate `Locator::path` explicitly via
/// [`Locator::with_segment`](dol_ir::Locator::with_segment).
pub fn locator_from_parts(interner: &mut Interner, name: &str, namespace: Option<&str>) -> Locator {
    let ns_sym = namespace
        .filter(|s| !s.is_empty())
        .map(|ns| intern_symbol(interner, ns));
    Locator {
        namespace: ns_sym,
        name: intern_symbol(interner, name),
        path: smallvec![],
    }
}

/// Build a [`Target`] for the given [`TargetKind`] addressed by name +
/// optional namespace, with [`SchemaBinding::Inferred`] (the conservative
/// default for builders).
pub fn target_from_parts(
    interner: &mut Interner,
    kind: TargetKind,
    name: &str,
    namespace: Option<&str>,
) -> Target {
    Target {
        kind,
        locator: locator_from_parts(interner, name, namespace),
        alias: None,
        schema: SchemaBinding::Inferred,
    }
}

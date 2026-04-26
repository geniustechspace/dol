//! Versioning constants and envelope types for the v2 IR wire format.

/// Current `dol-ir` schema version.
///
/// `dol-wire` and any persistent serialisation should embed this version in
/// the wire envelope so a v1 payload can be detected and decoded through
/// [`crate::compat::statement`].
pub const IR_SCHEMA_VERSION: u32 = 2;

/// Versioned envelope for serialised programs.
///
/// `body` is opaque at this layer — `dol-wire` parameterises it over the
/// concrete program body type.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VersionedProgram<Body> {
    pub version: u32,
    pub body: Body,
}

impl<Body> VersionedProgram<Body> {
    /// Wrap a body with the current [`IR_SCHEMA_VERSION`].
    pub fn current(body: Body) -> Self {
        Self {
            version: IR_SCHEMA_VERSION,
            body,
        }
    }
}

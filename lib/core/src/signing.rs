//! Signing & verification trait shapes.
//!
//! `dol-core::signing` declares the *shape* of cryptographic signing inside
//! the DOL stack without picking an algorithm. Concrete implementations
//! (Ed25519, ECDSA-P256, HMAC-SHA-256, …) live in downstream crates or in
//! application code that wires DOL into a host environment.
//!
//! The module is intentionally minimal:
//!
//! - [`Signer`] produces a signature over a byte slice. The signature is
//!   borrowed-or-owned via the associated [`Signer::Signature`] type so
//!   embedded implementations can return a stack array (`[u8; 64]`) and
//!   server implementations can return a `Vec<u8>`.
//! - [`Verifier`] accepts an arbitrary signature byte slice and validates it
//!   against a message. Verification can fail for any reason
//!   ([`VerifyError`]); concrete failure modes are intentionally opaque so
//!   implementations can avoid timing oracles.
//! - Both traits are `?Sized`-friendly (`&dyn Signer`) and `no_std + alloc`.
//!
//! The traits do not bake in a key-material story: implementations decide
//! whether a [`Signer`] holds a private key, talks to a TPM, an HSM, a
//! remote KMS, or just signs with a fixed test vector.
//!
//! # Example
//!
//! ```
//! use dol_core::signing::{Signer, Verifier, VerifyError};
//!
//! struct ConstSigner;
//! impl Signer for ConstSigner {
//!     type Signature = [u8; 4];
//!     type Error = core::convert::Infallible;
//!     fn sign(&self, _msg: &[u8]) -> Result<Self::Signature, Self::Error> {
//!         Ok(*b"SIGN")
//!     }
//! }
//!
//! struct ConstVerifier;
//! impl Verifier for ConstVerifier {
//!     fn verify(&self, _msg: &[u8], sig: &[u8]) -> Result<(), VerifyError> {
//!         if sig == b"SIGN" { Ok(()) } else { Err(VerifyError::Mismatch) }
//!     }
//! }
//!
//! let s = ConstSigner.sign(b"hello").unwrap();
//! ConstVerifier.verify(b"hello", &s).unwrap();
//! ```

use core::fmt;

/// Produces a signature over a message.
///
/// The associated [`Signature`](Signer::Signature) type is `AsRef<[u8]>` so
/// callers can hand the raw bytes to a [`Verifier`] without committing to a
/// particular byte layout. Returning `[u8; N]` keeps embedded implementations
/// allocation-free; returning `alloc::vec::Vec<u8>` lets server-side
/// implementations support variable-length signatures.
pub trait Signer {
    /// The signature type produced by [`Signer::sign`]. Typically `[u8; N]`
    /// for fixed-size schemes (Ed25519: `[u8; 64]`) or `Vec<u8>` for
    /// variable-length schemes.
    type Signature: AsRef<[u8]>;

    /// Signing failure. Implementations that cannot fail set this to
    /// [`core::convert::Infallible`].
    type Error: fmt::Debug;

    /// Sign `msg`, returning either an opaque signature or an
    /// implementation-specific error.
    ///
    /// The returned bytes are passed verbatim to [`Verifier::verify`]; the
    /// shape and length are an implementation detail of the signing scheme.
    fn sign(&self, msg: &[u8]) -> Result<Self::Signature, Self::Error>;
}

/// Verifies a signature against a message.
///
/// `verify` is intentionally `Result<(), VerifyError>`-shaped (rather than
/// returning `bool`) so implementations cannot leak the shape of the failure
/// through the return type and so callers cannot ignore the result.
pub trait Verifier {
    /// Validate that `sig` is a correct signature over `msg` under this
    /// verifier's key. Implementations should be constant-time with respect
    /// to the signature contents where the underlying primitive supports it.
    fn verify(&self, msg: &[u8], sig: &[u8]) -> Result<(), VerifyError>;
}

/// Reasons a [`Verifier`] may reject a signature.
///
/// The variants are intentionally coarse: leaking *why* a signature was
/// rejected is itself an information disclosure for many protocols, and
/// downstream code should generally treat any [`VerifyError`] as
/// authentication failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VerifyError {
    /// The signature did not authenticate the message under this verifier's
    /// key. This is the catch-all for cryptographic mismatch and should be
    /// indistinguishable from [`Self::Malformed`] to remote callers.
    Mismatch,
    /// The signature byte string was structurally invalid (wrong length,
    /// invalid encoding, …) and could not be parsed at all.
    Malformed,
    /// The verifier could not complete the check for an environmental reason
    /// (key unavailable, hardware token disconnected, …). Distinct from
    /// [`Self::Mismatch`] so callers can decide whether to retry.
    Unavailable,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Mismatch => "signature did not authenticate message",
            Self::Malformed => "signature was structurally invalid",
            Self::Unavailable => "verifier could not complete the check",
        };
        f.write_str(msg)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VerifyError {}

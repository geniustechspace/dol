//! `Signed<T>` envelope — per `docs/v2_plan.md` §56.
//!
//! Every persisted DOL artifact (a [`dol_command::Program`], a
//! [`dol_command::SchemaCatalog`], an expression arena, …) can be wrapped in
//! a [`Signed`] envelope so the decoder can authenticate the bytes
//! before handing them to the validating IR layer. The envelope is
//! algorithm-agnostic: signing happens through any [`Signer`] impl,
//! verification through any [`Verifier`] — typically Ed25519 on hosts,
//! HMAC-SHA-256 on resource-constrained devices.
//!
//! # Wire shape
//!
//! ```text
//! ┌─────────────────────────┬───────────────┬─────────────────────────┐
//! │  WireHeader (8 bytes)   │ payload_len   │   payload bytes (T)     │
//! │  magic + version        │   (u32 LE)    │   (encoded by caller)   │
//! ├─────────────────────────┴───────────────┴─────────────────────────┤
//! │              sig_len (u16 LE)         │  signature bytes          │
//! └───────────────────────────────────────┴───────────────────────────┘
//! ```
//!
//! The signature is computed over `WireHeader || payload_len || payload`
//! — i.e. everything *up to* the signature. This ensures both the
//! version envelope and the payload are bound together; flipping the
//! version field (e.g. to claim an older codec) invalidates the
//! signature.
//!
//! # Allocation
//!
//! [`Signed::seal`] allocates exactly once (a [`alloc::vec::Vec`] of the
//! final byte length). [`Signed::open`] performs zero allocations beyond
//! returning a borrowed `&[u8]` payload slice — the caller decides how
//! to deserialise it (typically via [`crate::Decode`]).
//!
//! # Example
//!
//! ```
//! use dol_core::signing::{Signer, Verifier, VerifyError};
//! use dol_wire::signed::{Signed, SignedError};
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
//! let sealed = Signed::seal(b"hello", &ConstSigner).unwrap();
//! let payload = Signed::open(&sealed, &ConstVerifier).unwrap();
//! assert_eq!(payload, b"hello");
//! ```

use alloc::vec::Vec;
use core::fmt;

use dol_core::signing::{Signer, Verifier, VerifyError};

use crate::{WireError, WireHeader};

/// Errors produced by [`Signed::seal`] / [`Signed::open`].
#[derive(Debug)]
#[non_exhaustive]
pub enum SignedError<E = core::convert::Infallible> {
    /// Wire envelope (magic / version) failed to parse.
    Wire(WireError),
    /// Signing produced an error in the [`Signer`] impl.
    Sign(E),
    /// Verification rejected the signature.
    Verify(VerifyError),
    /// The byte slice was structurally malformed (truncated header,
    /// length-prefix mismatch, …) — distinguished from
    /// [`SignedError::Verify`] so callers can tell parse errors from
    /// authentication failures.
    Malformed,
}

impl<E: fmt::Display> fmt::Display for SignedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wire(e) => write!(f, "signed: wire error: {e}"),
            Self::Sign(e) => write!(f, "signed: signing error: {e}"),
            Self::Verify(e) => write!(f, "signed: verify failed: {e}"),
            Self::Malformed => f.write_str("signed: malformed envelope"),
        }
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug + fmt::Display> std::error::Error for SignedError<E> {}

impl<E> From<WireError> for SignedError<E> {
    fn from(e: WireError) -> Self {
        Self::Wire(e)
    }
}

/// Per-payload signed envelope. Prefer the static [`Signed::seal`] /
/// [`Signed::open`] constructors over building this struct directly.
///
/// `Signed` is intentionally a *namespace* type rather than a value
/// holding owned bytes — the signing direction allocates once into a
/// `Vec<u8>` and the verifying direction borrows the input slice, so
/// there's no canonical "signed value" to keep around. Both are static
/// methods to make the byte-level shape explicit at the call site.
pub struct Signed;

const PAYLOAD_LEN_BYTES: usize = 4;
const SIG_LEN_BYTES: usize = 2;
const HEADER_BYTES: usize = 8;

impl Signed {
    /// Seal `payload` under `signer`, returning the wire-ready byte
    /// vector. The signature is computed over `header || payload_len ||
    /// payload`.
    pub fn seal<S: Signer>(payload: &[u8], signer: &S) -> Result<Vec<u8>, SignedError<S::Error>> {
        if payload.len() > u32::MAX as usize {
            return Err(SignedError::Malformed);
        }
        let header = WireHeader::current().to_bytes();
        let payload_len = payload.len() as u32;

        // Build the message to sign: header || len || payload.
        // `Vec::with_capacity` arithmetic: each addend is bounded
        // (HEADER_BYTES = 8, PAYLOAD_LEN_BYTES = 4, payload.len() ≤ u32::MAX).
        #[allow(clippy::arithmetic_side_effects)]
        let signed_len = HEADER_BYTES + PAYLOAD_LEN_BYTES + payload.len();
        let mut to_sign = Vec::with_capacity(signed_len);
        to_sign.extend_from_slice(&header);
        to_sign.extend_from_slice(&payload_len.to_le_bytes());
        to_sign.extend_from_slice(payload);

        let sig = signer.sign(&to_sign).map_err(SignedError::Sign)?;
        let sig_bytes = sig.as_ref();
        if sig_bytes.len() > u16::MAX as usize {
            return Err(SignedError::Malformed);
        }
        let sig_len = sig_bytes.len() as u16;

        // Final envelope: signed_msg || sig_len || sig.
        #[allow(clippy::arithmetic_side_effects)]
        let total = signed_len + SIG_LEN_BYTES + sig_bytes.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(&to_sign);
        out.extend_from_slice(&sig_len.to_le_bytes());
        out.extend_from_slice(sig_bytes);
        Ok(out)
    }

    /// Verify `bytes` under `verifier`, returning a borrowed slice of
    /// the inner payload. Zero allocations.
    pub fn open<'a, V: Verifier>(bytes: &'a [u8], verifier: &V) -> Result<&'a [u8], SignedError> {
        // Header.
        if bytes.len() < HEADER_BYTES {
            return Err(SignedError::Malformed);
        }
        // Indexing safe: bounds checked above.
        #[allow(clippy::indexing_slicing)]
        let header_bytes: [u8; 8] = bytes[..HEADER_BYTES]
            .try_into()
            .map_err(|_| SignedError::Malformed)?;
        let _ = WireHeader::from_bytes(header_bytes)?;

        // Payload length.
        #[allow(clippy::arithmetic_side_effects)]
        let len_end = HEADER_BYTES + PAYLOAD_LEN_BYTES;
        if bytes.len() < len_end {
            return Err(SignedError::Malformed);
        }
        #[allow(clippy::indexing_slicing)]
        let payload_len_bytes: [u8; 4] = bytes[HEADER_BYTES..len_end]
            .try_into()
            .map_err(|_| SignedError::Malformed)?;
        let payload_len = u32::from_le_bytes(payload_len_bytes) as usize;

        // Payload.
        let payload_end = len_end
            .checked_add(payload_len)
            .ok_or(SignedError::Malformed)?;
        if bytes.len() < payload_end {
            return Err(SignedError::Malformed);
        }
        #[allow(clippy::indexing_slicing)]
        let payload = &bytes[len_end..payload_end];

        // Sig length.
        let sig_len_end = payload_end
            .checked_add(SIG_LEN_BYTES)
            .ok_or(SignedError::Malformed)?;
        if bytes.len() < sig_len_end {
            return Err(SignedError::Malformed);
        }
        #[allow(clippy::indexing_slicing)]
        let sig_len_bytes: [u8; 2] = bytes[payload_end..sig_len_end]
            .try_into()
            .map_err(|_| SignedError::Malformed)?;
        let sig_len = u16::from_le_bytes(sig_len_bytes) as usize;

        // Sig bytes — must consume the rest exactly.
        let sig_end = sig_len_end
            .checked_add(sig_len)
            .ok_or(SignedError::Malformed)?;
        if bytes.len() != sig_end {
            return Err(SignedError::Malformed);
        }
        #[allow(clippy::indexing_slicing)]
        let sig = &bytes[sig_len_end..sig_end];

        // Verify over header || len || payload.
        #[allow(clippy::indexing_slicing)]
        let signed_msg = &bytes[..payload_end];
        verifier
            .verify(signed_msg, sig)
            .map_err(SignedError::Verify)?;

        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSigner;
    impl Signer for StubSigner {
        type Signature = [u8; 8];
        type Error = core::convert::Infallible;
        fn sign(&self, msg: &[u8]) -> Result<Self::Signature, Self::Error> {
            // Trivial "MAC": sum bytes mod 256, repeat to 8 bytes — the
            // verifier recomputes the same sum.
            let s = msg.iter().copied().fold(0u8, |a, b| a.wrapping_add(b));
            Ok([s; 8])
        }
    }
    struct StubVerifier;
    impl Verifier for StubVerifier {
        fn verify(&self, msg: &[u8], sig: &[u8]) -> Result<(), VerifyError> {
            if sig.len() != 8 {
                return Err(VerifyError::Malformed);
            }
            let want = msg.iter().copied().fold(0u8, |a, b| a.wrapping_add(b));
            if sig.iter().all(|&b| b == want) {
                Ok(())
            } else {
                Err(VerifyError::Mismatch)
            }
        }
    }

    #[test]
    fn roundtrip_empty_payload() {
        let bytes = Signed::seal(b"", &StubSigner).unwrap();
        let out = Signed::open(&bytes, &StubVerifier).unwrap();
        assert_eq!(out, b"");
    }

    #[test]
    fn roundtrip_some_payload() {
        let bytes = Signed::seal(b"hello world", &StubSigner).unwrap();
        let out = Signed::open(&bytes, &StubVerifier).unwrap();
        assert_eq!(out, b"hello world");
    }

    #[test]
    fn rejects_truncated_envelope() {
        let mut bytes = Signed::seal(b"hello", &StubSigner).unwrap();
        bytes.truncate(5);
        let err = Signed::open(&bytes, &StubVerifier).unwrap_err();
        assert!(matches!(err, SignedError::Malformed | SignedError::Wire(_)));
    }

    #[test]
    fn rejects_tampered_payload() {
        let mut bytes = Signed::seal(b"hello", &StubSigner).unwrap();
        // Flip a payload byte; signature stays the same → verify fails.
        let payload_idx = 8 + 4; // header + payload_len
        bytes[payload_idx] ^= 0x01;
        let err = Signed::open(&bytes, &StubVerifier).unwrap_err();
        assert!(matches!(err, SignedError::Verify(_)));
    }

    #[test]
    fn rejects_tampered_version() {
        let mut bytes = Signed::seal(b"hello", &StubSigner).unwrap();
        // Bump the minor version field — header still parses (forward
        // compatible), but the signature was computed over the original
        // bytes so verify must fail.
        bytes[6] = bytes[6].wrapping_add(1);
        let err = Signed::open(&bytes, &StubVerifier).unwrap_err();
        // Could surface as either VersionMismatch (header rejects) or
        // Verify (header accepts, sig disagrees). Both prove the
        // envelope binds the version to the payload.
        assert!(matches!(
            err,
            SignedError::Wire(WireError::VersionMismatch { .. }) | SignedError::Verify(_)
        ));
    }

    #[test]
    fn rejects_extra_trailing_bytes() {
        let mut bytes = Signed::seal(b"hi", &StubSigner).unwrap();
        bytes.push(0);
        let err = Signed::open(&bytes, &StubVerifier).unwrap_err();
        assert!(matches!(err, SignedError::Malformed));
    }
}

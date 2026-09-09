//! UKS, identity misbinding, and X25519 reflection checks.

use crate::crypto::network::digest::sha384;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::types::StrongDigest;

/// Reject a peer X25519 public that equals the local public (reflection).
pub fn reject_reflected_share(
    local_pk: &[u8; 32],
    peer_pk: &[u8; 32],
) -> Result<(), CryptoError> {
    if local_pk == peer_pk {
        Err(CryptoError::Unauthorized)
    } else {
        Ok(())
    }
}

/// SHA-384(`initiator || responder`). Zero or equal identities are Unauthorized.
pub fn identity_binding(
    initiator: &StrongDigest,
    responder: &StrongDigest,
) -> Result<StrongDigest, CryptoError> {
    if initiator.is_zero() || responder.is_zero() || initiator == responder {
        return Err(CryptoError::Unauthorized);
    }
    let mut concat = [0u8; 96];
    concat[..48].copy_from_slice(&initiator.0);
    concat[48..].copy_from_slice(&responder.0);
    Ok(sha384(&concat))
}

fn contains_window(haystack: &[u8], needle: &[u8; 48]) -> bool {
    if haystack.len() < 48 {
        return false;
    }
    let mut i = 0usize;
    while i + 48 <= haystack.len() {
        if &haystack[i..i + 48] == needle.as_slice() {
            return true;
        }
        i += 1;
    }
    false
}

/// Transcript must already contain both identity digests as 48-byte windows.
pub fn require_identity_binding(
    transcript: &Transcript,
    initiator: &StrongDigest,
    responder: &StrongDigest,
) -> Result<(), CryptoError> {
    if initiator.is_zero() || responder.is_zero() || initiator.0 == responder.0 {
        return Err(CryptoError::Unauthorized);
    }
    let bytes = transcript.as_bytes();
    if !contains_window(bytes, &initiator.0) || !contains_window(bytes, &responder.0) {
        return Err(CryptoError::Unauthorized);
    }
    Ok(())
}

/// Fail closed on zero identities, equal roles, or missing transcript binding.
pub fn require_bound_identities(
    transcript: &Transcript,
    initiator: &StrongDigest,
    responder: &StrongDigest,
) -> Result<(), CryptoError> {
    let _ = identity_binding(initiator, responder)?;
    require_identity_binding(transcript, initiator, responder)
}

/// Unknown-key-share: claimed peer must match the transcript-bound peer.
pub fn reject_unknown_key_share(
    claimed_peer: StrongDigest,
    transcript_bound_peer: StrongDigest,
) -> Result<(), CryptoError> {
    if claimed_peer.is_zero() || transcript_bound_peer.is_zero() {
        return Err(CryptoError::Unauthorized);
    }
    if claimed_peer != transcript_bound_peer {
        Err(CryptoError::Unauthorized)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::kem::MlKem768Secret;
    use crate::net::qdnf::crypto::handshake::{initiator_share, responder_complete};

    fn digest_fill(b: u8) -> StrongDigest {
        StrongDigest([b; 48])
    }

    #[test]
    fn reflected_x25519_publics_are_unauthorized() {
        let pk = [0x11u8; 32];
        assert_eq!(
            reject_reflected_share(&pk, &pk),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(reject_reflected_share(&[0x11u8; 32], &[0x22u8; 32]), Ok(()));

        let (_sk, kem_pk) = MlKem768Secret::generate().unwrap();
        let same = [11u8; 32];
        let ishare = initiator_share(&same, &kem_pk).unwrap();
        match responder_complete(&ishare, &same) {
            Err(e) => assert_eq!(e, CryptoError::Unauthorized),
            Ok(_) => panic!("reflected X25519 share was accepted"),
        }
    }

    #[test]
    fn swapped_identities_produce_different_binding_digests() {
        let initiator = digest_fill(0xA1);
        let responder = digest_fill(0xB2);
        let fwd = identity_binding(&initiator, &responder).unwrap();
        let rev = identity_binding(&responder, &initiator).unwrap();
        assert_ne!(fwd, rev);
        assert_eq!(
            identity_binding(&StrongDigest::ZERO, &responder),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(
            identity_binding(&initiator, &initiator),
            Err(CryptoError::Unauthorized)
        );
    }

    #[test]
    fn require_identity_binding_fails_if_transcript_lacks_initiator() {
        let initiator = digest_fill(0x11);
        let responder = digest_fill(0x22);
        let mut missing = Transcript::new();
        missing.append(b"responder", &responder.0).unwrap();
        assert_eq!(
            require_identity_binding(&missing, &initiator, &responder),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(
            require_bound_identities(&missing, &initiator, &responder),
            Err(CryptoError::Unauthorized)
        );

        let mut bound = Transcript::new();
        bound.append(b"initiator", &initiator.0).unwrap();
        bound.append(b"responder", &responder.0).unwrap();
        assert_eq!(
            require_identity_binding(&bound, &initiator, &responder),
            Ok(())
        );
        assert_eq!(
            require_bound_identities(&bound, &initiator, &responder),
            Ok(())
        );
        assert_eq!(
            require_bound_identities(&bound, &StrongDigest::ZERO, &responder),
            Err(CryptoError::Unauthorized)
        );
    }

    #[test]
    fn reject_unknown_key_share_mismatch_is_unauthorized() {
        let claimed = digest_fill(0x31);
        let bound = digest_fill(0x32);
        assert_eq!(
            reject_unknown_key_share(claimed, bound),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(reject_unknown_key_share(claimed, claimed), Ok(()));
        assert_eq!(
            reject_unknown_key_share(StrongDigest::ZERO, claimed),
            Err(CryptoError::Unauthorized)
        );
    }
}

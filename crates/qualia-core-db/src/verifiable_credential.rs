//! Verifiable Credentials — native issue / verify core (#19).
//!
//! A credential = an ISSUER (an agent), a SUBJECT (an agent), a set of claim quins, and
//! a validity window, sealed with an Ed25519 signature over a canonical SHA-256 digest of
//! those fields. This is the NATIVE proof (fast, fits the engine); a W3C JSON-LD Data
//! Integrity export is future work, and the lineage is the W3C Verifiable Claims WG.
//!
//! Two principles are enforced here:
//! * **Verification authenticates ORIGIN, not TRUTH** (principle-identifiers-not-identity):
//!   a valid signature proves *who said it*, not that the claim is true. A verified VC
//!   still enters the frame-relative machinery; it is never auto-promoted to fact.
//! * **Grounded issuers** (agency.n3 G1', via `agent.rs`): a credential whose issuer is
//!   an `ArtificialAgent` with no Principal is rejected by [`verify_grounded`] — an AI
//!   agent cannot issue free-floating credentials with no human accountable behind it.

use crate::NQuin;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// A credential: who attests, about whom, what, and for how long.
#[derive(Debug, Clone)]
pub struct Credential {
    /// The issuing agent's identifier.
    pub issuer: u64,
    /// The subject the claims are about.
    pub subject: u64,
    /// Transaction time (when issued), unix seconds.
    pub issued_at: u32,
    /// Valid-until, unix seconds; `0` = no expiry.
    pub valid_until: u32,
    /// The claim quins (the subject's attested attributes). Order is part of the credential.
    pub claims: Vec<NQuin>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum VcError {
    /// The signature does not verify against the issuer key over the credential bytes.
    InvalidSignature,
    /// `now` is past `valid_until`.
    Expired,
    /// The issuer is an ungrounded artificial agent (no Principal) — agency.n3 G1'.
    UngroundedIssuer,
}

/// Canonical SHA-256 digest over the binding fields + claim quins (claim count is
/// length-prefixed to prevent extension ambiguity). Streams — no allocation.
fn digest(c: &Credential) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"q42-vc-v1");
    h.update(c.issuer.to_le_bytes());
    h.update(c.subject.to_le_bytes());
    h.update(c.issued_at.to_le_bytes());
    h.update(c.valid_until.to_le_bytes());
    h.update((c.claims.len() as u64).to_le_bytes());
    for q in &c.claims {
        let b: &[u8; 48] = bytemuck::cast_ref(q);
        h.update(b);
    }
    h.finalize().into()
}

/// Issue: seal the credential with the issuer's Ed25519 signing key.
pub fn issue(signing_key: &SigningKey, credential: &Credential) -> Signature {
    signing_key.sign(&digest(credential))
}

/// Verify the signature + expiry. Authenticates the claim's ORIGIN (who issued it) and
/// that it has not lapsed — NOT that the claim is true.
pub fn verify(
    credential: &Credential,
    issuer_key: &VerifyingKey,
    signature: &Signature,
    now: u32,
) -> Result<(), VcError> {
    if credential.valid_until != 0 && now > credential.valid_until {
        return Err(VcError::Expired);
    }
    issuer_key
        .verify(&digest(credential), signature)
        .map_err(|_| VcError::InvalidSignature)
}

/// Verify as [`verify`], and additionally reject the credential if its issuer is an
/// ungrounded artificial agent in `index` (agency.n3 G1' — no human Principal behind it).
pub fn verify_grounded(
    credential: &Credential,
    issuer_key: &VerifyingKey,
    signature: &Signature,
    now: u32,
    index: &crate::indexing::QuinIndex,
) -> Result<(), VcError> {
    if crate::agent::is_ungrounded_agency(index, credential.issuer) {
        return Err(VcError::UngroundedIssuer);
    }
    verify(credential, issuer_key, signature, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{A_ARTIFICIAL_AGENT, P_OPERATED_BY, P_RDF_TYPE};
    use crate::indexing::QuinIndex;
    use crate::q_hash;

    fn key() -> SigningKey {
        // Static secret so the test needs no RNG (mirrors agency.rs).
        SigningKey::from_bytes(&[7u8; 32])
    }
    fn quin(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }
    fn sample() -> Credential {
        Credential {
            issuer: q_hash("did:example:issuer"),
            subject: q_hash("did:example:alice"),
            issued_at: 1_000,
            valid_until: 2_000,
            claims: vec![quin(
                q_hash("did:example:alice"),
                q_hash("https://ns.webcivics.net/capability/heldBy"),
                q_hash("cap:FluidDynamics"),
            )],
        }
    }

    #[test]
    fn issue_and_verify_roundtrip() {
        let sk = key();
        let c = sample();
        let sig = issue(&sk, &c);
        assert_eq!(verify(&c, &sk.verifying_key(), &sig, 1_500), Ok(()));
    }

    #[test]
    fn tampered_claim_fails_verification() {
        let sk = key();
        let mut c = sample();
        let sig = issue(&sk, &c);
        c.claims[0].object = q_hash("cap:ForgedCredential"); // tamper after signing
        assert_eq!(
            verify(&c, &sk.verifying_key(), &sig, 1_500),
            Err(VcError::InvalidSignature)
        );
    }

    #[test]
    fn wrong_issuer_key_fails() {
        let sk = key();
        let impostor = SigningKey::from_bytes(&[9u8; 32]);
        let c = sample();
        let sig = issue(&sk, &c);
        assert_eq!(
            verify(&c, &impostor.verifying_key(), &sig, 1_500),
            Err(VcError::InvalidSignature)
        );
    }

    #[test]
    fn expired_credential_fails() {
        let sk = key();
        let c = sample();
        let sig = issue(&sk, &c);
        assert_eq!(
            verify(&c, &sk.verifying_key(), &sig, 2_001),
            Err(VcError::Expired)
        );
    }

    #[test]
    fn ungrounded_ai_issuer_is_rejected_but_grounded_one_is_accepted() {
        let sk = key();
        let c = sample();
        let sig = issue(&sk, &c);

        // Issuer is an ArtificialAgent with NO Principal -> ungrounded -> rejected.
        let ungrounded = QuinIndex::from_slice(&[quin(c.issuer, P_RDF_TYPE, A_ARTIFICIAL_AGENT)]);
        assert_eq!(
            verify_grounded(&c, &sk.verifying_key(), &sig, 1_500, &ungrounded),
            Err(VcError::UngroundedIssuer)
        );

        // Same issuer, now with a human Principal behind it -> grounded -> signature governs.
        let human = q_hash("did:example:tim");
        let grounded = QuinIndex::from_slice(&[
            quin(c.issuer, P_RDF_TYPE, A_ARTIFICIAL_AGENT),
            quin(c.issuer, P_OPERATED_BY, human),
        ]);
        assert_eq!(
            verify_grounded(&c, &sk.verifying_key(), &sig, 1_500, &grounded),
            Ok(())
        );
    }
}

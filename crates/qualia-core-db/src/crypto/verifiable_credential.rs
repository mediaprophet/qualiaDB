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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The binary payload is too short to parse a Credential header.
    DecodeTooShort,
    /// The binary payload is too short for the declared claim count.
    DecodeBadClaimCount,
}

impl std::fmt::Display for VcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "VC invalid signature"),
            Self::Expired => write!(f, "VC expired"),
            Self::UngroundedIssuer => write!(f, "VC ungrounded issuer"),
            Self::DecodeTooShort => write!(f, "VC decode: too short"),
            Self::DecodeBadClaimCount => write!(f, "VC decode: bad claim count"),
        }
    }
}
impl std::error::Error for VcError {}

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

/// Issue: seal the credential with the issuer's post-quantum ML-DSA-65 secret key.
pub fn issue_pq(
    signing_key: &[u8; crate::crypto::network::types::ML_DSA_65_SK_LEN],
    credential: &Credential,
    out_sig: &mut [u8; crate::crypto::network::types::ML_DSA_65_SIG_LEN],
) -> Result<(), VcError> {
    use fips204::ml_dsa_65::PrivateKey;
    use fips204::traits::{SerDes, Signer};

    let Ok(sk) = PrivateKey::try_from_bytes(*signing_key) else {
        return Err(VcError::InvalidSignature);
    };
    let d = digest(credential);
    let Ok(sig) = sk.try_sign(&d, b"qualia:vc:pq:v1") else {
        return Err(VcError::InvalidSignature);
    };
    out_sig.copy_from_slice(&sig);
    Ok(())
}

/// Verify a post-quantum ML-DSA-65 credential signature + expiry.
pub fn verify_pq(
    credential: &Credential,
    issuer_pk: &[u8; crate::crypto::network::types::ML_DSA_65_PK_LEN],
    signature: &[u8; crate::crypto::network::types::ML_DSA_65_SIG_LEN],
    now: u32,
) -> Result<(), VcError> {
    use fips204::ml_dsa_65::PublicKey;
    use fips204::traits::{SerDes, Verifier};

    if credential.valid_until != 0 && now > credential.valid_until {
        return Err(VcError::Expired);
    }
    let Ok(pk) = PublicKey::try_from_bytes(*issuer_pk) else {
        return Err(VcError::InvalidSignature);
    };
    let d = digest(credential);
    if pk.verify(&d, signature, b"qualia:vc:pq:v1") {
        Ok(())
    } else {
        Err(VcError::InvalidSignature)
    }
}

/// Issue: seal the credential with a hybrid DualProof (ML-DSA-65 + Ed25519).
pub fn issue_dual(
    mldsa_sk: &[u8; crate::crypto::network::types::ML_DSA_65_SK_LEN],
    ed25519_seed: &[u8; 32],
    credential: &Credential,
    out: &mut crate::crypto::network::dual_sign::DualProof,
) -> Result<(), VcError> {
    let d = digest(credential);
    crate::crypto::network::dual_sign::sign_dual(
        mldsa_sk,
        ed25519_seed,
        &d,
        b"qualia:vc:dual:v1",
        out,
    )
    .map_err(|_| VcError::InvalidSignature)
}

/// Verify a hybrid DualProof (ML-DSA-65 + Ed25519) credential signature + expiry.
pub fn verify_dual(
    credential: &Credential,
    ed25519_pk: &[u8; crate::crypto::network::types::ED25519_PK_LEN],
    mldsa_pk: &[u8; crate::crypto::network::types::ML_DSA_65_PK_LEN],
    proof: &crate::crypto::network::dual_sign::DualProof,
    now: u32,
) -> Result<(), VcError> {
    if credential.valid_until != 0 && now > credential.valid_until {
        return Err(VcError::Expired);
    }
    let d = digest(credential);
    crate::crypto::network::dual_sign::verify_dual(
        mldsa_pk,
        ed25519_pk,
        &d,
        b"qualia:vc:dual:v1",
        proof,
    )
    .map_err(|_| VcError::InvalidSignature)
}

/// Serialize a `Credential` to binary format.
pub fn encode_credential(c: &Credential) -> Vec<u8> {
    let mut out = Vec::with_capacity(28 + c.claims.len() * 48);
    out.extend_from_slice(&c.issuer.to_le_bytes());
    out.extend_from_slice(&c.subject.to_le_bytes());
    out.extend_from_slice(&c.issued_at.to_le_bytes());
    out.extend_from_slice(&c.valid_until.to_le_bytes());
    out.extend_from_slice(&(c.claims.len() as u32).to_le_bytes());
    for q in &c.claims {
        let b: &[u8; 48] = bytemuck::cast_ref(q);
        out.extend_from_slice(b);
    }
    out
}

/// Deserialize a `Credential` from binary format.
pub fn decode_credential(bytes: &[u8]) -> Result<Credential, VcError> {
    if bytes.len() < 28 {
        return Err(VcError::DecodeTooShort);
    }
    let issuer = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let subject = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let issued_at = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let valid_until = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    let claims_len = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
    if bytes.len() < 28 + claims_len * 48 {
        return Err(VcError::DecodeBadClaimCount);
    }
    let mut claims = Vec::with_capacity(claims_len);
    for i in 0..claims_len {
        let start = 28 + i * 48;
        let b = &bytes[start..start + 48];
        // Claims sit at offset 28 + i*48 in the byte stream, so `b` is only
        // 4-aligned while NQuin (6×u64) needs 8-alignment — `from_bytes` would
        // panic (TargetAlignmentGreaterAndInputNotAligned) on any real decode.
        // `pod_read_unaligned` copies the 48 bytes with no alignment requirement.
        let q: NQuin = bytemuck::pod_read_unaligned(b);
        claims.push(q);
    }
    Ok(Credential {
        issuer,
        subject,
        issued_at,
        valid_until,
        claims,
    })
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

    #[test]
    fn encode_decode_roundtrips() {
        let c = sample();
        let bytes = encode_credential(&c);
        let back = decode_credential(&bytes).unwrap();
        assert_eq!(c.issuer, back.issuer);
        assert_eq!(c.subject, back.subject);
        assert_eq!(c.issued_at, back.issued_at);
        assert_eq!(c.valid_until, back.valid_until);
        assert_eq!(c.claims.len(), back.claims.len());
        assert_eq!(c.claims[0].subject, back.claims[0].subject);
    }

    #[test]
    fn decode_rejects_truncated() {
        assert_eq!(decode_credential(&[0u8; 10]), Err(VcError::DecodeTooShort));

        let c = sample();
        let mut bytes = encode_credential(&c);
        bytes.truncate(bytes.len() - 10);
        assert_eq!(decode_credential(&bytes), Err(VcError::DecodeBadClaimCount));
    }

    #[test]
    fn test_pq_and_dual_credential_issue_and_verify() {
        use crate::crypto::network::dual_sign::DualProof;
        use crate::crypto::network::mldsa::generate_keypair;
        use crate::crypto::network::types::{
            ED25519_PK_LEN, ED25519_SIG_LEN, ML_DSA_65_SIG_LEN,
        };

        let c = sample();
        let (mldsa_sk, mldsa_pk) = generate_keypair().unwrap();
        let ed_seed = [55u8; 32];
        let ed_signing = SigningKey::from_bytes(&ed_seed);
        let ed_pk_bytes: [u8; ED25519_PK_LEN] = ed_signing.verifying_key().to_bytes();

        // 1. Post-Quantum ML-DSA-65 test
        let mut pq_sig = [0u8; ML_DSA_65_SIG_LEN];
        issue_pq(&mldsa_sk, &c, &mut pq_sig).unwrap();
        assert_eq!(verify_pq(&c, &mldsa_pk, &pq_sig, 1_500), Ok(()));

        // Expired fails
        assert_eq!(verify_pq(&c, &mldsa_pk, &pq_sig, 2_500), Err(VcError::Expired));

        // 2. Hybrid DualProof test
        let mut dual_proof = DualProof {
            mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
            ed25519_sig: [0u8; ED25519_SIG_LEN],
        };
        issue_dual(&mldsa_sk, &ed_seed, &c, &mut dual_proof).unwrap();
        assert_eq!(
            verify_dual(&c, &ed_pk_bytes, &mldsa_pk, &dual_proof, 1_500),
            Ok(())
        );

        // Tampering fails
        let mut tampered = c.clone();
        tampered.claims[0].object = 999_999;
        assert_eq!(
            verify_pq(&tampered, &mldsa_pk, &pq_sig, 1_500),
            Err(VcError::InvalidSignature)
        );
        assert_eq!(
            verify_dual(&tampered, &ed_pk_bytes, &mldsa_pk, &dual_proof, 1_500),
            Err(VcError::InvalidSignature)
        );
    }
}


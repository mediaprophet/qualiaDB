//! Native Ed25519 attestation adapter (SI-04).
//!
//! Seals an [`InstrumentAttestation`] through the engine VC core. A valid
//! signature authenticates origin/integrity, not substantive truth.

use crate::crypto::verifiable_credential::{self, Credential, VcError};
use crate::q_hash;
use crate::semantic_instruments::attestation::kinds::{AttestationKind, InstrumentAttestation};
use crate::semantic_instruments::errors::InstrumentError;
use crate::NQuin;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};

/// Issued native attestation plus the Ed25519 seal over its VC digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeIssued {
    pub attestation: InstrumentAttestation,
    pub signature: Signature,
}

fn claim_quin(issuer: u64, predicate: u64, object: u64) -> NQuin {
    NQuin {
        subject: issuer,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: NQuin::calculate_parity(issuer, predicate, object, 0, 0),
    }
}

fn credential_from(attestation: &InstrumentAttestation) -> Credential {
    let issuer = q_hash(&attestation.actor);
    let subject = if attestation.kind == AttestationKind::CapabilityAward {
        q_hash(&attestation.award_subject)
    } else {
        q_hash(&attestation.release_id)
    };

    let mut claims = vec![
        claim_quin(
            issuer,
            q_hash(attestation.kind.as_iri()),
            q_hash(&attestation.content_digest),
        ),
        claim_quin(issuer, q_hash("si:releaseId"), q_hash(&attestation.release_id)),
        claim_quin(issuer, q_hash("si:signatureIsNotTruth"), q_hash("true")),
    ];
    if attestation.kind == AttestationKind::CapabilityAward {
        claims.push(claim_quin(
            issuer,
            q_hash("si:awardSubject"),
            q_hash(&attestation.award_subject),
        ));
        claims.push(claim_quin(
            issuer,
            q_hash("si:assessmentReleaseId"),
            q_hash(&attestation.assessment_release_id),
        ));
    }

    Credential {
        issuer,
        subject,
        issued_at: attestation.issued_at,
        valid_until: attestation.valid_until,
        claims,
    }
}

fn map_vc_error(err: VcError) -> InstrumentError {
    match err {
        VcError::Expired => InstrumentError::Expired,
        VcError::InvalidSignature
        | VcError::UngroundedIssuer
        | VcError::DecodeTooShort
        | VcError::DecodeBadClaimCount => InstrumentError::Tampered,
    }
}

/// Seal `attestation` with the issuer's Ed25519 key.
pub fn issue_native(
    signing_key: &SigningKey,
    attestation: InstrumentAttestation,
) -> Result<NativeIssued, InstrumentError> {
    attestation.validate()?;
    let credential = credential_from(&attestation);
    let signature = verifiable_credential::issue(signing_key, &credential);
    Ok(NativeIssued {
        attestation,
        signature,
    })
}

/// Verify origin of `issued` at `now_unix`. Does not assert the claim is true.
pub fn verify_native(
    issuer_key: &VerifyingKey,
    issued: &NativeIssued,
    now_unix: u32,
) -> Result<(), InstrumentError> {
    issued.attestation.validate()?;
    if issued.attestation.valid_until != 0 && now_unix > issued.attestation.valid_until {
        return Err(InstrumentError::Expired);
    }
    let credential = credential_from(&issued.attestation);
    verifiable_credential::verify(&credential, issuer_key, &issued.signature, now_unix)
        .map_err(map_vc_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn digest(fill: char) -> String {
        format!("sha256:{}", fill.to_string().repeat(64))
    }

    fn sample() -> InstrumentAttestation {
        InstrumentAttestation {
            kind: AttestationKind::Authorship,
            release_id: "si:release:demo-unit-convert".into(),
            content_digest: digest('a'),
            actor: "did:example:author".into(),
            issued_at: 1_000,
            valid_until: 2_000,
            origin_is_not_truth: true,
            award_subject: String::new(),
            assessment_release_id: String::new(),
        }
    }

    #[test]
    fn issue_and_verify_roundtrip() {
        let sk = key();
        let issued = issue_native(&sk, sample()).expect("issue");
        assert_eq!(
            verify_native(&sk.verifying_key(), &issued, 1_500),
            Ok(())
        );
        assert!(issued.attestation.origin_is_not_truth);
        assert_eq!(issued.attestation.kind, AttestationKind::Authorship);
    }

    #[test]
    fn tampered_content_digest_fails_verification() {
        let sk = key();
        let mut issued = issue_native(&sk, sample()).expect("issue");
        issued.attestation.content_digest = digest('b');
        assert_eq!(
            verify_native(&sk.verifying_key(), &issued, 1_500),
            Err(InstrumentError::Tampered)
        );
    }

    #[test]
    fn expired_attestation_fails() {
        let sk = key();
        let mut attestation = sample();
        attestation.valid_until = 10;
        let issued = issue_native(&sk, attestation).expect("issue");
        assert_eq!(
            verify_native(&sk.verifying_key(), &issued, 11),
            Err(InstrumentError::Expired)
        );
    }
}

//! W3C Verifiable Credential adapter for instrument attestations (SI-04).
//!
//! Signs with ML-DSA-65 via [`VcRuntime`]. A proof is origin, not truth.

use std::collections::HashMap;

use crate::identity::credentials::{Credential, VcError, VcRuntime};
use crate::semantic_instruments::attestation::kinds::{AttestationKind, InstrumentAttestation};
use crate::semantic_instruments::errors::InstrumentError;

const VC_CONTEXT_V1: &str = "https://www.w3.org/2018/credentials/v1";
const TYPE_VC: &str = "VerifiableCredential";
const TYPE_INSTRUMENT: &str = "InstrumentAttestation";

/// Issued W3C credential paired with the unsigned instrument claim it encodes.
#[derive(Debug, Clone)]
pub struct W3cIssued {
    pub attestation: InstrumentAttestation,
    pub credential: Credential,
}

/// Issue an ML-DSA-signed W3C VC whose subject is the release (or award holder).
pub fn issue_w3c(
    runtime: &VcRuntime,
    key_id: Option<&str>,
    attestation: InstrumentAttestation,
) -> Result<W3cIssued, InstrumentError> {
    attestation.validate()?;
    let unsigned = encode_credential(&attestation);
    let credential = runtime.issue(unsigned, key_id).map_err(map_vc_error)?;
    Ok(W3cIssued {
        attestation,
        credential,
    })
}

/// Verify the attached ML-DSA proof. Missing proof and tamper fail closed.
pub fn verify_w3c(runtime: &VcRuntime, issued: &W3cIssued) -> Result<(), InstrumentError> {
    match runtime.verify_credential(&issued.credential) {
        Ok(true) => Ok(()),
        Ok(false) => Err(InstrumentError::Tampered),
        Err(err) => Err(map_vc_error(err)),
    }
}

fn map_vc_error(err: VcError) -> InstrumentError {
    match err {
        VcError::MissingProof => InstrumentError::MissingProof,
        VcError::VerificationFailed | VcError::CryptoError(_) => InstrumentError::Tampered,
        VcError::SerializationError(msg) => InstrumentError::Canonical(msg),
        VcError::NotImplemented => InstrumentError::Canonical("vc not implemented".into()),
    }
}

fn encode_credential(attestation: &InstrumentAttestation) -> Credential {
    Credential {
        context: vec![VC_CONTEXT_V1.to_string()],
        id: attestation.claim_id(),
        types: vec![TYPE_VC.to_string(), TYPE_INSTRUMENT.to_string()],
        issuer: attestation.actor.clone(),
        issuance_date: attestation.issued_at.to_string(),
        credential_subject: encode_subject(attestation),
        proof: None,
    }
}

fn encode_subject(attestation: &InstrumentAttestation) -> HashMap<String, String> {
    let mut subject = HashMap::new();
    let subject_id = if attestation.kind == AttestationKind::CapabilityAward {
        attestation.award_subject.as_str()
    } else {
        attestation.release_id.as_str()
    };
    subject.insert("id".to_string(), subject_id.to_string());
    subject.insert("kind".to_string(), attestation.kind.as_iri().to_string());
    subject.insert(
        "contentDigest".to_string(),
        attestation.content_digest.clone(),
    );
    subject.insert("originIsNotTruth".to_string(), "true".to_string());
    if attestation.kind == AttestationKind::CapabilityAward {
        subject.insert(
            "awardSubject".to_string(),
            attestation.award_subject.clone(),
        );
        subject.insert(
            "assessmentInstrument".to_string(),
            attestation.assessment_release_id.clone(),
        );
    }
    subject
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::fiduciary_crypto::FiduciaryCrypto;

    fn digest() -> String {
        format!("sha256:{}", "ab".repeat(32))
    }

    fn sample(kind: AttestationKind) -> InstrumentAttestation {
        let mut attestation = InstrumentAttestation {
            kind,
            release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0".into(),
            content_digest: digest(),
            actor: "did:webizen:agent:demo-seed".into(),
            issued_at: 1_700_000_000,
            valid_until: 0,
            origin_is_not_truth: true,
            award_subject: String::new(),
            assessment_release_id: String::new(),
        };
        if kind == AttestationKind::CapabilityAward {
            attestation.release_id =
                "https://ns.webizen.org/demo/award/releases/1.0.0".into();
            attestation.award_subject = "did:webizen:person:learner".into();
            attestation.assessment_release_id =
                "https://ns.webizen.org/demo/unit-convert/releases/1.0.0".into();
        }
        attestation
    }

    fn signed_runtime() -> VcRuntime {
        let mut crypto = FiduciaryCrypto::new();
        crypto.generate_key("default".to_string()).unwrap();
        VcRuntime::new(crypto)
    }

    #[test]
    fn issue_verify_roundtrip() {
        let runtime = signed_runtime();
        let issued = issue_w3c(&runtime, Some("default"), sample(AttestationKind::Authorship))
            .expect("issue");
        assert!(issued.credential.proof.is_some());
        assert_eq!(
            issued.credential.context,
            vec![VC_CONTEXT_V1.to_string()]
        );
        assert_eq!(
            issued.credential.types,
            vec![TYPE_VC.to_string(), TYPE_INSTRUMENT.to_string()]
        );
        assert_eq!(issued.credential.id, issued.attestation.claim_id());
        assert_eq!(issued.credential.issuer, issued.attestation.actor);
        assert_eq!(
            issued.credential.issuance_date,
            issued.attestation.issued_at.to_string()
        );
        assert_eq!(
            issued.credential.credential_subject.get("id").map(String::as_str),
            Some(issued.attestation.release_id.as_str())
        );
        assert_eq!(
            issued.credential.credential_subject.get("kind").map(String::as_str),
            Some(AttestationKind::Authorship.as_iri())
        );
        assert_eq!(
            issued
                .credential
                .credential_subject
                .get("originIsNotTruth")
                .map(String::as_str),
            Some("true")
        );
        verify_w3c(&runtime, &issued).expect("verify");
    }

    #[test]
    fn tampered_issuer_fails_closed() {
        let runtime = signed_runtime();
        let mut issued =
            issue_w3c(&runtime, Some("default"), sample(AttestationKind::Authorship)).unwrap();
        issued.credential.issuer = "https://hacker.com".to_string();
        assert_eq!(
            verify_w3c(&runtime, &issued),
            Err(InstrumentError::Tampered)
        );
    }

    #[test]
    fn unsigned_credential_is_missing_proof() {
        let runtime = VcRuntime::new(FiduciaryCrypto::new());
        let attestation = sample(AttestationKind::Authorship);
        let issued = W3cIssued {
            credential: encode_credential(&attestation),
            attestation,
        };
        assert!(issued.credential.proof.is_none());
        assert_eq!(
            verify_w3c(&runtime, &issued),
            Err(InstrumentError::MissingProof)
        );
    }

    #[test]
    fn capability_award_subject_is_the_holder() {
        let runtime = signed_runtime();
        let attestation = sample(AttestationKind::CapabilityAward);
        let holder = attestation.award_subject.clone();
        let instrument = attestation.release_id.clone();
        let assessment = attestation.assessment_release_id.clone();
        let issued = issue_w3c(&runtime, Some("default"), attestation).expect("issue award");
        let subject = &issued.credential.credential_subject;
        assert_eq!(subject.get("id").map(String::as_str), Some(holder.as_str()));
        assert_ne!(subject.get("id").map(String::as_str), Some(instrument.as_str()));
        assert_eq!(
            subject.get("awardSubject").map(String::as_str),
            Some(holder.as_str())
        );
        assert_eq!(
            subject.get("assessmentInstrument").map(String::as_str),
            Some(assessment.as_str())
        );
        verify_w3c(&runtime, &issued).expect("verify award");
    }

    #[test]
    fn signature_as_truth_is_refused_before_issue() {
        let runtime = signed_runtime();
        let mut attestation = sample(AttestationKind::Endorsement);
        attestation.origin_is_not_truth = false;
        assert_eq!(
            issue_w3c(&runtime, Some("default"), attestation).unwrap_err(),
            InstrumentError::SignatureAsTruth
        );
    }
}

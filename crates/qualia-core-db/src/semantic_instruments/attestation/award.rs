//! Capability-award issuance as an unsigned attestation claim.
//!
//! This constructor records a learner/worker as the award subject of an
//! assessment-instrument release. It does **not** run the instrument, and it
//! does **not** treat evaluator success as sufficient grounds to issue.

use crate::semantic_instruments::attestation::kinds::{AttestationKind, InstrumentAttestation};
use crate::semantic_instruments::errors::InstrumentError;

/// Cold-path inputs for a capability award. `capability` is the concept IRI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAward {
    pub learner: String,
    pub assessment_release_id: String,
    pub content_digest: String,
    pub issuer: String,
    /// Concept IRI of the awarded capability (adapters carry this; the unsigned
    /// release claim binds learner + assessment release).
    pub capability: String,
    pub issued_at: u32,
    pub valid_until: u32,
}

/// Construct a `CapabilityAward` attestation. Origin is recorded; origin is not
/// substantive truth. Passing an evaluator is not a reason to call this.
pub fn issue_capability_award(
    award: CapabilityAward,
) -> Result<InstrumentAttestation, InstrumentError> {
    if award.learner == award.assessment_release_id {
        return Err(InstrumentError::AwardIsInstrument);
    }
    let attestation = InstrumentAttestation {
        kind: AttestationKind::CapabilityAward,
        release_id: award.assessment_release_id.clone(),
        content_digest: award.content_digest,
        actor: award.issuer,
        issued_at: award.issued_at,
        valid_until: award.valid_until,
        origin_is_not_truth: true,
        award_subject: award.learner,
        assessment_release_id: award.assessment_release_id,
    };
    attestation.validate()?;
    Ok(attestation)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn sample_award() -> CapabilityAward {
        CapabilityAward {
            learner: "did:example:learner".into(),
            assessment_release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0".into(),
            content_digest: DIGEST.into(),
            issuer: "did:example:issuer".into(),
            capability: "https://ns.webizen.org/demo/concepts/units".into(),
            issued_at: 1_700_000_000,
            valid_until: 1_800_000_000,
        }
    }

    #[test]
    fn happy_path_constructs_claim_without_running_instrument() {
        let award = sample_award();
        let issued = issue_capability_award(award.clone()).expect("issue");
        issued.validate().expect("valid");
        assert_eq!(issued.kind, AttestationKind::CapabilityAward);
        assert_eq!(issued.release_id, award.assessment_release_id);
        assert_eq!(issued.assessment_release_id, award.assessment_release_id);
        assert_eq!(issued.award_subject, award.learner);
        assert_eq!(issued.actor, award.issuer);
        assert_eq!(issued.content_digest, award.content_digest);
        assert_eq!(issued.issued_at, award.issued_at);
        assert_eq!(issued.valid_until, award.valid_until);
        assert!(issued.origin_is_not_truth);
        assert_ne!(issued.award_subject, issued.assessment_release_id);
    }

    #[test]
    fn learner_equal_to_instrument_is_award_is_instrument() {
        let mut award = sample_award();
        award.learner = award.assessment_release_id.clone();
        let err = issue_capability_award(award).expect_err("must fail");
        assert_eq!(err, InstrumentError::AwardIsInstrument);
    }

    #[test]
    fn issued_claim_records_origin_is_not_truth() {
        let issued = issue_capability_award(sample_award()).expect("issue");
        assert!(issued.origin_is_not_truth);
        assert_ne!(
            issued.origin_is_not_truth, false,
            "a signature/claim of origin is not substantive truth"
        );
    }
}

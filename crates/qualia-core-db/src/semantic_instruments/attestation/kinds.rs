//! Attestation kinds bound to an immutable instrument release digest.
//!
//! A signature proves origin/integrity, not substantive truth. Authorship,
//! review, endorsement, publication, withdrawal and award issuance remain
//! distinct predicates (`si:authoredBy` … `si:issuedBy`).

use crate::semantic_instruments::errors::InstrumentError;

/// One action. A single attestation carries exactly one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttestationKind {
    Authorship,
    Contribution,
    TechnicalReview,
    ProfessionalReview,
    Endorsement,
    Publication,
    Withdrawal,
    CapabilityAward,
}

impl AttestationKind {
    pub const fn as_iri(self) -> &'static str {
        match self {
            Self::Authorship => "si:authoredBy",
            Self::Contribution => "si:contributedBy",
            Self::TechnicalReview => "si:technicallyReviewedBy",
            Self::ProfessionalReview => "si:professionallyReviewedBy",
            Self::Endorsement => "si:endorsedBy",
            Self::Publication => "si:publishedBy",
            Self::Withdrawal => "si:withdrawnBy",
            Self::CapabilityAward => "si:issuedBy",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InstrumentError> {
        match raw.trim() {
            "si:authoredBy" | "authorship" => Ok(Self::Authorship),
            "si:contributedBy" | "contribution" => Ok(Self::Contribution),
            "si:technicallyReviewedBy" | "technical-review" => Ok(Self::TechnicalReview),
            "si:professionallyReviewedBy" | "professional-review" => Ok(Self::ProfessionalReview),
            "si:endorsedBy" | "endorsement" => Ok(Self::Endorsement),
            "si:publishedBy" | "publication" => Ok(Self::Publication),
            "si:withdrawnBy" | "withdrawal" => Ok(Self::Withdrawal),
            "si:issuedBy" | "capability-award" => Ok(Self::CapabilityAward),
            _ => Err(InstrumentError::InvalidAttestationKind),
        }
    }
}

/// Unsigned claim over a release. `origin_is_not_truth` must be true.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InstrumentAttestation {
    pub kind: AttestationKind,
    pub release_id: String,
    pub content_digest: String,
    pub actor: String,
    pub issued_at: u32,
    pub valid_until: u32,
    pub origin_is_not_truth: bool,
    /// Learner/worker DID when `kind` is CapabilityAward; empty otherwise.
    #[serde(default)]
    pub award_subject: String,
    /// Assessment instrument release; required for CapabilityAward.
    #[serde(default)]
    pub assessment_release_id: String,
}

impl InstrumentAttestation {
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if !self.origin_is_not_truth {
            return Err(InstrumentError::SignatureAsTruth);
        }
        if self.release_id.trim().is_empty() {
            return Err(InstrumentError::MissingField("release_id"));
        }
        if !self.content_digest.starts_with("sha256:") || self.content_digest.len() != 71 {
            return Err(InstrumentError::MissingField("content_digest"));
        }
        if self.actor.trim().is_empty() {
            return Err(InstrumentError::MissingField("actor"));
        }
        if self.kind == AttestationKind::CapabilityAward {
            if self.award_subject.trim().is_empty() {
                return Err(InstrumentError::MissingField("award_subject"));
            }
            if self.assessment_release_id.trim().is_empty() {
                return Err(InstrumentError::MissingField("assessment_release_id"));
            }
            if self.award_subject == self.assessment_release_id
                || self.award_subject == self.release_id
            {
                return Err(InstrumentError::AwardIsInstrument);
            }
        }
        Ok(())
    }

    /// Stable id for status/revocation maps (not a signature).
    pub fn claim_id(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.kind.as_iri(),
            self.release_id,
            self.content_digest,
            self.actor,
            self.issued_at
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        format!("sha256:{}", "ab".repeat(32))
    }

    fn base(kind: AttestationKind) -> InstrumentAttestation {
        InstrumentAttestation {
            kind,
            release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0".into(),
            content_digest: digest(),
            actor: "did:webizen:agent:demo-seed".into(),
            issued_at: 1_700_000_000,
            valid_until: 0,
            origin_is_not_truth: true,
            award_subject: String::new(),
            assessment_release_id: String::new(),
        }
    }

    #[test]
    fn kinds_have_distinct_predicates() {
        let iris = [
            AttestationKind::Authorship.as_iri(),
            AttestationKind::Contribution.as_iri(),
            AttestationKind::TechnicalReview.as_iri(),
            AttestationKind::ProfessionalReview.as_iri(),
            AttestationKind::Endorsement.as_iri(),
            AttestationKind::Publication.as_iri(),
            AttestationKind::Withdrawal.as_iri(),
            AttestationKind::CapabilityAward.as_iri(),
        ];
        let mut set = std::collections::BTreeSet::new();
        for iri in iris {
            assert!(set.insert(iri), "duplicate predicate {iri}");
        }
    }

    #[test]
    fn signature_as_truth_rejected() {
        let mut a = base(AttestationKind::Authorship);
        a.origin_is_not_truth = false;
        assert_eq!(a.validate(), Err(InstrumentError::SignatureAsTruth));
    }

    #[test]
    fn award_cannot_be_the_instrument() {
        let mut a = base(AttestationKind::CapabilityAward);
        a.award_subject = a.release_id.clone();
        a.assessment_release_id = a.release_id.clone();
        assert_eq!(a.validate(), Err(InstrumentError::AwardIsInstrument));
    }
}

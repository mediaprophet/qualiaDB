//! Explicit attestation status and revocation (cold path).
//!
//! Revocation stops **future** reliance. It does not delete the historical
//! claim. Withdrawal is its own attestation kind and does not auto-revoke
//! other claims — this registry is the only revocation mechanism.

use std::collections::BTreeMap;

use crate::semantic_instruments::attestation::kinds::InstrumentAttestation;
use crate::semantic_instruments::errors::InstrumentError;

/// Current reliance status of an unsigned attestation claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationStatus {
    Valid,
    Expired,
    Revoked { reason: String, revoked_at: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RevocationRecord {
    reason: String,
    revoked_at: u32,
}

/// Explicit revocation map keyed by `InstrumentAttestation::claim_id`.
#[derive(Debug, Clone, Default)]
pub struct StatusRegistry {
    revoked: BTreeMap<String, RevocationRecord>,
}

impl StatusRegistry {
    pub fn new() -> Self {
        Self {
            revoked: BTreeMap::new(),
        }
    }

    /// Record a revocation. The attestation itself is not deleted.
    pub fn revoke(&mut self, claim_id: &str, reason: &str, revoked_at: u32) {
        self.revoked.insert(
            claim_id.to_string(),
            RevocationRecord {
                reason: reason.to_string(),
                revoked_at,
            },
        );
    }

    /// Historical claim ids that have been revoked (still listable).
    pub fn revoked_ids(&self) -> impl Iterator<Item = &str> + '_ {
        self.revoked.keys().map(String::as_str)
    }

    pub fn status(&self, attestation: &InstrumentAttestation, now: u32) -> AttestationStatus {
        if let Some(record) = self.revoked.get(&attestation.claim_id()) {
            return AttestationStatus::Revoked {
                reason: record.reason.clone(),
                revoked_at: record.revoked_at,
            };
        }
        if attestation.valid_until != 0 && now > attestation.valid_until {
            return AttestationStatus::Expired;
        }
        AttestationStatus::Valid
    }

    /// Fail-closed future reliance. Revoked and expired claims must not be used.
    pub fn assert_rely(
        &self,
        attestation: &InstrumentAttestation,
        now: u32,
    ) -> Result<(), InstrumentError> {
        match self.status(attestation, now) {
            AttestationStatus::Valid => Ok(()),
            AttestationStatus::Expired => Err(InstrumentError::Expired),
            AttestationStatus::Revoked { .. } => Err(InstrumentError::Revoked),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::attestation::kinds::AttestationKind;

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn sample(kind: AttestationKind, valid_until: u32) -> InstrumentAttestation {
        InstrumentAttestation {
            kind,
            release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0".into(),
            content_digest: DIGEST.into(),
            actor: "did:example:issuer".into(),
            issued_at: 1_000,
            valid_until,
            origin_is_not_truth: true,
            award_subject: String::new(),
            assessment_release_id: String::new(),
        }
    }

    #[test]
    fn revoke_then_assert_rely_fails_with_revoked() {
        let attestation = sample(AttestationKind::Authorship, 0);
        let mut registry = StatusRegistry::new();
        registry.revoke(&attestation.claim_id(), "key-compromise", 2_000);
        let err = registry
            .assert_rely(&attestation, 1_500)
            .expect_err("revoked must not be relied on");
        assert_eq!(err, InstrumentError::Revoked);
        match registry.status(&attestation, 1_500) {
            AttestationStatus::Revoked { reason, revoked_at } => {
                assert_eq!(reason, "key-compromise");
                assert_eq!(revoked_at, 2_000);
            }
            other => panic!("expected Revoked, got {other:?}"),
        }
    }

    #[test]
    fn revoked_ids_still_contains_the_claim() {
        let attestation = sample(AttestationKind::Publication, 0);
        let mut registry = StatusRegistry::new();
        let claim = attestation.claim_id();
        registry.revoke(&claim, "withdrawn-from-catalog", 9);
        assert!(registry.revoked_ids().any(|id| id == claim));
        assert_eq!(registry.revoked_ids().count(), 1);
    }

    #[test]
    fn expiry_without_revoke_is_expired() {
        let attestation = sample(AttestationKind::Endorsement, 100);
        let registry = StatusRegistry::new();
        assert_eq!(registry.status(&attestation, 101), AttestationStatus::Expired);
        assert_eq!(
            registry.assert_rely(&attestation, 101),
            Err(InstrumentError::Expired)
        );
        assert!(registry.revoked_ids().next().is_none());
    }

    #[test]
    fn valid_unrevoked_is_valid() {
        let attestation = sample(AttestationKind::TechnicalReview, 100);
        let registry = StatusRegistry::new();
        assert_eq!(registry.status(&attestation, 100), AttestationStatus::Valid);
        registry
            .assert_rely(&attestation, 50)
            .expect("unexpired unrevoked");
        let never_expires = sample(AttestationKind::Contribution, 0);
        assert_eq!(
            registry.status(&never_expires, u32::MAX),
            AttestationStatus::Valid
        );
    }

    #[test]
    fn withdrawal_kind_does_not_auto_revoke_other_claims() {
        let live = sample(AttestationKind::Authorship, 0);
        let withdrawal = sample(AttestationKind::Withdrawal, 0);
        let registry = StatusRegistry::new();
        assert_eq!(registry.status(&live, 1), AttestationStatus::Valid);
        assert_eq!(registry.status(&withdrawal, 1), AttestationStatus::Valid);
        assert!(registry.revoked_ids().next().is_none());
    }
}

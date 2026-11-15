//! Plane join prohibition and typed evidence-relation validation (IDF-02).
//!
//! No universal NaturalAgent join key exists: identity-equivalence and person-merge
//! attempts fail closed (`QdnfError::Unauthorized | QdnfError::Denied`), whereas
//! typed cross-plane evidence morphisms (`idf:supportedByInstrument`,
//! `idf:assertedAbout`, `idf:observedAtHandle`) are explicitly permitted.

use crate::net::qdnf::authority::Plane;
use crate::net::qdnf::errors::QdnfError;

/// Proposed merge key. None of these may join planes or persons.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKey {
    Did = 1,
    Wallet = 2,
    Location = 3,
    Role = 4,
    Alias = 5,
    ClassifierSimilarity = 6,
    SameAsAssertion = 7,
    BiometricSample = 8,
    AccountHandle = 9,
    SensorCoPresence = 10,
    OrgRepresentative = 11,
    RelianceRetraction = 12,
}

impl JoinKey {
    pub const ALL: [Self; 12] = [
        Self::Did,
        Self::Wallet,
        Self::Location,
        Self::Role,
        Self::Alias,
        Self::ClassifierSimilarity,
        Self::SameAsAssertion,
        Self::BiometricSample,
        Self::AccountHandle,
        Self::SensorCoPresence,
        Self::OrgRepresentative,
        Self::RelianceRetraction,
    ];
}

/// Bounded join request. Copy-only; no string aliases on the hot path.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinAttempt {
    pub left: Plane,
    pub right: Plane,
    pub key: JoinKey,
}

/// Every join-key attempt is refused. Similarity and possession are not identity.
#[inline]
pub fn attempt_join(attempt: JoinAttempt) -> Result<(), QdnfError> {
    let _ = (attempt.left, attempt.right);
    match attempt.key {
        JoinKey::Did
        | JoinKey::Wallet
        | JoinKey::Role
        | JoinKey::BiometricSample
        | JoinKey::AccountHandle
        | JoinKey::OrgRepresentative => Err(QdnfError::Unauthorized),
        JoinKey::Location
        | JoinKey::Alias
        | JoinKey::ClassifierSimilarity
        | JoinKey::SameAsAssertion
        | JoinKey::SensorCoPresence
        | JoinKey::RelianceRetraction => Err(QdnfError::Denied),
    }
}

/// First-class typed cross-plane evidence morphisms in `C_IDF` (§1.3.3).
/// These link planes without collapsing or equating their subjects.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRelation {
    /// `idf:supportedByInstrument` (`Claim -> Instrument`)
    SupportedByInstrument = 1,
    /// `idf:assertedAbout` (`Claim -> Entity | Instrument | Handle`)
    AssertedAbout = 2,
    /// `idf:observedAtHandle` (`Claim | Instrument -> Handle`)
    ObservedAtHandle = 3,
}

impl EvidenceRelation {
    pub const IRI_SUPPORTED_BY_INSTRUMENT: &'static str = "urn:qualia:idf:supportedByInstrument";
    pub const IRI_ASSERTED_ABOUT: &'static str = "urn:qualia:idf:assertedAbout";
    pub const IRI_OBSERVED_AT_HANDLE: &'static str = "urn:qualia:idf:observedAtHandle";
}

/// Validate a typed cross-plane evidence relation. Never merges planes.
#[inline]
pub fn validate_evidence_relation(
    relation: EvidenceRelation,
    from: Plane,
    to: Plane,
) -> Result<(), QdnfError> {
    match relation {
        EvidenceRelation::SupportedByInstrument => {
            if from == Plane::Claim && to == Plane::Instrument {
                Ok(())
            } else {
                Err(QdnfError::Denied)
            }
        }
        EvidenceRelation::AssertedAbout => {
            if from == Plane::Claim
                && matches!(to, Plane::Entity | Plane::Instrument | Plane::Handle | Plane::Claim)
            {
                Ok(())
            } else {
                Err(QdnfError::Denied)
            }
        }
        EvidenceRelation::ObservedAtHandle => {
            if matches!(from, Plane::Claim | Plane::Instrument) && to == Plane::Handle {
                Ok(())
            } else {
                Err(QdnfError::Denied)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLANES: [Plane; 4] = [
        Plane::Entity,
        Plane::Claim,
        Plane::Handle,
        Plane::Instrument,
    ];

    fn err_of(key: JoinKey) -> QdnfError {
        attempt_join(JoinAttempt {
            left: Plane::Entity,
            right: Plane::Instrument,
            key,
        })
        .unwrap_err()
    }

    #[test]
    fn did_join_is_unauthorized() {
        assert_eq!(err_of(JoinKey::Did), QdnfError::Unauthorized);
    }

    #[test]
    fn wallet_join_is_unauthorized() {
        assert_eq!(err_of(JoinKey::Wallet), QdnfError::Unauthorized);
    }

    #[test]
    fn location_join_is_denied() {
        assert_eq!(err_of(JoinKey::Location), QdnfError::Denied);
    }

    #[test]
    fn role_join_is_unauthorized() {
        assert_eq!(err_of(JoinKey::Role), QdnfError::Unauthorized);
    }

    #[test]
    fn alias_join_is_denied() {
        assert_eq!(err_of(JoinKey::Alias), QdnfError::Denied);
    }

    #[test]
    fn classifier_similarity_join_is_denied() {
        assert_eq!(err_of(JoinKey::ClassifierSimilarity), QdnfError::Denied);
    }

    #[test]
    fn f1_extended_join_keys_are_refused() {
        assert_eq!(err_of(JoinKey::SameAsAssertion), QdnfError::Denied);
        assert_eq!(err_of(JoinKey::BiometricSample), QdnfError::Unauthorized);
        assert_eq!(err_of(JoinKey::AccountHandle), QdnfError::Unauthorized);
        assert_eq!(err_of(JoinKey::SensorCoPresence), QdnfError::Denied);
        assert_eq!(err_of(JoinKey::OrgRepresentative), QdnfError::Unauthorized);
        assert_eq!(err_of(JoinKey::RelianceRetraction), QdnfError::Denied);
    }

    #[test]
    fn every_join_key_refuses_every_plane_pair() {
        for &left in &PLANES {
            for &right in &PLANES {
                for key in JoinKey::ALL {
                    let outcome = attempt_join(JoinAttempt { left, right, key });
                    assert!(
                        matches!(outcome, Err(QdnfError::Denied | QdnfError::Unauthorized)),
                        "join {left:?}+{right:?} via {key:?} must not succeed"
                    );
                }
            }
        }
    }

    #[test]
    fn typed_evidence_relations_permitted_only_for_valid_domain_and_range_planes() {
        assert_eq!(
            validate_evidence_relation(
                EvidenceRelation::SupportedByInstrument,
                Plane::Claim,
                Plane::Instrument
            ),
            Ok(())
        );
        assert_eq!(
            validate_evidence_relation(
                EvidenceRelation::SupportedByInstrument,
                Plane::Entity,
                Plane::Instrument
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            validate_evidence_relation(
                EvidenceRelation::AssertedAbout,
                Plane::Claim,
                Plane::Entity
            ),
            Ok(())
        );
        assert_eq!(
            validate_evidence_relation(
                EvidenceRelation::ObservedAtHandle,
                Plane::Claim,
                Plane::Handle
            ),
            Ok(())
        );
        assert_eq!(
            validate_evidence_relation(
                EvidenceRelation::ObservedAtHandle,
                Plane::Entity,
                Plane::Handle
            ),
            Err(QdnfError::Denied)
        );
    }
}

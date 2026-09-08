//! Plane join prohibition. No universal NaturalAgent join key exists.

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
}

impl JoinKey {
    pub const ALL: [Self; 6] = [
        Self::Did,
        Self::Wallet,
        Self::Location,
        Self::Role,
        Self::Alias,
        Self::ClassifierSimilarity,
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
        JoinKey::Did | JoinKey::Wallet | JoinKey::Role => Err(QdnfError::Unauthorized),
        JoinKey::Location | JoinKey::Alias | JoinKey::ClassifierSimilarity => {
            Err(QdnfError::Denied)
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
}

//! Authorised remote processing (E15.3, E15.5, E15.6).
//!
//! Remote matching is unselectable until a complete qualification contract
//! exists *and* independent measurement passes. This crate has no operational
//! biometric corpus. The linear-algebra privacy engine (BFV packed integer
//! arithmetic and DP releases) is not a qualified biometric matcher.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Bound fields required on an encrypted remote object (E15.3).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteBoundFields {
    pub modality: StrongDigest,
    pub algorithm_version: StrongDigest,
    pub quality: u16,
    pub provenance: StrongDigest,
    pub purpose: StrongDigest,
    pub recipient: StrongDigest,
    pub retention_unix: u64,
}

impl RemoteBoundFields {
    #[inline]
    pub fn missing_any(&self) -> bool {
        self.modality.is_zero()
            || self.algorithm_version.is_zero()
            || self.quality == 0
            || self.provenance.is_zero()
            || self.purpose.is_zero()
            || self.recipient.is_zero()
            || self.retention_unix == 0
    }
}

/// Encrypted artifact for a selected recipient/worker. Ciphertext only.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncryptedRemoteObject {
    pub ciphertext_digest: StrongDigest,
    pub bound: RemoteBoundFields,
    pub explicitly_authorised: bool,
}

/// Admit remote *processing* only when every bound field is present on an
/// encrypted, explicitly authorised object. Admission is not qualification
/// and does not make remote matching selectable.
pub fn admit_remote_processing(obj: &EncryptedRemoteObject) -> Result<(), QdnfError> {
    if !obj.explicitly_authorised {
        return Err(QdnfError::Denied);
    }
    if obj.ciphertext_digest.is_zero() {
        return Err(QdnfError::Denied);
    }
    if obj.bound.missing_any() {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

/// Owner-approved measurement contract required before remote qualification.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationCriteria {
    pub threshold: u16,
    pub sample_size: u32,
    pub spoof_limit: u16,
    pub leakage_limit: u16,
}

impl QualificationCriteria {
    #[inline]
    pub const fn any_zero(self) -> bool {
        self.threshold == 0
            || self.sample_size == 0
            || self.spoof_limit == 0
            || self.leakage_limit == 0
    }
}

/// Check that a criteria document is present and non-zero.
///
/// `Ok` means the contract fields are filled. It is not operational
/// qualification: [`remote_matching_selectable`] stays false, and
/// [`synthetic_accuracy_is_operational`] is false.
pub fn qualify_remote(c: Option<&QualificationCriteria>) -> Result<(), QdnfError> {
    let c = match c {
        None => return Err(QdnfError::UnknownProfile),
        Some(c) => c,
    };
    if c.any_zero() {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

/// Remote matching remains unselectable: E15.6 independent measurement has
/// not been performed, and E15.5 SMPC is not qualified.
#[inline]
pub const fn remote_matching_selectable() -> bool {
    false
}

/// Existing privacy engine is packed BFV / DP, not a biometric matcher.
#[inline]
pub const fn smpc_matching_qualified() -> bool {
    false
}

/// Synthetic fixtures prove protocol wiring, not operational accuracy.
#[inline]
pub const fn synthetic_accuracy_is_operational() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 0xC4;
        x
    }

    fn complete_fields() -> RemoteBoundFields {
        RemoteBoundFields {
            modality: d(1),
            algorithm_version: d(2),
            quality: 7,
            provenance: d(3),
            purpose: d(4),
            recipient: d(5),
            retention_unix: 1_700_000_000,
        }
    }

    fn complete_object() -> EncryptedRemoteObject {
        EncryptedRemoteObject {
            ciphertext_digest: d(9),
            bound: complete_fields(),
            explicitly_authorised: true,
        }
    }

    fn complete_criteria() -> QualificationCriteria {
        QualificationCriteria {
            threshold: 10,
            sample_size: 1000,
            spoof_limit: 5,
            leakage_limit: 3,
        }
    }

    #[test]
    fn missing_authorisation_is_denied() {
        let mut obj = complete_object();
        obj.explicitly_authorised = false;
        assert_eq!(
            admit_remote_processing(&obj).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn missing_any_bound_field_is_denied() {
        let cases: [RemoteBoundFields; 7] = [
            RemoteBoundFields {
                modality: StrongDigest::ZERO,
                ..complete_fields()
            },
            RemoteBoundFields {
                algorithm_version: StrongDigest::ZERO,
                ..complete_fields()
            },
            RemoteBoundFields {
                quality: 0,
                ..complete_fields()
            },
            RemoteBoundFields {
                provenance: StrongDigest::ZERO,
                ..complete_fields()
            },
            RemoteBoundFields {
                purpose: StrongDigest::ZERO,
                ..complete_fields()
            },
            RemoteBoundFields {
                recipient: StrongDigest::ZERO,
                ..complete_fields()
            },
            RemoteBoundFields {
                retention_unix: 0,
                ..complete_fields()
            },
        ];
        for bound in cases {
            let obj = EncryptedRemoteObject {
                ciphertext_digest: d(9),
                bound,
                explicitly_authorised: true,
            };
            assert_eq!(
                admit_remote_processing(&obj).unwrap_err(),
                QdnfError::Denied
            );
        }
    }

    #[test]
    fn unencrypted_object_is_denied() {
        let mut obj = complete_object();
        obj.ciphertext_digest = StrongDigest::ZERO;
        assert_eq!(
            admit_remote_processing(&obj).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn complete_authorised_encrypted_object_admits_processing() {
        admit_remote_processing(&complete_object()).unwrap();
        assert!(!remote_matching_selectable());
    }

    #[test]
    fn qualify_remote_none_is_unknown_profile() {
        assert_eq!(qualify_remote(None).unwrap_err(), QdnfError::UnknownProfile);
        assert!(!remote_matching_selectable());
    }

    #[test]
    fn qualify_remote_zeros_are_denied() {
        let zeros = QualificationCriteria {
            threshold: 0,
            sample_size: 0,
            spoof_limit: 0,
            leakage_limit: 0,
        };
        assert_eq!(qualify_remote(Some(&zeros)).unwrap_err(), QdnfError::Denied);
        let partial = QualificationCriteria {
            threshold: 10,
            sample_size: 0,
            spoof_limit: 5,
            leakage_limit: 3,
        };
        assert_eq!(
            qualify_remote(Some(&partial)).unwrap_err(),
            QdnfError::Denied
        );
        assert!(!remote_matching_selectable());
    }

    #[test]
    fn complete_criteria_do_not_select_remote_or_claim_accuracy() {
        qualify_remote(Some(&complete_criteria())).unwrap();
        assert!(!remote_matching_selectable());
        assert!(!smpc_matching_qualified());
        assert!(!synthetic_accuracy_is_operational());
    }
}

//! QDNF E15 biometric protection and privacy-preserving verification.
//!
//! Capture, templates, match results and identity assertions are distinct
//! objects. A biometric match never grants network authority. Remote matching
//! stays unselectable until independent qualification criteria exist and pass;
//! synthetic fixtures do not establish operational accuracy. The existing
//! linear-algebra privacy engine is not a qualified biometric matcher.
//!
//! Honest non-claims: this library does not implement operational FAR/FRR,
//! liveness, unlinkability proofs, or SMPC biometric recognition.

pub mod capture;
pub mod match_local;
pub mod policy;
pub mod remote;
pub mod template;

pub use capture::{retain_raw_as_network_id, Modality, RawCapture};
pub use match_local::{
    execution_permit_from_match, match_grants_network_authority, match_local, MatchDecision,
    MatchResult,
};
pub use policy::{
    assert_identity_from_match, cross_purpose_link, publish_template, recover_with_key_ref,
    recover_with_pin, recovery_without_biometric, reuse_template, silent_identify,
    unsalted_reusable_hash, unsalted_reusable_hash_allowed, IdentityAssertion, ReuseIntent,
};
pub use remote::{
    admit_remote_processing, qualify_remote, remote_matching_selectable, smpc_matching_qualified,
    synthetic_accuracy_is_operational, EncryptedRemoteObject, QualificationCriteria,
    RemoteBoundFields,
};
pub use template::{
    derive_template, revoke_template, CancellableId, DerivedTemplate, MAX_TEMPLATE_SLOTS,
};

/// Discriminator for the four E15.1 object kinds. Not a wire opcode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiometricObjectKind {
    RawCapture = 1,
    DerivedTemplate = 2,
    MatchResult = 3,
    IdentityAssertion = 4,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_object_kinds_are_distinct() {
        let kinds = [
            BiometricObjectKind::RawCapture,
            BiometricObjectKind::DerivedTemplate,
            BiometricObjectKind::MatchResult,
            BiometricObjectKind::IdentityAssertion,
        ];
        let mut i = 0usize;
        while i < kinds.len() {
            let mut j = i + 1;
            while j < kinds.len() {
                assert_ne!(kinds[i], kinds[j]);
                j += 1;
            }
            i += 1;
        }
    }

    #[test]
    fn reexported_honest_gates() {
        assert!(!match_grants_network_authority());
        assert!(!remote_matching_selectable());
        assert!(recovery_without_biometric());
        assert!(!unsalted_reusable_hash_allowed());
        assert!(!smpc_matching_qualified());
        assert!(!synthetic_accuracy_is_operational());
    }
}

//! Known-peer medical communications (E14 / NET-05.21–24).
//!
//! Private authenticated pairing, standing care permits, recipient envelopes
//! and an encrypted mailbox (bounded ciphertext bytes). Transport delivery never
//! asserts clinical review. No medical payload bytes are stored on generic logs.

pub mod envelope;
pub mod grants;
pub mod help;
pub mod mailbox;
pub mod pairing;
pub mod permit;

pub use envelope::{
    forward_ciphertext, generic_log_contains_medical_payload, inherit_response_label,
    intermediary_decrypts, restore_labelled_backup, seal_envelope, verify_envelope,
    visible_recipients, ClinicalBoundary, GenericLog, GenericLogRecord, IntermediaryRole,
    PayloadEnvelope,
};
pub use grants::{CareGrant, CareGrantKind, GrantTable, MAX_VISIBLE};
pub use help::{
    admit_confidential_help, notify_guardian, notify_guardian_automatically, CapacityMandatePolicy,
};
pub use mailbox::{
    digest_attachment_pages, enqueue_offline, export_interchange, network_asserts_clinical_review,
    patient_record_associated, release_queued, stream_attachment, InterchangeAdapter, Mailbox,
    MailboxSlot, MailboxState, OfflineQueue, OfflineSlot, MAILBOX_CIPHERTEXT_BYTES,
    STREAM_PAGE_BYTES,
};
pub use pairing::{
    diagnosis_discovery_allowed, public_patient_index_allowed, publish_relationship,
    vip_association_indexed, ClinicalPair, PairingTable,
};
pub use permit::{PermitTable, StandingPermit};

use crate::net::qdnf::authority::{ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;

/// Occupied pairing, permit, grant, log and mailbox slots.
pub const MAX_SLOTS: usize = 8;

fn first_empty<T: Copy>(slots: &[Option<T>; MAX_SLOTS]) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_SLOTS {
        if slots[i].is_none() {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Policy admission without a caller-supplied `grant_current` boolean.
fn admit_outcome(outcome: PolicyOutcome) -> Result<(), QdnfError> {
    match outcome {
        PolicyOutcome::Allow => Ok(()),
        PolicyOutcome::Deny => Err(QdnfError::Denied),
        PolicyOutcome::Challenge => Err(QdnfError::Challenge),
        PolicyOutcome::NeedsHuman => Err(QdnfError::NeedsHuman),
        PolicyOutcome::Incomplete => Err(QdnfError::Incomplete),
        PolicyOutcome::Error => Err(QdnfError::Unauthorized),
    }
}

fn require_active(state: ContactState) -> Result<(), QdnfError> {
    match state {
        ContactState::Active => Ok(()),
        ContactState::Blocked | ContactState::Suspended => Err(QdnfError::Denied),
        ContactState::Request | ContactState::Consent => Err(QdnfError::Unauthorized),
    }
}

fn require_active_mutual(patient: ContactState, clinician: ContactState) -> Result<(), QdnfError> {
    require_active(patient)?;
    require_active(clinician)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::authority::{ContactState, PolicyOutcome, TemporalGrant};
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::policy_labels::{
        encode_label_into, join_one, project_sink, verify_label, Confidentiality, DerivedSink,
        JobLabelContext, LabelFields, VerifiedLabel, NO_PUBLIC_INDEX, NO_TRAINING,
    };
    use crate::net::qdnf::types::{ProfileId, StrongDigest};

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 0xE1;
        x
    }

    fn grant_until(audience: StrongDigest, exp: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: d(0x11),
            audience_digest: audience,
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: exp,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    fn verified(conf: Confidentiality, iss: StrongDigest, audience: StrongDigest) -> VerifiedLabel {
        let mut fields = LabelFields::request(conf, iss);
        fields.audience = audience;
        fields.restriction_bits = NO_TRAINING | NO_PUBLIC_INDEX;
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        verify_label(fields, &buf[..n]).unwrap()
    }

    #[test]
    fn e14_7_offline_queue_expiry_stays_sealed() {
        let mut q = OfflineQueue::new();
        let slot = enqueue_offline(
            &mut q,
            OfflineSlot {
                recipient: d(2),
                expires_unix: 50,
                ciphertext_len: 16,
                ciphertext_digest: d(9),
            },
            10,
        )
        .unwrap();
        assert_eq!(
            release_queued(&mut q, slot, 50, ContactState::Active, ContactState::Active),
            Err(QdnfError::Expired)
        );
        assert_eq!(q.get(slot).unwrap().ciphertext_digest, d(9));
        assert_eq!(q.get(slot).unwrap().ciphertext_len, 16);
    }

    #[test]
    fn e14_7_clinician_departure_revoke_is_denied() {
        let mut grants = GrantTable::new();
        let clinician = d(2);
        grants
            .add(CareGrant {
                kind: CareGrantKind::Clinician,
                recipient: clinician,
                grant: grant_until(clinician, 100),
            })
            .unwrap();
        grants.revoke(CareGrantKind::Clinician, clinician).unwrap();
        assert_eq!(
            grants.authorize_kind(
                CareGrantKind::Clinician,
                clinician,
                ContactState::Active,
                ContactState::Active,
                10,
            ),
            Err(QdnfError::Denied)
        );
        let mut mb = Mailbox::new();
        let s = mb.store(b"pending-ct", clinician, 1).unwrap();
        mb.revoke_clinician(clinician).unwrap();
        assert_eq!(mb.deliver(s, clinician, 1), Err(QdnfError::Revoked));
        assert!(!network_asserts_clinical_review());
    }

    #[test]
    fn e14_7_referral_requires_separate_grant() {
        let mut grants = GrantTable::new();
        let clinician = d(2);
        let referral = d(4);
        grants
            .add(CareGrant {
                kind: CareGrantKind::Clinician,
                recipient: clinician,
                grant: grant_until(clinician, 100),
            })
            .unwrap();
        assert_eq!(
            grants.authorize_kind(
                CareGrantKind::Referral,
                referral,
                ContactState::Active,
                ContactState::Active,
                10,
            ),
            Err(QdnfError::Unauthorized)
        );
        grants
            .add(CareGrant {
                kind: CareGrantKind::Referral,
                recipient: referral,
                grant: grant_until(referral, 100),
            })
            .unwrap();
        assert!(grants
            .authorize_kind(
                CareGrantKind::Referral,
                referral,
                ContactState::Active,
                ContactState::Active,
                10,
            )
            .is_ok());
    }

    #[test]
    fn e14_7_mistaken_recipient_denied() {
        let clinician = d(2);
        let other = d(9);
        let label = verified(Confidentiality::C2Sensitive, d(1), clinician);
        let mut grants = GrantTable::new();
        grants
            .add(CareGrant {
                kind: CareGrantKind::Clinician,
                recipient: clinician,
                grant: grant_until(clinician, 100),
            })
            .unwrap();
        let env = seal_envelope(
            &grants,
            &label,
            d(0x21),
            d(0x11),
            100,
            b"sealed-ct",
            PolicyOutcome::Allow,
            ContactState::Active,
            ContactState::Active,
            10,
        )
        .unwrap();
        assert_eq!(verify_envelope(&env, other, 10), Err(QdnfError::Denied));
        assert!(verify_envelope(&env, clinician, 10).is_ok());
        let seen = visible_recipients(&env);
        assert_eq!(seen[0], clinician);
        assert_eq!(seen[1], StrongDigest::ZERO);
    }

    #[test]
    fn e14_7_backup_restore_does_not_drop_label() {
        let stored = verified(Confidentiality::C2Sensitive, d(1), d(2));
        let mut ctx = JobLabelContext::new(*stored.fields()).unwrap();
        let backup = project_sink(&ctx, DerivedSink::Backup).unwrap();
        assert_eq!(backup.confidentiality, Confidentiality::C2Sensitive);
        assert_eq!(
            backup.restriction_bits & (NO_TRAINING | NO_PUBLIC_INDEX),
            NO_TRAINING | NO_PUBLIC_INDEX
        );
        assert!(restore_labelled_backup(&stored, &backup).is_ok());
        let mut dropped = backup;
        dropped.confidentiality = Confidentiality::C0Public;
        dropped.restriction_bits = 0;
        assert_eq!(
            restore_labelled_backup(&stored, &dropped),
            Err(QdnfError::Denied)
        );
        ctx.seal();
    }

    #[test]
    fn e14_7_response_inherits_request_label() {
        let request = LabelFields::request(Confidentiality::C1Private, d(1));
        let consulted = verified(Confidentiality::C2Sensitive, d(1), d(2));
        let mut out = LabelFields::blank();
        inherit_response_label(&request, &consulted, &mut out).unwrap();
        assert_eq!(out.confidentiality, Confidentiality::C2Sensitive);
        assert_eq!(out.restriction_bits & NO_TRAINING, NO_TRAINING);
        let mut acc = request;
        join_one(&mut acc, consulted.fields()).unwrap();
        assert_eq!(acc.confidentiality, out.confidentiality);
        assert_eq!(acc.restriction_bits, out.restriction_bits);
    }
}

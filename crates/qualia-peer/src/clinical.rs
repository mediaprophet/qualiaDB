//! Public application API for known-peer medical communications (E14).
//!
//! Pairing, standing permits, envelopes, mailbox and confidential help go
//! through this owner. Transport delivery never asserts clinical review.

use crate::{PeerHost, QdnfError};
use qualia_core_db::net::qdnf::authority::{ContactState, PolicyOutcome, TemporalGrant};
use qualia_core_db::net::qdnf::clinical::{
    admit_confidential_help, diagnosis_discovery_allowed, enqueue_offline, export_interchange,
    inherit_response_label, notify_guardian, patient_record_associated, public_patient_index_allowed,
    release_queued, restore_labelled_backup, seal_envelope, stream_attachment, vip_association_indexed,
    CapacityMandatePolicy, CareGrant, CareGrantKind, ClinicalPair, GrantTable, InterchangeAdapter,
    Mailbox, OfflineQueue, OfflineSlot, PairingTable, PayloadEnvelope, PermitTable, StandingPermit,
    STREAM_PAGE_BYTES,
};
use qualia_core_db::net::qdnf::contracts::{BoundGenerations, LiveGenerations};
use qualia_core_db::net::qdnf::policy_labels::{
    encode_label_into, verify_label, Confidentiality, LabelFields, VerifiedLabel,
};
use qualia_core_db::net::qdnf::types::{Generation, ProfileId, StrongDigest};

/// Session-owned clinical tables. Not a public patient index.
#[derive(Clone, Copy, Debug)]
pub struct ClinicalSession {
    pairings: PairingTable,
    permits: PermitTable,
    grants: GrantTable,
    mailbox: Mailbox,
    offline: OfflineQueue,
}

impl PeerHost {
    /// Application clinical owner for this host cell. Independent of pairing IPC.
    pub fn clinical_session(&self) -> ClinicalSession {
        ClinicalSession::new()
    }
}

impl ClinicalSession {
    pub fn new() -> Self {
        Self {
            pairings: PairingTable::new(),
            permits: PermitTable::new(),
            grants: GrantTable::new(),
            mailbox: Mailbox::new(),
            offline: OfflineQueue::new(),
        }
    }

    pub fn admit_pair(&mut self, pair: ClinicalPair) -> Result<usize, QdnfError> {
        if public_patient_index_allowed()
            || vip_association_indexed()
            || diagnosis_discovery_allowed()
        {
            return Err(QdnfError::Denied);
        }
        self.pairings.admit(pair)
    }

    pub fn refuse_public_index(&self, pair: ClinicalPair) -> Result<(), QdnfError> {
        self.pairings.index_public(pair)
    }

    pub fn add_permit(&mut self, permit: StandingPermit) -> Result<usize, QdnfError> {
        self.permits.add(permit)
    }

    pub fn set_live(&mut self, live: LiveGenerations) {
        self.permits.set_live(live);
    }

    pub fn authorize_permit(
        &self,
        permit: &StandingPermit,
        recipient: StrongDigest,
        patient_contact: ContactState,
        clinician_contact: ContactState,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        self.permits.authorize(
            permit.patient,
            permit.clinician,
            permit.purpose,
            permit.record_scope,
            recipient,
            patient_contact,
            clinician_contact,
            PolicyOutcome::Allow,
            now_unix,
        )
    }

    pub fn add_grant(&mut self, grant: CareGrant) -> Result<usize, QdnfError> {
        self.grants.add(grant)
    }

    pub fn seal(
        &self,
        label: &VerifiedLabel,
        projection: StrongDigest,
        purpose: StrongDigest,
        expires_unix: u64,
        ciphertext: &[u8],
        patient_contact: ContactState,
        clinician_contact: ContactState,
        now_unix: u64,
    ) -> Result<PayloadEnvelope, QdnfError> {
        seal_envelope(
            &self.grants,
            label,
            projection,
            purpose,
            expires_unix,
            ciphertext,
            PolicyOutcome::Allow,
            patient_contact,
            clinician_contact,
            now_unix,
        )
    }

    pub fn store_mailbox(
        &mut self,
        ct: &[u8],
        recipient: StrongDigest,
        bound: BoundGenerations,
    ) -> Result<usize, QdnfError> {
        self.mailbox.store_bound(ct, recipient, bound)
    }

    pub fn deliver_mailbox(
        &mut self,
        slot: usize,
        recipient: StrongDigest,
        live: LiveGenerations,
    ) -> Result<(), QdnfError> {
        self.mailbox.deliver_live(slot, recipient, live)
    }

    pub fn revoke_clinician(&mut self, clinician: StrongDigest) -> Result<(), QdnfError> {
        self.mailbox.revoke_clinician(clinician)
    }

    pub fn enqueue_offline(&mut self, slot: OfflineSlot, now: u64) -> Result<usize, QdnfError> {
        enqueue_offline(&mut self.offline, slot, now)
    }

    pub fn release_offline(
        &self,
        slot: usize,
        now: u64,
        patient_contact: ContactState,
        clinician_contact: ContactState,
    ) -> Result<(), QdnfError> {
        release_queued(
            &self.offline,
            slot,
            now,
            patient_contact,
            clinician_contact,
        )
    }
}

impl Default for ClinicalSession {
    fn default() -> Self {
        Self::new()
    }
}

fn digest(tag: u8) -> StrongDigest {
    let mut d = StrongDigest::ZERO;
    d.0[0] = tag;
    d.0[47] = 0xC1;
    d
}

fn grant_until(audience: StrongDigest, exp: u64) -> TemporalGrant {
    TemporalGrant {
        purpose_digest: digest(0x11),
        audience_digest: audience,
        authority_generation: 1,
        not_before_unix: 0,
        expires_unix: exp,
        profile: ProfileId::QDNF_CRYPTO_1,
    }
}

fn label(conf: Confidentiality, iss: StrongDigest) -> VerifiedLabel {
    let mut fields = LabelFields::request(conf, iss);
    if conf.requires_audience() {
        fields.audience = iss;
    }
    let mut buf = [0u8; 256];
    let n = encode_label_into(&fields, &mut buf).expect("encode");
    verify_label(fields, &buf[..n]).expect("verify")
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::net::qdnf::clinical::{
        network_asserts_clinical_review, notify_guardian_automatically,
    };

    fn host() -> PeerHost {
        PeerHost::new(4096).unwrap()
    }

    #[test]
    fn public_indexes_and_diagnosis_discovery_are_denied() {
        let mut c = host().clinical_session();
        let pair = ClinicalPair {
            patient: digest(1),
            clinician: digest(2),
            patient_contact: ContactState::Active,
            clinician_contact: ContactState::Active,
            route: digest(3),
        };
        assert!(c.admit_pair(pair).is_ok());
        assert_eq!(c.refuse_public_index(pair), Err(QdnfError::Denied));
        assert!(!public_patient_index_allowed());
        assert!(!vip_association_indexed());
        assert!(!diagnosis_discovery_allowed());
    }

    #[test]
    fn blocked_contact_cannot_release_permit() {
        let mut c = host().clinical_session();
        let clinician = digest(2);
        let p = StandingPermit {
            patient: digest(1),
            clinician,
            purpose: digest(3),
            record_scope: digest(4),
            grant: grant_until(clinician, 100),
        };
        c.add_permit(p).unwrap();
        assert_eq!(
            c.authorize_permit(
                &p,
                clinician,
                ContactState::Active,
                ContactState::Blocked,
                10
            ),
            Err(QdnfError::Denied)
        );
        assert!(c
            .authorize_permit(
                &p,
                clinician,
                ContactState::Active,
                ContactState::Active,
                10
            )
            .is_ok());
        c.set_live(LiveGenerations {
            source: Generation(1),
            policy: Generation(2),
            identity: Generation(1),
        });
        assert_eq!(
            c.authorize_permit(
                &p,
                clinician,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn envelope_mailbox_offline_backup_and_help() {
        let mut c = host().clinical_session();
        let clinician = digest(2);
        let patient = digest(1);
        c.add_grant(CareGrant {
            kind: CareGrantKind::Clinician,
            recipient: clinician,
            grant: grant_until(clinician, 100),
        })
        .unwrap();
        c.add_grant(CareGrant {
            kind: CareGrantKind::Referral,
            recipient: digest(8),
            grant: grant_until(digest(8), 100),
        })
        .unwrap();
        let lbl = label(Confidentiality::C2Sensitive, digest(1));
        assert_eq!(
            c.seal(
                &lbl,
                digest(0x21),
                digest(0x11),
                100,
                b"",
                ContactState::Active,
                ContactState::Active,
                10
            )
            .unwrap_err(),
            QdnfError::Malformed
        );
        let env = c
            .seal(
                &lbl,
                digest(0x21),
                digest(0x11),
                100,
                b"sealed-ct",
                ContactState::Active,
                ContactState::Active,
                10
            )
            .unwrap();
        assert_eq!(env.recipient_count(), 2);

        let bound = BoundGenerations::new(Generation(1), Generation(1), Generation(1));
        let slot = c.store_mailbox(b"mailbox-ct", clinician, bound).unwrap();
        c.revoke_clinician(clinician).unwrap();
        assert_eq!(
            c.deliver_mailbox(slot, clinician, bound.as_live()),
            Err(QdnfError::Revoked)
        );

        let mut c2 = host().clinical_session();
        let slot = c2.store_mailbox(b"mailbox-ct", clinician, bound).unwrap();
        let mut live = bound.as_live();
        live.identity = Generation(2);
        assert_eq!(
            c2.deliver_mailbox(slot, clinician, live),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(
            c2.deliver_mailbox(slot, digest(9), bound.as_live()),
            Err(QdnfError::Unauthorized)
        );
        assert!(c2.deliver_mailbox(slot, clinician, bound.as_live()).is_ok());
        assert!(!network_asserts_clinical_review());

        let off = c2
            .enqueue_offline(
                OfflineSlot {
                    recipient: clinician,
                    expires_unix: 20,
                    ciphertext_len: 8,
                    ciphertext_digest: digest(9),
                },
                1,
            )
            .unwrap();
        assert_eq!(
            c2.release_offline(off, 20, ContactState::Active, ContactState::Active),
            Err(QdnfError::Expired)
        );

        let stored = label(Confidentiality::C2Sensitive, digest(1));
        let mut dropped = *stored.fields();
        dropped.confidentiality = Confidentiality::C0Public;
        assert_eq!(
            restore_labelled_backup(&stored, &dropped),
            Err(QdnfError::Denied)
        );
        let mut out = LabelFields::request(Confidentiality::C0Public, digest(2));
        inherit_response_label(stored.fields(), &stored, &mut out).unwrap();
        assert_eq!(out.confidentiality, Confidentiality::C2Sensitive);

        assert!(!notify_guardian_automatically());
        assert_eq!(
            notify_guardian(
                CapacityMandatePolicy::IndependentConfidentialHelp,
                digest(3),
                ContactState::Active,
            ),
            Err(QdnfError::Denied)
        );
        assert!(admit_confidential_help(
            digest(8),
            CapacityMandatePolicy::IndependentConfidentialHelp,
            ContactState::Blocked,
        )
        .is_ok());

        let adapter = InterchangeAdapter {
            mapping_digest: digest(9),
            retains_full_label: false,
        };
        assert_eq!(export_interchange(&stored, &adapter), Err(QdnfError::Denied));
        assert_eq!(
            patient_record_associated(patient, digest(4), digest(9)),
            Err(QdnfError::Denied)
        );
        let pages = digest(5);
        let mut page = [0u8; STREAM_PAGE_BYTES as usize];
        let imaging = [7u8; 64];
        let expected = qualia_core_db::net::qdnf::clinical::digest_attachment_pages(&imaging)
            .unwrap();
        stream_attachment(&imaging, imaging.len() as u32, expected, &mut page).unwrap();
        let _ = pages;
    }
}

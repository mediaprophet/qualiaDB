//! Known-peer clinical session controls (NET-05.21–24).
//!
//! Implementation lives in [`crate::net::qdnf::clinical`]. This module re-exports
//! pairing, standing permits, ciphertext-only intermediaries and the encrypted
//! mailbox (bounded ciphertext bytes).

pub use crate::net::qdnf::clinical::{
    forward_ciphertext, generic_log_contains_medical_payload, intermediary_decrypts,
    public_patient_index_allowed, publish_relationship, vip_association_indexed, ClinicalBoundary,
    ClinicalPair, GenericLog, GenericLogRecord, IntermediaryRole, Mailbox, MailboxSlot,
    MailboxState, PairingTable, PermitTable, StandingPermit, MAX_SLOTS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::authority::{ContactState, PolicyOutcome, TemporalGrant};
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::types::{ProfileId, StrongDigest};

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x
    }
    fn grant_until(exp: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: d(0x11),
            audience_digest: StrongDigest::ZERO,
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: exp,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }
    fn pair(patient: u8, clinician: u8, blocked: bool) -> ClinicalPair {
        ClinicalPair {
            patient: d(patient),
            clinician: d(clinician),
            patient_contact: ContactState::Active,
            clinician_contact: if blocked {
                ContactState::Blocked
            } else {
                ContactState::Active
            },
            route: StrongDigest::ZERO,
        }
    }
    fn auth(
        t: &PermitTable,
        p: &StandingPermit,
        scope: StrongDigest,
        recip: StrongDigest,
        now: u64,
    ) -> Result<(), QdnfError> {
        t.authorize(
            p.patient,
            p.clinician,
            p.purpose,
            scope,
            recip,
            ContactState::Active,
            ContactState::Active,
            PolicyOutcome::Allow,
            now,
        )
    }

    #[test]
    fn net_05_21_private_pairing_no_public_index() {
        let mut table = PairingTable::new();
        let p = pair(1, 2, false);
        assert!(table.admit(p).is_ok());
        assert_eq!(table.index_public(p), Err(QdnfError::Denied));
        assert_eq!(
            publish_relationship(p.patient, p.clinician),
            Err(QdnfError::Denied)
        );
        assert!(!public_patient_index_allowed());
        assert!(!vip_association_indexed());
        let grant = grant_until(100);
        let route = d(9);
        assert_eq!(
            table.exchange_route(
                p.patient,
                p.clinician,
                route,
                PolicyOutcome::Allow,
                &grant,
                10
            ),
            Ok(route)
        );
        let mut blocked = PairingTable::new();
        let b = pair(1, 2, true);
        assert!(blocked.admit(b).is_ok());
        assert_eq!(
            blocked.exchange_route(
                b.patient,
                b.clinician,
                route,
                PolicyOutcome::Allow,
                &grant,
                10
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            table.exchange_route(d(3), d(4), route, PolicyOutcome::Allow, &grant, 10),
            Err(QdnfError::Unauthorized)
        );
        let mut full = PairingTable::new();
        let mut n = 1u8;
        while n <= 8 {
            assert!(full.admit(pair(n, n + 10, false)).is_ok());
            n += 1;
        }
        assert_eq!(full.admit(pair(20, 21, false)), Err(QdnfError::Capacity));
    }

    #[test]
    fn net_05_22_standing_scope_and_extra_recipient() {
        let mut t = PermitTable::new();
        let grant = grant_until(100);
        let p = StandingPermit {
            patient: d(1),
            clinician: d(2),
            purpose: d(3),
            record_scope: d(4),
            grant,
        };
        assert!(t.add(p).is_ok());
        assert!(auth(&t, &p, p.record_scope, p.clinician, 10).is_ok());
        assert!(auth(&t, &p, p.record_scope, p.patient, 10).is_ok());
        assert_eq!(
            auth(&t, &p, p.record_scope, d(9), 10),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            auth(&t, &p, d(8), p.clinician, 10),
            Err(QdnfError::Unauthorized)
        );
        assert!(t
            .add(StandingPermit {
                record_scope: d(8),
                grant,
                ..p
            })
            .is_ok());
        assert!(auth(&t, &p, d(8), p.clinician, 10).is_ok());
        assert!(t
            .add(StandingPermit {
                clinician: d(9),
                grant,
                ..p
            })
            .is_ok());
        assert!(t
            .authorize(
                p.patient,
                d(9),
                p.purpose,
                p.record_scope,
                d(9),
                ContactState::Active,
                ContactState::Active,
                PolicyOutcome::Allow,
                10,
            )
            .is_ok());
        assert!(t
            .add(StandingPermit {
                grant: grant_until(5),
                record_scope: d(7),
                ..p
            })
            .is_ok());
        assert_eq!(auth(&t, &p, d(7), p.clinician, 10), Err(QdnfError::Expired));
    }

    #[test]
    fn net_05_23_ciphertext_only_forward_and_boundary() {
        let ct = b"clinical-ciphertext";
        let mut log = GenericLog::new();
        let got = forward_ciphertext(ct, &mut log).expect("forward");
        let rec = log.last().expect("meta");
        assert_eq!(rec.ciphertext_len as usize, ct.len());
        assert_eq!(rec.ciphertext_digest, got);
        assert_eq!(got, sha384(ct));
        assert_eq!(log.append_ciphertext_bytes(ct), Err(QdnfError::Denied));
        assert!(!generic_log_contains_medical_payload());
        assert!(!intermediary_decrypts());
        assert!(!IntermediaryRole::NETWORK.decrypts);
        assert!(IntermediaryRole::NETWORK
            .forward(ct, &mut GenericLog::new())
            .is_ok());
        assert_eq!(
            IntermediaryRole { decrypts: true }.forward(ct, &mut GenericLog::new()),
            Err(QdnfError::Denied)
        );
        let svc = d(0x42);
        let boundary = ClinicalBoundary { service: svc };
        assert!(boundary.decrypt(svc, ct).is_ok());
        assert_eq!(boundary.decrypt(d(0x43), ct), Err(QdnfError::Denied));
    }

    #[test]
    fn net_05_24_mailbox_revoke_substitute_generation_states() {
        let ct = b"pending-encrypted";
        let clinician = d(2);
        let other = d(3);
        let mut mb = Mailbox::new();
        let slot = mb.store(ct, clinician, 1).expect("store");
        assert_eq!(mb.get(slot).unwrap().state, MailboxState::Stored);
        assert_eq!(mb.mark_reviewed(slot), Err(QdnfError::Denied));
        assert_eq!(
            mb.substitute_recipient(slot, other),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(mb.deliver(slot, other, 1), Err(QdnfError::Unauthorized));
        assert_eq!(
            mb.deliver(slot, clinician, 2),
            Err(QdnfError::StaleGeneration)
        );
        assert!(mb.deliver(slot, clinician, 1).is_ok());
        assert_eq!(mb.get(slot).unwrap().state, MailboxState::Delivered);
        assert!(mb.mark_reviewed(slot).is_ok());
        assert_eq!(mb.get(slot).unwrap().state, MailboxState::ClinicianReviewed);
        let mut pending = Mailbox::new();
        let s = pending.store(ct, clinician, 1).expect("pending");
        assert!(pending.revoke_clinician(clinician).is_ok());
        assert_eq!(pending.get(s).unwrap().state, MailboxState::Stored);
        assert_eq!(pending.deliver(s, clinician, 1), Err(QdnfError::Revoked));
        assert_eq!(pending.deliver(s, other, 1), Err(QdnfError::Unauthorized));
        assert_eq!(pending.get(s).unwrap().ciphertext_digest, sha384(ct));
        assert_eq!(pending.get(s).unwrap().ciphertext_len as usize, ct.len());
    }
}

//! Known-peer clinical session controls (NET-05.21–24).
//! Private directed pairing, standing permits, ciphertext-only intermediaries,
//! and an encrypted mailbox (length + digest only; no medical payload bytes).

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::authority::{admit_service, ContactState, PolicyOutcome, TemporalGrant};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Occupied pairing, permit, log, and mailbox slots.
pub const MAX_SLOTS: usize = 8;

#[rustfmt::skip]
pub const fn public_patient_index_allowed() -> bool { false }
#[rustfmt::skip]
pub const fn vip_association_indexed() -> bool { false }
#[rustfmt::skip]
pub const fn generic_log_contains_medical_payload() -> bool { false }
#[rustfmt::skip]
pub const fn intermediary_decrypts() -> bool { false }

fn first_empty<T: Copy>(slots: &[Option<T>; MAX_SLOTS]) -> Option<usize> {
    (0..MAX_SLOTS).find(|&i| slots[i].is_none())
}

/// Directed known patient/clinician pair. Not a public index record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClinicalPair {
    pub patient: StrongDigest,
    pub clinician: StrongDigest,
    pub patient_contact: ContactState,
    pub clinician_contact: ContactState,
    pub route: StrongDigest,
}

/// Eight-slot private pairing table.
#[derive(Clone, Copy, Debug)]
pub struct PairingTable {
    slots: [Option<ClinicalPair>; MAX_SLOTS],
}

#[rustfmt::skip]
impl PairingTable {
    pub const fn new() -> Self {
        Self { slots: [None; MAX_SLOTS] }
    }

    fn find(&self, patient: StrongDigest, clinician: StrongDigest) -> Option<usize> {
        (0..MAX_SLOTS).find(|&i| {
            self.slots[i].is_some_and(|p| p.patient == patient && p.clinician == clinician)
        })
    }

    /// Duplicate is idempotent. A ninth distinct pair is Capacity.
    pub fn admit(&mut self, pair: ClinicalPair) -> Result<usize, QdnfError> {
        if pair.patient == StrongDigest::ZERO || pair.clinician == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        if let Some(i) = self.find(pair.patient, pair.clinician) {
            self.slots[i] = Some(pair);
            return Ok(i);
        }
        let i = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[i] = Some(pair);
        Ok(i)
    }

    /// Direct route exchange. Pair must be admitted; Blocked contacts are Denied.
    pub fn exchange_route(
        &mut self, patient: StrongDigest, clinician: StrongDigest, route: StrongDigest,
        outcome: PolicyOutcome, grant: &TemporalGrant, now_unix: u64,
    ) -> Result<StrongDigest, QdnfError> {
        if route == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        let i = self.find(patient, clinician).ok_or(QdnfError::Unauthorized)?;
        let mut pair = self.slots[i].ok_or(QdnfError::Unauthorized)?;
        if pair.patient_contact == ContactState::Blocked
            || pair.clinician_contact == ContactState::Blocked
        {
            return Err(QdnfError::Denied);
        }
        grant.current_at(now_unix)?;
        admit_service(outcome, true)?;
        pair.route = route;
        self.slots[i] = Some(pair);
        Ok(route)
    }

    /// Public indexing of a known pair is always Denied.
    pub fn index_public(&self, _pair: ClinicalPair) -> Result<(), QdnfError> {
        Err(QdnfError::Denied)
    }
}

/// Relationship publication is always Denied (no public / VIP / edge index).
pub fn publish_relationship(
    _patient: StrongDigest,
    _clinician: StrongDigest,
) -> Result<(), QdnfError> {
    Err(QdnfError::Denied)
}

/// Routine transfer/reply between a pair, bound to purpose and record scope.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandingPermit {
    pub patient: StrongDigest,
    pub clinician: StrongDigest,
    pub purpose: StrongDigest,
    pub record_scope: StrongDigest,
    pub grant: TemporalGrant,
}

/// Eight-slot standing-permit table.
#[derive(Clone, Copy, Debug)]
pub struct PermitTable {
    slots: [Option<StandingPermit>; MAX_SLOTS],
}

#[rustfmt::skip]
impl PermitTable {
    pub const fn new() -> Self {
        Self { slots: [None; MAX_SLOTS] }
    }

    /// Duplicate (pair, purpose, scope) is idempotent.
    pub fn add(&mut self, permit: StandingPermit) -> Result<usize, QdnfError> {
        if permit.patient == StrongDigest::ZERO
            || permit.clinician == StrongDigest::ZERO
            || permit.purpose == StrongDigest::ZERO
            || permit.record_scope == StrongDigest::ZERO
        {
            return Err(QdnfError::Malformed);
        }
        if let Some(i) = (0..MAX_SLOTS).find(|&i| {
            self.slots[i].is_some_and(|p| {
                p.patient == permit.patient
                    && p.clinician == permit.clinician
                    && p.purpose == permit.purpose
                    && p.record_scope == permit.record_scope
            })
        }) {
            self.slots[i] = Some(permit);
            return Ok(i);
        }
        let idx = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[idx] = Some(permit);
        Ok(idx)
    }

    /// Extra recipients and other scopes are Unauthorized until a matching permit
    /// is added. Current grant required.
    pub fn authorize(
        &self, patient: StrongDigest, clinician: StrongDigest, purpose: StrongDigest,
        record_scope: StrongDigest, recipient: StrongDigest, outcome: PolicyOutcome, now_unix: u64,
    ) -> Result<(), QdnfError> {
        if recipient != clinician && recipient != patient {
            return Err(QdnfError::Unauthorized);
        }
        let found = (0..MAX_SLOTS).find_map(|i| {
            self.slots[i].filter(|p| {
                p.patient == patient
                    && p.clinician == clinician
                    && p.purpose == purpose
                    && p.record_scope == record_scope
            })
        });
        let p = found.ok_or(QdnfError::Unauthorized)?;
        p.grant.current_at(now_unix)?;
        admit_service(outcome, true)
    }
}

/// Network intermediary. Generic relays set `decrypts` to false.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntermediaryRole {
    pub decrypts: bool,
}

impl IntermediaryRole {
    pub const NETWORK: Self = Self { decrypts: false };

    /// Ciphertext-only forward. A decrypting intermediary is Denied.
    pub fn forward(self, ct: &[u8], log: &mut GenericLog) -> Result<StrongDigest, QdnfError> {
        if self.decrypts {
            return Err(QdnfError::Denied);
        }
        forward_ciphertext(ct, log)
    }
}

/// Declared clinical-service decryption boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClinicalBoundary {
    pub service: StrongDigest,
}

impl ClinicalBoundary {
    /// Admit decryption only when `service` matches. Does not emit plaintext.
    pub fn decrypt(&self, service: StrongDigest, ct: &[u8]) -> Result<(), QdnfError> {
        if ct.is_empty() {
            return Err(QdnfError::Malformed);
        }
        if self.service == StrongDigest::ZERO || self.service != service {
            return Err(QdnfError::Denied);
        }
        Ok(())
    }
}

/// Generic network/payment log: length and digest only.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLogRecord {
    pub ciphertext_len: u32,
    pub ciphertext_digest: StrongDigest,
}

/// Eight-slot generic log. Payload bytes are never stored.
#[derive(Clone, Copy, Debug)]
pub struct GenericLog {
    records: [Option<GenericLogRecord>; MAX_SLOTS],
}

impl GenericLog {
    pub const fn new() -> Self {
        Self {
            records: [None; MAX_SLOTS],
        }
    }

    pub fn last(&self) -> Option<GenericLogRecord> {
        (0..MAX_SLOTS).rev().find_map(|i| self.records[i])
    }

    /// Storing ciphertext bytes on a generic log is always Denied.
    pub fn append_ciphertext_bytes(&mut self, ct: &[u8]) -> Result<(), QdnfError> {
        let _ = ct;
        Err(QdnfError::Denied)
    }

    fn record_meta(&mut self, len: u32, digest: StrongDigest) -> Result<(), QdnfError> {
        let i = first_empty(&self.records).ok_or(QdnfError::Capacity)?;
        self.records[i] = Some(GenericLogRecord {
            ciphertext_len: len,
            ciphertext_digest: digest,
        });
        Ok(())
    }
}

/// Forward ciphertext. Records only length and digest on `log`.
pub fn forward_ciphertext(ct: &[u8], log: &mut GenericLog) -> Result<StrongDigest, QdnfError> {
    if ct.is_empty() {
        return Err(QdnfError::Malformed);
    }
    let len = u32::try_from(ct.len()).map_err(|_| QdnfError::Range)?;
    let digest = sha384(ct);
    log.record_meta(len, digest)?;
    Ok(digest)
}

/// Mailbox lifecycle. Stored cannot skip to ClinicianReviewed.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxState {
    Stored = 1,
    Delivered = 2,
    ClinicianReviewed = 3,
}

/// Encrypted mailbox slot. Ciphertext bytes are not retained.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailboxSlot {
    pub ciphertext_len: u32,
    pub ciphertext_digest: StrongDigest,
    pub recipient: StrongDigest,
    pub key_generation: u64,
    pub grant_revoked: bool,
    pub state: MailboxState,
}

/// Eight-slot pending encrypted mailbox.
#[derive(Clone, Copy, Debug)]
pub struct Mailbox {
    slots: [Option<MailboxSlot>; MAX_SLOTS],
}

#[rustfmt::skip]
impl Mailbox {
    pub const fn new() -> Self {
        Self { slots: [None; MAX_SLOTS] }
    }

    pub fn get(&self, slot: usize) -> Result<MailboxSlot, QdnfError> {
        let err = if slot >= MAX_SLOTS { QdnfError::Range } else { QdnfError::Closed };
        self.slots.get(slot).copied().flatten().ok_or(err)
    }

    /// Store ciphertext as length + digest + recipient. Pending state is Stored.
    pub fn store(&mut self, ct: &[u8], recipient: StrongDigest, key_generation: u64) -> Result<usize, QdnfError> {
        if ct.is_empty() || recipient == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        let len = u32::try_from(ct.len()).map_err(|_| QdnfError::Range)?;
        let i = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[i] = Some(MailboxSlot {
            ciphertext_len: len,
            ciphertext_digest: sha384(ct),
            recipient,
            key_generation,
            grant_revoked: false,
            state: MailboxState::Stored,
        });
        Ok(i)
    }

    /// Revoke clinician grants. Pending slots stay Stored.
    pub fn revoke_clinician(&mut self, clinician: StrongDigest) -> Result<(), QdnfError> {
        if clinician == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        for slot in self.slots.iter_mut() {
            if let Some(s) = slot {
                if s.recipient == clinician {
                    s.grant_revoked = true;
                }
            }
        }
        Ok(())
    }

    /// Deliver Stored material to the recorded recipient. No substitution.
    pub fn deliver(&mut self, slot: usize, recipient: StrongDigest, key_generation: u64) -> Result<(), QdnfError> {
        let mut s = self.get(slot)?;
        if recipient != s.recipient {
            return Err(QdnfError::Unauthorized);
        }
        if s.grant_revoked {
            return Err(QdnfError::Revoked);
        }
        if key_generation != s.key_generation {
            return Err(QdnfError::StaleGeneration);
        }
        match s.state {
            MailboxState::Stored => {
                s.state = MailboxState::Delivered;
                self.slots[slot] = Some(s);
                Ok(())
            }
            MailboxState::Delivered => Ok(()),
            MailboxState::ClinicianReviewed => Err(QdnfError::Conflict),
        }
    }

    /// ClinicianReviewed requires Delivered. Stored cannot skip.
    pub fn mark_reviewed(&mut self, slot: usize) -> Result<(), QdnfError> {
        let mut s = self.get(slot)?;
        match s.state {
            MailboxState::Stored => Err(QdnfError::Denied),
            MailboxState::Delivered => {
                s.state = MailboxState::ClinicianReviewed;
                self.slots[slot] = Some(s);
                Ok(())
            }
            MailboxState::ClinicianReviewed => Ok(()),
        }
    }

    /// Unapproved recipient substitution is always Unauthorized.
    pub fn substitute_recipient(&mut self, slot: usize, new_recipient: StrongDigest) -> Result<(), QdnfError> {
        let _ = (self.get(slot)?, new_recipient);
        Err(QdnfError::Unauthorized)
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use crate::net::qdnf::types::ProfileId;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x
    }
    fn grant_until(exp: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: d(0x11), audience_digest: d(0x12), authority_generation: 1,
            not_before_unix: 0, expires_unix: exp, profile: ProfileId::QDNF_CRYPTO_1,
        }
    }
    fn pair(patient: u8, clinician: u8, blocked: bool) -> ClinicalPair {
        ClinicalPair {
            patient: d(patient), clinician: d(clinician),
            patient_contact: ContactState::Active,
            clinician_contact: if blocked { ContactState::Blocked } else { ContactState::Active },
            route: StrongDigest::ZERO,
        }
    }
    fn auth(
        t: &PermitTable, p: &StandingPermit, scope: StrongDigest, recip: StrongDigest, now: u64,
    ) -> Result<(), QdnfError> {
        t.authorize(p.patient, p.clinician, p.purpose, scope, recip, PolicyOutcome::Allow, now)
    }

    #[test]
    fn net_05_21_private_pairing_no_public_index() {
        let mut table = PairingTable::new();
        let p = pair(1, 2, false);
        assert!(table.admit(p).is_ok());
        assert_eq!(table.index_public(p), Err(QdnfError::Denied));
        assert_eq!(publish_relationship(p.patient, p.clinician), Err(QdnfError::Denied));
        assert!(!public_patient_index_allowed());
        assert!(!vip_association_indexed());
        let grant = grant_until(100);
        let route = d(9);
        assert_eq!(table.exchange_route(p.patient, p.clinician, route, PolicyOutcome::Allow, &grant, 10), Ok(route));
        let mut blocked = PairingTable::new();
        let b = pair(1, 2, true);
        assert!(blocked.admit(b).is_ok());
        assert_eq!(blocked.exchange_route(b.patient, b.clinician, route, PolicyOutcome::Allow, &grant, 10), Err(QdnfError::Denied));
        assert_eq!(table.exchange_route(d(3), d(4), route, PolicyOutcome::Allow, &grant, 10), Err(QdnfError::Unauthorized));
        let mut full = PairingTable::new();
        let mut n = 1u8;
        while n <= 8 { assert!(full.admit(pair(n, n + 10, false)).is_ok()); n += 1; }
        assert_eq!(full.admit(pair(20, 21, false)), Err(QdnfError::Capacity));
    }

    #[test]
    fn net_05_22_standing_scope_and_extra_recipient() {
        let mut t = PermitTable::new();
        let grant = grant_until(100);
        let p = StandingPermit { patient: d(1), clinician: d(2), purpose: d(3), record_scope: d(4), grant };
        assert!(t.add(p).is_ok());
        assert!(auth(&t, &p, p.record_scope, p.clinician, 10).is_ok());
        assert!(auth(&t, &p, p.record_scope, p.patient, 10).is_ok());
        assert_eq!(auth(&t, &p, p.record_scope, d(9), 10), Err(QdnfError::Unauthorized));
        assert_eq!(auth(&t, &p, d(8), p.clinician, 10), Err(QdnfError::Unauthorized));
        assert!(t.add(StandingPermit { record_scope: d(8), grant, ..p }).is_ok());
        assert!(auth(&t, &p, d(8), p.clinician, 10).is_ok());
        assert!(t.add(StandingPermit { clinician: d(9), grant, ..p }).is_ok());
        assert!(t.authorize(p.patient, d(9), p.purpose, p.record_scope, d(9), PolicyOutcome::Allow, 10).is_ok());
        assert!(t.add(StandingPermit { grant: grant_until(5), record_scope: d(7), ..p }).is_ok());
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
        assert!(IntermediaryRole::NETWORK.forward(ct, &mut GenericLog::new()).is_ok());
        assert_eq!(IntermediaryRole { decrypts: true }.forward(ct, &mut GenericLog::new()), Err(QdnfError::Denied));
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
        assert_eq!(mb.substitute_recipient(slot, other), Err(QdnfError::Unauthorized));
        assert_eq!(mb.deliver(slot, other, 1), Err(QdnfError::Unauthorized));
        assert_eq!(mb.deliver(slot, clinician, 2), Err(QdnfError::StaleGeneration));
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

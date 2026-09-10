//! Mailbox store, deliver, review and application-ack transitions.

use super::super::{first_empty, MAX_SLOTS};
use super::slot::{network_asserts_clinical_review, MailboxSlot, MailboxState};
use crate::net::peer::replication::custody::{
    mark_application_acked, mark_delivered, mark_stored, CustodyState,
};
use crate::net::qdnf::contracts::{recheck_permit, BoundGenerations, LiveGenerations};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Eight-slot pending encrypted mailbox.
#[derive(Clone, Copy, Debug)]
pub struct Mailbox {
    slots: [Option<MailboxSlot>; MAX_SLOTS],
}

impl Mailbox {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
        }
    }

    pub fn get(&self, slot: usize) -> Result<MailboxSlot, QdnfError> {
        let err = if slot >= MAX_SLOTS {
            QdnfError::Range
        } else {
            QdnfError::Closed
        };
        self.slots.get(slot).copied().flatten().ok_or(err)
    }

    /// Copy ciphertext bytes into a Stored slot. Length/digest without bytes is rejected.
    pub fn store(
        &mut self,
        ct: &[u8],
        recipient: StrongDigest,
        key_generation: u64,
    ) -> Result<usize, QdnfError> {
        let slot = MailboxSlot::from_ciphertext(ct, recipient, key_generation)?;
        let i = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[i] = Some(slot);
        Ok(i)
    }

    /// Length + digest is not content. Never becomes Stored.
    pub fn store_len_digest(
        &mut self,
        ciphertext_len: u32,
        ciphertext_digest: StrongDigest,
        recipient: StrongDigest,
        key_generation: u64,
    ) -> Result<usize, QdnfError> {
        let _ = (ciphertext_len, ciphertext_digest, recipient, key_generation);
        match mark_stored(true, false) {
            Ok(_) => Err(QdnfError::Incomplete),
            Err(e) => Err(e),
        }
    }

    /// Revoke clinician grants. Pending slots stay Stored.
    pub fn revoke_clinician(&mut self, clinician: StrongDigest) -> Result<(), QdnfError> {
        if clinician == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(s) = self.slots[i].as_mut() {
                if s.recipient == clinician {
                    s.grant_revoked = true;
                }
            }
            i += 1;
        }
        Ok(())
    }

    /// Deliver Stored material to the recorded recipient. No substitution.
    /// Transport delivery does not mark ClinicianReviewed or ApplicationAcked.
    pub fn deliver(
        &mut self,
        slot: usize,
        recipient: StrongDigest,
        key_generation: u64,
    ) -> Result<(), QdnfError> {
        debug_assert!(!network_asserts_clinical_review());
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
        recheck_permit(
            s.bound,
            LiveGenerations {
                source: crate::net::qdnf::types::Generation(key_generation),
                policy: crate::net::qdnf::types::Generation(key_generation),
                identity: crate::net::qdnf::types::Generation(key_generation),
            },
        )?;
        match s.state {
            MailboxState::Stored => {
                s.custody = mark_delivered(s.custody)?;
                s.state = MailboxState::Delivered;
                self.slots[slot] = Some(s);
                Ok(())
            }
            MailboxState::Delivered => Ok(()),
            MailboxState::ClinicianReviewed => Err(QdnfError::Conflict),
        }
    }

    /// Commit/release recheck using live source/policy/identity generations (E12.5).
    pub fn deliver_live(
        &mut self,
        slot: usize,
        recipient: StrongDigest,
        live: LiveGenerations,
    ) -> Result<(), QdnfError> {
        let s = self.get(slot)?;
        recheck_permit(s.bound, live)?;
        self.deliver(slot, recipient, s.key_generation)
    }

    /// Store ciphertext bound to compiled source/policy/identity generations.
    pub fn store_bound(
        &mut self,
        ct: &[u8],
        recipient: StrongDigest,
        bound: BoundGenerations,
    ) -> Result<usize, QdnfError> {
        let i = self.store(ct, recipient, bound.source.0)?;
        if let Some(slot) = self.slots[i].as_mut() {
            slot.bound = bound;
        }
        Ok(i)
    }

    /// Application acknowledgement. Requires transport Delivered.
    /// Distinct from ClinicianReviewed.
    pub fn mark_application_acked(&mut self, slot: usize) -> Result<(), QdnfError> {
        let mut s = self.get(slot)?;
        s.custody = mark_application_acked(s.custody)?;
        self.slots[slot] = Some(s);
        Ok(())
    }

    /// ClinicianReviewed requires Delivered. Stored cannot skip.
    /// Network delivery never calls this.
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
    pub fn substitute_recipient(
        &mut self,
        slot: usize,
        new_recipient: StrongDigest,
    ) -> Result<(), QdnfError> {
        let _ = (self.get(slot)?, new_recipient);
        Err(QdnfError::Unauthorized)
    }

    /// Retained ciphertext for `slot`.
    pub fn ciphertext(&self, slot: usize) -> Result<&[u8], QdnfError> {
        match self.slots.get(slot) {
            None => Err(QdnfError::Range),
            Some(None) => Err(QdnfError::Closed),
            Some(Some(s)) => Ok(s.ciphertext_bytes()),
        }
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x
    }

    #[test]
    fn mailbox_stored_requires_ciphertext_bytes() {
        let mut ct = *b"pending-encrypted";
        let original = ct;
        let clinician = d(2);
        let mut mb = Mailbox::new();
        let slot = mb.store(&ct, clinician, 1).expect("store");
        ct[0] ^= 0xff;
        let s = mb.get(slot).unwrap();
        assert_eq!(s.state, MailboxState::Stored);
        assert_eq!(s.custody, CustodyState::Stored);
        assert!(s.bytes_present());
        assert_eq!(s.ciphertext_bytes(), original.as_slice());
        assert_eq!(mb.ciphertext(slot).unwrap(), original.as_slice());
        assert_eq!(s.ciphertext_digest, sha384(&original));
        assert_eq!(s.ciphertext_len as usize, original.len());
        assert_ne!(s.ciphertext_bytes(), ct.as_slice());
        assert!(!s.is_application_acked());
        assert!(!network_asserts_clinical_review());
    }

    #[test]
    fn length_digest_only_is_incomplete() {
        let clinician = d(2);
        let ct = b"not-copied";
        let mut mb = Mailbox::new();
        assert_eq!(
            mb.store_len_digest(ct.len() as u32, sha384(ct), clinician, 1),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(mb.get(0), Err(QdnfError::Closed));
        assert_eq!(mb.ciphertext(0), Err(QdnfError::Closed));
        assert_eq!(mark_stored(true, false), Err(QdnfError::Incomplete));
    }

    #[test]
    fn delivered_is_not_application_acked() {
        let ct = b"pending-encrypted";
        let clinician = d(2);
        let mut mb = Mailbox::new();
        let slot = mb.store(ct, clinician, 1).expect("store");
        assert_eq!(mb.mark_application_acked(slot), Err(QdnfError::Incomplete));
        assert!(mb.deliver(slot, clinician, 1).is_ok());
        let s = mb.get(slot).unwrap();
        assert_eq!(s.state, MailboxState::Delivered);
        assert_eq!(s.custody, CustodyState::Delivered);
        assert_ne!(s.custody, CustodyState::ApplicationAcked);
        assert!(!s.is_application_acked());
        assert_ne!(s.state, MailboxState::ClinicianReviewed);
        assert!(!network_asserts_clinical_review());
        assert_eq!(mb.ciphertext(slot).unwrap(), ct);
        assert!(mb.mark_application_acked(slot).is_ok());
        let acked = mb.get(slot).unwrap();
        assert!(acked.is_application_acked());
        assert_eq!(acked.custody, CustodyState::ApplicationAcked);
        assert_eq!(acked.state, MailboxState::Delivered);
        assert_ne!(acked.state, MailboxState::ClinicianReviewed);
        assert!(mb.mark_reviewed(slot).is_ok());
        let reviewed = mb.get(slot).unwrap();
        assert_eq!(reviewed.state, MailboxState::ClinicianReviewed);
        assert!(reviewed.is_application_acked());
        assert_eq!(reviewed.ciphertext_bytes(), ct);
    }

    #[test]
    fn mailbox_revoke_substitute_generation_states() {
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
        assert!(!mb.get(slot).unwrap().is_application_acked());
        assert!(!network_asserts_clinical_review());
        assert!(mb.mark_reviewed(slot).is_ok());
        assert_eq!(mb.get(slot).unwrap().state, MailboxState::ClinicianReviewed);
        let mut pending = Mailbox::new();
        let s = pending.store(ct, clinician, 1).expect("pending");
        assert!(pending.revoke_clinician(clinician).is_ok());
        assert_eq!(pending.get(s).unwrap().state, MailboxState::Stored);
        assert_eq!(pending.deliver(s, clinician, 1), Err(QdnfError::Revoked));
        assert_eq!(pending.get(s).unwrap().ciphertext_digest, sha384(ct));
        assert_eq!(pending.get(s).unwrap().ciphertext_len as usize, ct.len());
        assert_eq!(pending.get(s).unwrap().ciphertext_bytes(), ct);
    }

    #[test]
    fn live_policy_generation_advance_is_stale() {
        use crate::net::qdnf::types::Generation;
        let clinician = d(2);
        let mut mb = Mailbox::new();
        let bound = BoundGenerations::new(Generation(1), Generation(1), Generation(1));
        let slot = mb.store_bound(b"ct", clinician, bound).expect("store");
        let mut live = bound.as_live();
        live.policy = Generation(2);
        assert_eq!(
            mb.deliver_live(slot, clinician, live),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(mb.get(slot).unwrap().state, MailboxState::Stored);
        assert!(mb.deliver_live(slot, clinician, bound.as_live()).is_ok());
    }
}

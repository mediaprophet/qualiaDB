//! Encrypted mailbox: length + digest only. Transport never asserts review.

use super::{first_empty, require_active_mutual, MAX_SLOTS};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::authority::ContactState;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::VerifiedLabel;
use crate::net::qdnf::types::StrongDigest;
use sha2::{Digest, Sha384};

/// Page size for streaming attachment bytes. Matches replication originals.
pub const STREAM_PAGE_BYTES: u32 = 4096;

/// Transport delivery never asserts clinical review or care.
#[inline]
pub const fn network_asserts_clinical_review() -> bool {
    false
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

    /// Store ciphertext as length + digest + recipient. Pending state is Stored.
    pub fn store(
        &mut self,
        ct: &[u8],
        recipient: StrongDigest,
        key_generation: u64,
    ) -> Result<usize, QdnfError> {
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
    /// Transport delivery does not mark ClinicianReviewed.
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
    pub fn substitute_recipient(
        &mut self,
        slot: usize,
        new_recipient: StrongDigest,
    ) -> Result<(), QdnfError> {
        let _ = (self.get(slot)?, new_recipient);
        Err(QdnfError::Unauthorized)
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Offline queued ciphertext metadata. Expired packages stay sealed.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfflineSlot {
    pub recipient: StrongDigest,
    pub expires_unix: u64,
    pub ciphertext_len: u32,
    pub ciphertext_digest: StrongDigest,
}

#[derive(Clone, Copy, Debug)]
pub struct OfflineQueue {
    slots: [Option<OfflineSlot>; MAX_SLOTS],
}

impl OfflineQueue {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
        }
    }

    pub fn get(&self, slot: usize) -> Result<OfflineSlot, QdnfError> {
        if slot >= MAX_SLOTS {
            return Err(QdnfError::Range);
        }
        self.slots[slot].ok_or(QdnfError::Closed)
    }
}

impl Default for OfflineQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub fn enqueue_offline(
    queue: &mut OfflineQueue,
    slot: OfflineSlot,
    _now: u64,
) -> Result<usize, QdnfError> {
    if slot.recipient == StrongDigest::ZERO
        || slot.ciphertext_len == 0
        || slot.ciphertext_digest == StrongDigest::ZERO
    {
        return Err(QdnfError::Malformed);
    }
    let i = first_empty(&queue.slots).ok_or(QdnfError::Capacity)?;
    queue.slots[i] = Some(slot);
    Ok(i)
}

/// Release a queued package. Expiry and live Active contact are checked here.
pub fn release_queued(
    queue: &OfflineQueue,
    slot: usize,
    now: u64,
    patient_contact: ContactState,
    clinician_contact: ContactState,
) -> Result<(), QdnfError> {
    let item = queue.get(slot)?;
    if now >= item.expires_unix {
        return Err(QdnfError::Expired);
    }
    require_active_mutual(patient_contact, clinician_contact)?;
    Ok(())
}

fn digest_pages(bytes: &[u8]) -> StrongDigest {
    let mut hasher = Sha384::new();
    let mut off = 0usize;
    while off < bytes.len() {
        let end = core::cmp::min(off + STREAM_PAGE_BYTES as usize, bytes.len());
        hasher.update(&bytes[off..end]);
        off = end;
    }
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    d
}

/// Digest `bytes` in [`STREAM_PAGE_BYTES`] pages.
pub fn digest_attachment_pages(bytes: &[u8]) -> Result<StrongDigest, QdnfError> {
    if bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }
    Ok(digest_pages(bytes))
}

/// Stream caller-owned pages and verify the running SHA-384.
pub fn stream_attachment(
    bytes: &[u8],
    expected_len: u32,
    expected: StrongDigest,
    page: &mut [u8],
) -> Result<StrongDigest, QdnfError> {
    if expected == StrongDigest::ZERO || bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }
    let len = u32::try_from(bytes.len()).map_err(|_| QdnfError::Range)?;
    if len != expected_len {
        return Err(QdnfError::Range);
    }
    if page.is_empty() {
        return Err(QdnfError::Capacity);
    }
    let mut hasher = Sha384::new();
    let mut off = 0usize;
    let cap = if page.len() > STREAM_PAGE_BYTES as usize {
        STREAM_PAGE_BYTES as usize
    } else {
        page.len()
    };
    while off < bytes.len() {
        let want = core::cmp::min(cap, bytes.len() - off);
        page[..want].copy_from_slice(&bytes[off..off + want]);
        hasher.update(&page[..want]);
        off += want;
    }
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    if d != expected {
        return Err(QdnfError::Conflict);
    }
    Ok(d)
}

/// Patient-record association is independent of network identity.
pub fn patient_record_associated(
    patient: StrongDigest,
    record: StrongDigest,
    bound_patient: StrongDigest,
) -> Result<(), QdnfError> {
    if patient == StrongDigest::ZERO || record == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if patient != bound_patient {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

/// Reviewed interchange adapter. Networking does not encode a medical decision.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterchangeAdapter {
    pub mapping_digest: StrongDigest,
    pub retains_full_label: bool,
}

pub fn export_interchange(
    label: &VerifiedLabel,
    adapter: &InterchangeAdapter,
) -> Result<VerifiedLabel, QdnfError> {
    if adapter.mapping_digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if !adapter.retains_full_label {
        return Err(QdnfError::Denied);
    }
    Ok(*label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x
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
    }

    #[test]
    fn offline_expiry_and_inactive_contact() {
        let mut q = OfflineQueue::new();
        let slot = enqueue_offline(
            &mut q,
            OfflineSlot {
                recipient: d(2),
                expires_unix: 20,
                ciphertext_len: 8,
                ciphertext_digest: d(9),
            },
            1,
        )
        .unwrap();
        assert!(release_queued(&q, slot, 10, ContactState::Active, ContactState::Active).is_ok());
        assert_eq!(
            release_queued(&q, slot, 20, ContactState::Active, ContactState::Active),
            Err(QdnfError::Expired)
        );
        assert_eq!(
            release_queued(&q, slot, 10, ContactState::Active, ContactState::Suspended),
            Err(QdnfError::Denied)
        );
        assert_eq!(q.get(slot).unwrap().ciphertext_digest, d(9));
    }

    #[test]
    fn attachment_pages_match_sha384_and_association() {
        let bytes = [0x11u8; 80];
        let digest = digest_attachment_pages(&bytes).unwrap();
        assert_eq!(digest, sha384(&bytes));
        let mut page = [0u8; STREAM_PAGE_BYTES as usize];
        assert_eq!(
            stream_attachment(&bytes, 80, digest, &mut page).unwrap(),
            digest
        );
        let mut wrong = digest;
        wrong.0[0] ^= 1;
        assert_eq!(
            stream_attachment(&bytes, 80, wrong, &mut page),
            Err(QdnfError::Conflict)
        );
        assert!(patient_record_associated(d(1), d(3), d(1)).is_ok());
        assert_eq!(
            patient_record_associated(d(1), d(3), d(2)),
            Err(QdnfError::Denied)
        );
        assert!(!network_asserts_clinical_review());
        let mut fields = crate::net::qdnf::policy_labels::LabelFields::request(
            crate::net::qdnf::policy_labels::Confidentiality::C2Sensitive,
            d(1),
        );
        fields.audience = d(2);
        let mut buf = [0u8; 256];
        let n = crate::net::qdnf::policy_labels::encode_label_into(&fields, &mut buf).unwrap();
        let lab = crate::net::qdnf::policy_labels::verify_label(fields, &buf[..n]).unwrap();
        let adapter = InterchangeAdapter {
            mapping_digest: d(0x44),
            retains_full_label: true,
        };
        assert_eq!(export_interchange(&lab, &adapter).unwrap(), lab);
        assert_eq!(
            export_interchange(
                &lab,
                &InterchangeAdapter {
                    mapping_digest: d(0x44),
                    retains_full_label: false,
                }
            ),
            Err(QdnfError::Denied)
        );
    }
}

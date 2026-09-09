//! Encrypted mailbox: bounded ciphertext bytes, not length + digest only.
//! Transport never asserts review.

use super::{first_empty, require_active_mutual, MAX_SLOTS};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::authority::ContactState;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::VerifiedLabel;
use crate::net::qdnf::types::StrongDigest;
use sha2::{Digest, Sha384};

mod lifecycle;
mod slot;

pub use lifecycle::Mailbox;
pub use slot::{
    network_asserts_clinical_review, MailboxSlot, MailboxState, MAILBOX_CIPHERTEXT_BYTES,
    STREAM_PAGE_BYTES,
};

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

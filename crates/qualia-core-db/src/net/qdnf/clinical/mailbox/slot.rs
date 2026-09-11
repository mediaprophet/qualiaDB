//! Encrypted mailbox slot. Ciphertext bytes are retained in the slot.

use crate::crypto::network::digest::sha384;
use crate::net::peer::replication::custody::{mark_stored, CustodyState};
use crate::net::qdnf::contracts::BoundGenerations;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Page size for streaming attachment bytes. Matches replication originals.
pub const STREAM_PAGE_BYTES: u32 = 4096;

/// Inline ciphertext capacity. Equal to [`STREAM_PAGE_BYTES`].
pub const MAILBOX_CIPHERTEXT_BYTES: usize = STREAM_PAGE_BYTES as usize;

/// Transport delivery never asserts clinical review or care.
#[inline]
pub const fn network_asserts_clinical_review() -> bool {
    false
}

/// Mailbox lifecycle. Stored cannot skip to ClinicianReviewed.
/// Delivered is transport and is not ClinicianReviewed.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxState {
    Stored = 1,
    Delivered = 2,
    ClinicianReviewed = 3,
}

/// Encrypted mailbox slot. Bounded ciphertext bytes are retained inline.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailboxSlot {
    pub ciphertext: [u8; MAILBOX_CIPHERTEXT_BYTES],
    pub ciphertext_len: u32,
    pub ciphertext_digest: StrongDigest,
    pub recipient: StrongDigest,
    pub key_generation: u64,
    pub bound: BoundGenerations,
    pub grant_revoked: bool,
    pub state: MailboxState,
    pub custody: CustodyState,
}

impl MailboxSlot {
    /// Copy `ct` into a Stored slot. Length/digest without bytes is rejected.
    pub(super) fn from_ciphertext(
        ct: &[u8],
        recipient: StrongDigest,
        key_generation: u64,
    ) -> Result<Self, QdnfError> {
        if ct.is_empty() || recipient == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        if ct.len() > MAILBOX_CIPHERTEXT_BYTES {
            return Err(QdnfError::Capacity);
        }
        let custody = mark_stored(false, true)?;
        let mut ciphertext = [0u8; MAILBOX_CIPHERTEXT_BYTES];
        ciphertext[..ct.len()].copy_from_slice(ct);
        Ok(Self {
            ciphertext,
            ciphertext_len: ct.len() as u32,
            ciphertext_digest: sha384(ct),
            recipient,
            key_generation,
            bound: BoundGenerations::new(
                Generation(key_generation),
                Generation(key_generation),
                Generation(key_generation),
            ),
            grant_revoked: false,
            state: MailboxState::Stored,
            custody,
        })
    }

    /// Retained ciphertext prefix.
    #[inline]
    pub fn ciphertext_bytes(&self) -> &[u8] {
        &self.ciphertext[..self.ciphertext_len as usize]
    }

    /// True when this slot holds a non-empty ciphertext copy.
    #[inline]
    pub fn bytes_present(&self) -> bool {
        self.ciphertext_len > 0 && (self.ciphertext_len as usize) <= MAILBOX_CIPHERTEXT_BYTES
    }

    /// Application acknowledgement is distinct from transport Delivered.
    #[inline]
    pub fn is_application_acked(&self) -> bool {
        self.custody == CustodyState::ApplicationAcked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ciphertext_copies_bytes() {
        let mut rec = StrongDigest::ZERO;
        rec.0[0] = 2;
        let ct = b"slot-ct";
        let slot = MailboxSlot::from_ciphertext(ct, rec, 1).expect("slot");
        assert_eq!(slot.state, MailboxState::Stored);
        assert_eq!(slot.custody, CustodyState::Stored);
        assert_eq!(slot.ciphertext_bytes(), ct);
        assert!(slot.bytes_present());
        assert!(!slot.is_application_acked());
        assert!(!network_asserts_clinical_review());
    }
}

//! Bounded revocation snapshot verification for sensing authorization.
//!
//! An absent receipt is meaningful only after an external authority confirms a
//! complete, current, anti-rollback checkpoint. This module verifies bounded
//! contents and signatures; it does not implement durable checkpoint storage.

use sha2::{Digest, Sha256};

use crate::governance::consent_contract::{ConsentGrant, RevocationReceipt};

use super::authorization::{AuthorizationError, RevocationAuthority, TrustedUnixClock};

pub const MAX_REVOCATION_RECEIPTS: usize = 32;
const SNAPSHOT_DOMAIN: &[u8] = b"qualia:revocation-snapshot:v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevocationSnapshotError {
    TooManyReceipts,
    WrongGrant,
    InvalidReceipt,
    DuplicateReceipt,
    CheckpointMismatch,
    SnapshotDigestMismatch,
    AuthorityUnavailable,
    UnknownCheckpoint,
}

/// Fixed-size checkpoint claims supplied by storage/ledger code. The fields are
/// untrusted until a `RevocationCheckpointAuthority` accepts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevocationCheckpoint {
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    sequence: u64,
    record_count: u8,
    snapshot_digest: [u8; 32],
}

impl RevocationCheckpoint {
    pub const fn new(
        grant_id: [u8; 32],
        grant_digest: [u8; 32],
        sequence: u64,
        record_count: u8,
        snapshot_digest: [u8; 32],
    ) -> Self {
        Self {
            grant_id,
            grant_digest,
            sequence,
            record_count,
            snapshot_digest,
        }
    }

    pub const fn grant_id(self) -> [u8; 32] {
        self.grant_id
    }
    pub const fn grant_digest(self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
    pub const fn record_count(self) -> u8 {
        self.record_count
    }
    pub const fn snapshot_digest(self) -> [u8; 32] {
        self.snapshot_digest
    }
}

/// External trusted store/ledger checkpoint verifier. It must authenticate the
/// checkpoint, prove snapshot completeness (including absence), enforce freshness
/// against persisted anti-rollback state, and fail closed if any part is unknown.
pub trait RevocationCheckpointAuthority {
    fn verify_current_complete(
        &self,
        grant: &ConsentGrant,
        checkpoint: &RevocationCheckpoint,
        receipts: &[RevocationReceipt],
        current_time_unix: u64,
    ) -> Result<(), RevocationSnapshotError>;
}

/// Verified view for one grant and one accepted checkpoint. Construction is only
/// possible after local receipt checks and the external completeness/freshness gate.
pub struct VerifiedRevocationSnapshot<'g, 'r, 'a> {
    grant: &'g ConsentGrant,
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    checkpoint: RevocationCheckpoint,
    receipts: &'r [RevocationReceipt],
    authority: &'a dyn RevocationCheckpointAuthority,
}

impl<'g, 'r, 'a> VerifiedRevocationSnapshot<'g, 'r, 'a> {
    pub fn verify(
        grant: &'g ConsentGrant,
        checkpoint: RevocationCheckpoint,
        receipts: &'r [RevocationReceipt],
        authority: &'a dyn RevocationCheckpointAuthority,
        clock: &mut dyn TrustedUnixClock,
    ) -> Result<Self, RevocationSnapshotError> {
        if receipts.len() > MAX_REVOCATION_RECEIPTS {
            return Err(RevocationSnapshotError::TooManyReceipts);
        }
        let grant_digest = grant.compute_digest();
        if checkpoint.grant_id != grant.grant_id
            || checkpoint.grant_digest != grant_digest
            || usize::from(checkpoint.record_count) != receipts.len()
            || grant.grant_id == [0; 32]
        {
            return Err(RevocationSnapshotError::CheckpointMismatch);
        }
        if snapshot_digest(grant, receipts) != checkpoint.snapshot_digest {
            return Err(RevocationSnapshotError::SnapshotDigestMismatch);
        }
        for (index, receipt) in receipts.iter().enumerate() {
            if receipt.grant_id != grant.grant_id {
                return Err(RevocationSnapshotError::WrongGrant);
            }
            if receipts[..index]
                .iter()
                .any(|previous| previous.receipt_id == receipt.receipt_id)
            {
                return Err(RevocationSnapshotError::DuplicateReceipt);
            }
            receipt
                .verify_against_grant(grant)
                .map_err(|_| RevocationSnapshotError::InvalidReceipt)?;
        }
        let now = clock
            .now_unix()
            .map_err(|_| RevocationSnapshotError::AuthorityUnavailable)?;
        authority.verify_current_complete(grant, &checkpoint, receipts, now)?;
        Ok(Self {
            grant,
            grant_id: grant.grant_id,
            grant_digest,
            checkpoint,
            receipts,
            authority,
        })
    }

    pub const fn grant_id(&self) -> [u8; 32] {
        self.grant_id
    }
    pub const fn grant_digest(&self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn checkpoint(&self) -> RevocationCheckpoint {
        self.checkpoint
    }
    fn receipt_present(&self) -> bool {
        !self.receipts.is_empty()
    }
}

impl RevocationAuthority for VerifiedRevocationSnapshot<'_, '_, '_> {
    fn is_revoked(
        &self,
        grant_id: &[u8; 32],
        grant_digest: &[u8; 32],
        current_time_unix: u64,
    ) -> Result<bool, AuthorizationError> {
        if grant_id != &self.grant_id || grant_digest != &self.grant_digest {
            return Err(AuthorizationError::RevocationUnknown);
        }
        self.authority
            .verify_current_complete(
                self.grant,
                &self.checkpoint,
                self.receipts,
                current_time_unix,
            )
            .map_err(|_| AuthorizationError::RevocationUnknown)?;
        Ok(self.receipt_present())
    }
}

/// Deterministic digest over the ordered, bounded receipt slice and grant.
/// Checkpoint verifiers must use the same ordering and versioned domain.
pub fn snapshot_digest(grant: &ConsentGrant, receipts: &[RevocationReceipt]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(SNAPSHOT_DOMAIN);
    hash.update(grant.grant_id);
    hash.update(grant.compute_digest());
    hash.update((receipts.len() as u32).to_be_bytes());
    for receipt in receipts {
        hash.update(receipt.compute_digest());
        hash.update(receipt.signature);
    }
    hash.finalize().into()
}

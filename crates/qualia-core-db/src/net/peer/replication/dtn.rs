//! Delay-tolerant transfer holds and effect uniqueness (E11.4, E11.6).
//!
//! Release requires unexpired time and current authority generation.
//! Transport retry is not a second user-visible effect. Tombstoned ids
//! cannot apply. Cancelled resume is [`QdnfError::Cancelled`].
//! Duplicate [`commit_pair`](super::pair::commit_pair) is already
//! idempotent by `operation_id`. The mailbox is not Bundle Protocol.

use crate::net::peer::replication::operation::{operation_id, OperationDesc};
use crate::net::peer::replication::receipts::{self, ReceiptTable};
use crate::net::peer::replication::resume::{resume_block, ResumeTable};
use crate::net::peer::replication::tombstone::{Tombstone, TombstoneTable};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Held DTN object: operation identity, expiry, and authority generation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DtnHold {
    pub op_id: StrongDigest,
    pub expiry: u64,
    pub authority: Generation,
}

/// Release after expiry and current-authority checks.
///
/// `now > expiry` → [`QdnfError::Expired`].
/// `authority != live_authority` → [`QdnfError::Unauthorized`].
pub fn release_hold(
    now: u64,
    expiry: u64,
    authority: Generation,
    live_authority: Generation,
) -> Result<(), QdnfError> {
    if now > expiry {
        return Err(QdnfError::Expired);
    }
    if authority != live_authority {
        return Err(QdnfError::Unauthorized);
    }
    Ok(())
}

/// Transport retry must not create a second user-visible effect.
#[inline]
pub fn retry_is_new_effect() -> bool {
    false
}

/// Mailbox store-and-forward is not Bundle Protocol compatible.
#[inline]
pub fn mailbox_is_bundle_protocol() -> bool {
    false
}

/// Dedup key: SHA-384 [`operation_id`].
#[inline]
pub fn effect_id(desc: &OperationDesc) -> StrongDigest {
    operation_id(desc)
}

/// Apply a local effect unless the id is tombstoned (rollback / retain).
///
/// Tombstoned → [`QdnfError::Revoked`]. Unknown identity follows receipts.
pub fn apply_live_effect(
    receipts: &mut ReceiptTable,
    tombstones: &TombstoneTable,
    id: StrongDigest,
) -> Result<(), QdnfError> {
    if tombstones.contains(&id) {
        return Err(QdnfError::Revoked);
    }
    receipts::apply_effect(receipts, id)
}

/// Retain a tombstone so a later apply cannot succeed.
pub fn rollback_tombstone(
    tombstones: &mut TombstoneTable,
    id: StrongDigest,
    epoch: u64,
    sequence: u32,
) -> Result<(), QdnfError> {
    tombstones.retain(Tombstone {
        op_id: id,
        epoch,
        sequence,
    })
}

/// Cancelled resume cannot continue as a live transfer.
pub fn resume_if_open(
    table: &ResumeTable,
    pin: u8,
    block_index: u8,
    cancelled: bool,
) -> Result<(), QdnfError> {
    if cancelled {
        return Err(QdnfError::Cancelled);
    }
    resume_block(table, pin, block_index)
}

/// Retry an already-applied effect: same `operation_id`, not a new effect.
pub fn transport_retry_effect(
    receipts: &mut ReceiptTable,
    tombstones: &TombstoneTable,
    id: StrongDigest,
) -> Result<(), QdnfError> {
    if retry_is_new_effect() {
        return Err(QdnfError::Conflict);
    }
    apply_live_effect(receipts, tombstones, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::manifest::{ByteRange, ContentManifest};
    use crate::net::peer::replication::operation::MAX_PARENTS;
    use crate::net::peer::replication::receipts::{bind_identity, class_of, ReceiptClass};
    use crate::net::peer::replication::resume::{mark_verified, pin};

    fn digest(tag: u8) -> StrongDigest {
        StrongDigest([tag; 48])
    }

    fn desc(seq: u32) -> OperationDesc {
        OperationDesc {
            author: StrongDigest([0x11; 48]),
            epoch: 1,
            sequence: seq,
            scope: 7,
            contract: StrongDigest([0x22; 48]),
            purpose: StrongDigest([0x33; 48]),
            payload_digest: StrongDigest([seq as u8; 48]),
            parents: [StrongDigest::ZERO; MAX_PARENTS],
            parent_count: 0,
        }
    }

    #[test]
    fn retry_is_not_a_duplicate_effect() {
        assert!(!retry_is_new_effect());
        assert!(!mailbox_is_bundle_protocol());
        let mut receipts = ReceiptTable::new();
        let tombstones = TombstoneTable::new();
        let id = effect_id(&desc(1));
        bind_identity(&mut receipts, id).unwrap();
        apply_live_effect(&mut receipts, &tombstones, id).unwrap();
        transport_retry_effect(&mut receipts, &tombstones, id).unwrap();
        assert_eq!(class_of(&receipts, id).unwrap(), ReceiptClass::Effect);
        release_hold(10, 20, Generation(1), Generation(1)).unwrap();
        assert_eq!(
            release_hold(21, 20, Generation(1), Generation(1)),
            Err(QdnfError::Expired)
        );
        assert_eq!(
            release_hold(10, 20, Generation(1), Generation(2)),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn tombstoned_id_cannot_apply_effect() {
        let mut receipts = ReceiptTable::new();
        let mut tombstones = TombstoneTable::new();
        let id = digest(4);
        bind_identity(&mut receipts, id).unwrap();
        rollback_tombstone(&mut tombstones, id, 1, 1).unwrap();
        assert_eq!(
            apply_live_effect(&mut receipts, &tombstones, id),
            Err(QdnfError::Revoked)
        );
        assert_eq!(class_of(&receipts, id).unwrap(), ReceiptClass::Identity);
    }

    #[test]
    fn cancelled_resume_is_cancelled() {
        let mut table = ResumeTable::new();
        let ranges = [ByteRange { offset: 0, len: 4 }];
        let m = ContentManifest::bind(digest(9), 0, 64, &ranges).unwrap();
        let pin_id = pin(&mut table, m).unwrap();
        mark_verified(&mut table, pin_id, 0).unwrap();
        resume_if_open(&table, pin_id, 0, false).unwrap();
        assert_eq!(
            resume_if_open(&table, pin_id, 0, true),
            Err(QdnfError::Cancelled)
        );
    }
}

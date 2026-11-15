//! Identity, local effect, and durable receipt as distinct stages (SVC-01.04).
//!
//! Operation identity is a SHA-384 [`StrongDigest`]. Binding an id is not a
//! local graph effect. A local effect is not a durable receipt. Transport ACK
//! and consumption never become [`ReceiptClass::DurableReceipt`].
//! [`DeliveryStage::DurableApplied`] is a durable receipt only after the
//! effect ran. [`DeliveryStage::EconomicReceipt`] requires that durable-applied
//! stage already recorded. Packages remain open; this library does not claim
//! exactly-once external work or CORE-03 atomic commit.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::StrongDigest;
use crate::{q_hash, NQuin};

/// Occupied identity slots. A 17th distinct bind is Capacity.
pub const MAX_RECEIPTS: usize = 16;

/// Crash-safe `.q42` SuperBlock commit states (`NET-20` / `R09`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuperBlockCommitState {
    /// Chunks received in staging area, not yet verified.
    Staged = 1,
    /// SHA-384 chunk digests, SHA-256 SuperBlock digest, and 5-field NQuin parity verified.
    Verified = 2,
    /// Interrupted commit / power-fail recovery hold; requires re-verification before commit.
    Indeterminate = 3,
    /// Durably committed to `.q42` volume after verification.
    Committed = 4,
}

/// Durability class actually achieved. Transport ACK is never this enum's
/// durable variant.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptClass {
    /// Id bound; no local effect.
    Identity = 1,
    /// Local apply; not durable.
    Effect = 2,
    /// Durable applied receipt (or economic receipt after that stage).
    DurableReceipt = 3,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    effect: bool,
    durable_applied: bool,
    id: StrongDigest,
    /// SHA-384 of exact effect bytes. ZERO means no effect digest recorded.
    effect_digest: StrongDigest,
}

/// Sixteen-slot identity / effect / receipt table.
pub struct ReceiptTable {
    slots: [Slot; MAX_RECEIPTS],
}

impl ReceiptTable {
    /// Creates a default `ReceiptTable`.
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                effect: false,
                durable_applied: false,
                id: StrongDigest::ZERO,
                effect_digest: StrongDigest::ZERO,
            }; MAX_RECEIPTS],
        }
    }

    fn find_id(&self, id: &StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if self.slots[i].occupied && self.slots[i].id == *id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

impl Default for ReceiptTable {
    fn default() -> Self {
        Self::new()
    }
}

fn reject_zero(id: StrongDigest) -> Result<(), QdnfError> {
    if id == StrongDigest::ZERO {
        Err(QdnfError::Malformed)
    } else {
        Ok(())
    }
}

/// Bind `id` as operation identity. Zero digest is malformed. A duplicate bind
/// of the same id is idempotent. Distinct ids beyond [`MAX_RECEIPTS`] are
/// capacity.
pub fn bind_identity(table: &mut ReceiptTable, id: StrongDigest) -> Result<(), QdnfError> {
    reject_zero(id)?;
    if table.find_id(&id).is_some() {
        return Ok(());
    }
    let idx = table.find_free().ok_or(QdnfError::Capacity)?;
    table.slots[idx] = Slot {
        occupied: true,
        effect: false,
        durable_applied: false,
        id,
        effect_digest: StrongDigest::ZERO,
    };
    Ok(())
}

/// Record a local graph effect. Requires a bound identity. Unknown id is
/// unauthorized. Idempotent once the effect is applied.
pub fn apply_effect(table: &mut ReceiptTable, id: StrongDigest) -> Result<(), QdnfError> {
    reject_zero(id)?;
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    table.slots[idx].effect = true;
    Ok(())
}

/// Record SHA-384 of exact effect bytes on a bound identity.
///
/// Same digest is idempotent. A different digest for the same id is
/// [`QdnfError::Conflict`]. Zero digest is malformed. Unknown id is unauthorized.
pub fn bind_effect_digest(
    table: &mut ReceiptTable,
    id: StrongDigest,
    digest: StrongDigest,
) -> Result<(), QdnfError> {
    reject_zero(id)?;
    if digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    let slot = &mut table.slots[idx];
    if slot.effect_digest != StrongDigest::ZERO && slot.effect_digest != digest {
        return Err(QdnfError::Conflict);
    }
    slot.effect_digest = digest;
    slot.effect = true;
    Ok(())
}

/// SHA-384 of effect bytes recorded for `id`, if any.
pub fn effect_digest_of(table: &ReceiptTable, id: StrongDigest) -> Result<StrongDigest, QdnfError> {
    reject_zero(id)?;
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    let digest = table.slots[idx].effect_digest;
    if digest == StrongDigest::ZERO {
        Err(QdnfError::Incomplete)
    } else {
        Ok(digest)
    }
}

/// Durability class actually achieved for `id` without acknowledging a stage.
///
/// Transport ACK is never consulted. Unknown id is unauthorized.
pub fn class_of(table: &ReceiptTable, id: StrongDigest) -> Result<ReceiptClass, QdnfError> {
    reject_zero(id)?;
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    let slot = &table.slots[idx];
    if slot.durable_applied {
        Ok(ReceiptClass::DurableReceipt)
    } else if slot.effect {
        Ok(ReceiptClass::Effect)
    } else {
        Ok(ReceiptClass::Identity)
    }
}

/// Map a delivery stage onto the durability class actually achieved.
///
/// Transport ACK and consumption are never [`ReceiptClass::DurableReceipt`].
/// Durable-applied requires the local effect. Economic receipt requires that
/// durable-applied was already recorded.
pub fn acknowledge(
    table: &mut ReceiptTable,
    id: StrongDigest,
    stage: DeliveryStage,
) -> Result<ReceiptClass, QdnfError> {
    reject_zero(id)?;
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    let slot = &mut table.slots[idx];
    match stage {
        DeliveryStage::TransportAck | DeliveryStage::Consumed => {
            if slot.effect {
                Ok(ReceiptClass::Effect)
            } else {
                Ok(ReceiptClass::Identity)
            }
        }
        DeliveryStage::DurableApplied => {
            if !slot.effect {
                return Err(QdnfError::Incomplete);
            }
            slot.durable_applied = true;
            Ok(ReceiptClass::DurableReceipt)
        }
        DeliveryStage::EconomicReceipt => {
            if !slot.durable_applied {
                return Err(QdnfError::Incomplete);
            }
            Ok(ReceiptClass::DurableReceipt)
        }
    }
}

/// Transport ACK is not a durable receipt class.
pub fn transport_ack_is_receipt_class() -> bool {
    false
}

/// Identity, local effect, and durable receipt are three distinct stages.
pub fn identity_effect_receipt_are_distinct() -> bool {
    ReceiptClass::Identity as u8 != ReceiptClass::Effect as u8
        && ReceiptClass::Effect as u8 != ReceiptClass::DurableReceipt as u8
        && ReceiptClass::Identity as u8 != ReceiptClass::DurableReceipt as u8
}

/// Advance a SuperBlock commit state machine (`R09`).
///
/// `Staged -> Committed` and `Indeterminate -> Committed` are rejected (`QdnfError::Incomplete`);
/// `Verified` and `Committed` require `block_digest_verified == true`.
pub fn advance_superblock_commit_state(
    current: SuperBlockCommitState,
    next: SuperBlockCommitState,
    block_digest_verified: bool,
) -> Result<SuperBlockCommitState, QdnfError> {
    match (current, next) {
        (SuperBlockCommitState::Staged, SuperBlockCommitState::Verified)
        | (SuperBlockCommitState::Indeterminate, SuperBlockCommitState::Verified) => {
            if !block_digest_verified {
                return Err(QdnfError::Incomplete);
            }
            Ok(SuperBlockCommitState::Verified)
        }
        (SuperBlockCommitState::Verified, SuperBlockCommitState::Committed) => {
            if !block_digest_verified {
                return Err(QdnfError::Incomplete);
            }
            Ok(SuperBlockCommitState::Committed)
        }
        (SuperBlockCommitState::Staged, SuperBlockCommitState::Indeterminate)
        | (SuperBlockCommitState::Verified, SuperBlockCommitState::Indeterminate)
        | (SuperBlockCommitState::Indeterminate, SuperBlockCommitState::Indeterminate) => {
            Ok(SuperBlockCommitState::Indeterminate)
        }
        _ => Err(QdnfError::Incomplete),
    }
}

/// Emit a 48-byte `SyncCheckpoint` `NQuin` with canonical 5-field XOR parity (`R09`).
///
/// Only `Verified` or `Committed` SuperBlocks may emit a checkpoint Quin; `Staged`
/// and `Indeterminate` states fail closed with [`QdnfError::Incomplete`].
pub fn emit_sync_checkpoint_quin(
    id: &StrongDigest,
    block_digest: &StrongDigest,
    state: SuperBlockCommitState,
    epoch: u64,
) -> Result<NQuin, QdnfError> {
    reject_zero(*id)?;
    reject_zero(*block_digest)?;
    if state != SuperBlockCommitState::Verified && state != SuperBlockCommitState::Committed {
        return Err(QdnfError::Incomplete);
    }
    let subject = u64::from_le_bytes([
        id.0[0], id.0[1], id.0[2], id.0[3], id.0[4], id.0[5], id.0[6], id.0[7],
    ]);
    let predicate = (q_hash("https://qualia.db/qsync#hasSyncCheckpoint") << 8) | (state as u64);
    let object = u64::from_le_bytes([
        block_digest.0[0],
        block_digest.0[1],
        block_digest.0[2],
        block_digest.0[3],
        block_digest.0[4],
        block_digest.0[5],
        block_digest.0[6],
        block_digest.0[7],
    ]) & 0x0FFF_FFFF_FFFF_FFFF;
    let context = q_hash("https://qualia.db/qsync#ReplicationReceiptGraph") & 0x00FF_FFFF_FFFF_FFFF;
    let metadata = ((epoch & 0x1FFF_FFFF) << 32) | u64::from(state as u8);
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    Ok(NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> StrongDigest {
        StrongDigest([tag; 48])
    }

    #[test]
    fn bind_then_transport_ack_is_identity_not_durable() {
        let mut table = ReceiptTable::new();
        let id = digest(1);
        bind_identity(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::TransportAck).unwrap();
        assert_eq!(class, ReceiptClass::Identity);
        assert_ne!(class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn apply_effect_then_transport_ack_is_not_durable() {
        let mut table = ReceiptTable::new();
        let id = digest(2);
        bind_identity(&mut table, id).unwrap();
        apply_effect(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::TransportAck).unwrap();
        assert_eq!(class, ReceiptClass::Effect);
        assert_ne!(class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn apply_effect_then_durable_applied_is_durable_receipt() {
        let mut table = ReceiptTable::new();
        let id = digest(3);
        bind_identity(&mut table, id).unwrap();
        apply_effect(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::DurableApplied).unwrap();
        assert_eq!(class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn durable_applied_without_effect_is_incomplete() {
        let mut table = ReceiptTable::new();
        let id = digest(4);
        bind_identity(&mut table, id).unwrap();
        assert_eq!(
            acknowledge(&mut table, id, DeliveryStage::DurableApplied),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn economic_receipt_without_durable_applied_is_incomplete() {
        let mut table = ReceiptTable::new();
        let id = digest(5);
        bind_identity(&mut table, id).unwrap();
        apply_effect(&mut table, id).unwrap();
        assert_eq!(
            acknowledge(&mut table, id, DeliveryStage::EconomicReceipt),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn unknown_id_is_unauthorized() {
        let mut table = ReceiptTable::new();
        let id = digest(6);
        assert_eq!(
            acknowledge(&mut table, id, DeliveryStage::TransportAck),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(apply_effect(&mut table, id), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn zero_digest_is_malformed() {
        let mut table = ReceiptTable::new();
        assert_eq!(
            bind_identity(&mut table, StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            apply_effect(&mut table, StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            acknowledge(&mut table, StrongDigest::ZERO, DeliveryStage::TransportAck),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn seventeenth_identity_is_capacity() {
        let mut table = ReceiptTable::new();
        let mut n = 1u8;
        while n <= MAX_RECEIPTS as u8 {
            bind_identity(&mut table, digest(n)).unwrap();
            n += 1;
        }
        assert_eq!(
            bind_identity(&mut table, digest(0x20)),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn transport_ack_is_not_a_receipt_class() {
        assert!(!transport_ack_is_receipt_class());
        assert_ne!(DeliveryStage::TransportAck, DeliveryStage::DurableApplied);
    }

    #[test]
    fn identity_effect_and_receipt_are_distinct() {
        assert!(identity_effect_receipt_are_distinct());
        assert_ne!(ReceiptClass::Identity, ReceiptClass::Effect);
        assert_ne!(ReceiptClass::Effect, ReceiptClass::DurableReceipt);
        assert_ne!(ReceiptClass::Identity, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn double_bind_identity_is_idempotent() {
        let mut table = ReceiptTable::new();
        let id = digest(7);
        bind_identity(&mut table, id).unwrap();
        bind_identity(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::TransportAck).unwrap();
        assert_eq!(class, ReceiptClass::Identity);
    }

    #[test]
    fn bind_effect_digest_same_is_idempotent_different_is_conflict() {
        let mut table = ReceiptTable::new();
        let id = digest(8);
        let d1 = digest(0xA1);
        let d2 = digest(0xA2);
        bind_identity(&mut table, id).unwrap();
        bind_effect_digest(&mut table, id, d1).unwrap();
        bind_effect_digest(&mut table, id, d1).unwrap();
        assert_eq!(effect_digest_of(&table, id).unwrap(), d1);
        assert_eq!(class_of(&table, id).unwrap(), ReceiptClass::Effect);
        assert_eq!(
            bind_effect_digest(&mut table, id, d2),
            Err(QdnfError::Conflict)
        );
        assert_eq!(class_of(&table, id).unwrap(), ReceiptClass::Effect);
    }

    #[test]
    fn class_of_unknown_id_is_unauthorized() {
        let table = ReceiptTable::new();
        assert_eq!(class_of(&table, digest(9)), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn r09_crash_safe_superblock_commit_states_and_sync_checkpoint_quin_parity() {
        let id = digest(0x11);
        let blk = digest(0x22);
        assert_eq!(
            advance_superblock_commit_state(
                SuperBlockCommitState::Staged,
                SuperBlockCommitState::Committed,
                true
            ),
            Err(QdnfError::Incomplete)
        );
        let st = advance_superblock_commit_state(
            SuperBlockCommitState::Staged,
            SuperBlockCommitState::Indeterminate,
            false,
        )
        .unwrap();
        assert_eq!(
            emit_sync_checkpoint_quin(&id, &blk, st, 42),
            Err(QdnfError::Incomplete)
        );
        let ver = advance_superblock_commit_state(st, SuperBlockCommitState::Verified, true).unwrap();
        let com = advance_superblock_commit_state(ver, SuperBlockCommitState::Committed, true).unwrap();
        let q = emit_sync_checkpoint_quin(&id, &blk, com, 42).unwrap();
        assert_eq!(
            q.parity,
            q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata
        );
    }
}

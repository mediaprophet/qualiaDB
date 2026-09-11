//! Durable pairing of network operation, graph/artifact effect, and receipt.
//!
//! One owner binds three things under one SHA-384 [`operation_id`]:
//! the CORE-03 [`IntentTable`] intent, the exact effect bytes, and the
//! [`ReceiptClass`]. Transport ACK is never a durable receipt.
//!
//! # Crash / finality (CORE-03 [`IntentTable`] contract)
//!
//! Pairing is the in-memory CORE-03 intent contract. It does **not** claim
//! OS process-kill disk durability. [`crate::net::peer::replication::crash::disk_backend_crash_injected`]
//! is false.
//!
//! - **Identity-only after crash before effect:** restart exposes a bound
//!   operation identity. Receipt class is [`ReceiptClass::Identity`], not
//!   durable.
//! - **Effect without commit:** local effect may be recorded. Receipt class
//!   stays [`ReceiptClass::Effect`]. Not durable.
//! - **Commit without durable ACK:** recover as durable **only** if a
//!   commit marker is present in the recovered stream. This module does not
//!   invent a durable receipt from a live ACK. A commit marker is the
//!   durability evidence `recover_pair` re-applies.
//! - **Aborted or incomplete markers:** not durable.
//!
//! Restart that replays a commit marker plus a matching effect digest can
//! re-apply [`DeliveryStage::DurableApplied`]. Transport ACK after any of
//! these stages remains Identity or Effect.

use crate::crypto::network::digest::sha384;
use crate::net::peer::replication::operation::{
    operation_id, tx_from_operation_id, OpTable, OperationDesc,
};
use crate::net::peer::replication::receipts::{self, ReceiptClass, ReceiptTable};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::wal_intent::{
    bind_exact_object, ArtifactKind, IntentRecord, IntentTable, MarkerKind, TxId, MAX_INTENTS,
    MAX_OBJECT_BYTES,
};

/// Network operation + exact effect + receipt class under one operation id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurablePairing {
    pub op_id: StrongDigest,
    pub tx: TxId,
    pub effect_digest: StrongDigest,
    pub receipt_class: ReceiptClass,
}

fn pairing_from(
    op_id: StrongDigest,
    tx: TxId,
    effect_digest: StrongDigest,
    receipts: &ReceiptTable,
) -> Result<DurablePairing, QdnfError> {
    Ok(DurablePairing {
        op_id,
        tx,
        effect_digest,
        receipt_class: receipts::class_of(receipts, op_id)?,
    })
}

/// Stage intent + identity + local effect. Does not mark [`ReceiptClass::DurableReceipt`].
///
/// Empty `effect_bytes` is malformed. Duplicate `operation_id` is idempotent
/// when the effect digest matches, [`QdnfError::Conflict`] if it differs.
///
/// `ops` is the operation-identity table. Pairing owns [`IntentTable`] staging
/// of the exact effect bytes and does not call [`OpTable::submit`] (that
/// would occupy the same [`TxId`] with a different object).
pub fn stage_paired(
    intents: &mut IntentTable,
    receipts: &mut ReceiptTable,
    ops: &mut OpTable,
    desc: &OperationDesc,
    effect_bytes: &[u8],
    writer_id: u64,
    authority_generation: Generation,
) -> Result<DurablePairing, QdnfError> {
    if effect_bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }

    let effect_digest = sha384(effect_bytes);
    let op_id = operation_id(desc);
    let tx = tx_from_operation_id(&op_id);
    let _ = ops.get_committed(&op_id);

    match receipts::effect_digest_of(receipts, op_id) {
        Ok(existing) if existing == effect_digest => {
            return pairing_from(op_id, tx, effect_digest, receipts);
        }
        Ok(_) => return Err(QdnfError::Conflict),
        Err(QdnfError::Unauthorized) => {}
        Err(QdnfError::Incomplete) => {}
        Err(e) => return Err(e),
    }

    let mut copy = [0u8; MAX_OBJECT_BYTES];
    let object = bind_exact_object(ArtifactKind::Evidence, effect_bytes, &mut copy)?;
    match intents.stage_intent(tx, object, writer_id, authority_generation) {
        Ok(_) => {}
        Err(QdnfError::Conflict) => {
            if let Ok(existing) = receipts::effect_digest_of(receipts, op_id) {
                if existing != effect_digest {
                    return Err(QdnfError::Conflict);
                }
            }
        }
        Err(e) => return Err(e),
    }

    receipts::bind_identity(receipts, op_id)?;
    receipts::bind_effect_digest(receipts, op_id, effect_digest)?;
    pairing_from(op_id, tx, effect_digest, receipts)
}

/// Commit the staged intent, then ACK [`DeliveryStage::DurableApplied`].
///
/// Commit failure (including stale generation) does not ACK durable. Receipt
/// class stays below [`ReceiptClass::DurableReceipt`]. A second commit of an
/// already-committed tx is treated as already durable at the intent layer
/// ([`QdnfError::Conflict`]) and the durable ACK is still applied.
pub fn commit_pair(
    intents: &mut IntentTable,
    receipts: &mut ReceiptTable,
    pairing: DurablePairing,
    generation: Generation,
) -> Result<DurablePairing, QdnfError> {
    match intents.commit(pairing.tx, generation) {
        Ok(()) => {}
        Err(QdnfError::Conflict) => {}
        Err(e) => return Err(e),
    }
    let class = receipts::acknowledge(receipts, pairing.op_id, DeliveryStage::DurableApplied)?;
    Ok(DurablePairing {
        receipt_class: class,
        ..pairing
    })
}

/// Rebuild receipt class from a CORE-03 marker stream.
///
/// Uses [`IntentTable::recover_into`]. Committed marker + matching effect
/// digest → re-apply durable. Aborted or incomplete → not durable.
///
/// Recovered markers do not carry writer id or generation (`encode_marker`
/// does not encode them). Restaging into `intents` uses writer `0` and
/// [`Generation::ZERO`]. `ops` cannot be rebuilt from the marker stream
/// (no [`OperationDesc`] on the wire).
pub fn recover_pair(
    stream: &[u8],
    intents: &mut IntentTable,
    receipts: &mut ReceiptTable,
    ops: &mut OpTable,
    desc: &OperationDesc,
    effect_digest: StrongDigest,
) -> Result<DurablePairing, QdnfError> {
    if effect_digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }

    let op_id = operation_id(desc);
    let tx = tx_from_operation_id(&op_id);
    let _ = ops.get_committed(&op_id);

    let mut recovered = [IntentRecord::EMPTY; MAX_INTENTS];
    let n = IntentTable::recover_into(stream, &mut recovered)?;

    let mut terminal: Option<MarkerKind> = None;
    let mut matched_digest = StrongDigest::ZERO;
    let mut i = 0usize;
    while i < n {
        let rec = recovered[i];
        if rec.tx.bytes == tx.bytes {
            terminal = Some(rec.kind);
            matched_digest = rec.object.digest;
        }
        i += 1;
    }

    let kind = terminal.ok_or(QdnfError::Incomplete)?;
    let digest_matches = matched_digest == effect_digest;

    if digest_matches || kind == MarkerKind::Abort {
        restage_recovered(intents, tx, effect_digest, kind)?;
    }

    receipts::bind_identity(receipts, op_id)?;
    match kind {
        MarkerKind::Commit if digest_matches => {
            receipts::bind_effect_digest(receipts, op_id, effect_digest)?;
            let class = receipts::acknowledge(receipts, op_id, DeliveryStage::DurableApplied)?;
            Ok(DurablePairing {
                op_id,
                tx,
                effect_digest,
                receipt_class: class,
            })
        }
        MarkerKind::Intent if digest_matches => {
            receipts::bind_effect_digest(receipts, op_id, effect_digest)?;
            pairing_from(op_id, tx, effect_digest, receipts)
        }
        MarkerKind::Abort => pairing_from(op_id, tx, effect_digest, receipts),
        MarkerKind::Commit | MarkerKind::Intent => pairing_from(op_id, tx, effect_digest, receipts),
    }
}

fn restage_recovered(
    intents: &mut IntentTable,
    tx: TxId,
    effect_digest: StrongDigest,
    kind: MarkerKind,
) -> Result<(), QdnfError> {
    let object = crate::wal_intent::ExactObjectRef {
        kind: ArtifactKind::Evidence,
        digest: effect_digest,
        byte_len: 0,
    };
    match intents.stage_intent(tx, object, 0, Generation::ZERO) {
        Ok(_) => {}
        Err(QdnfError::Conflict) => return Ok(()),
        Err(e) => return Err(e),
    }
    match kind {
        MarkerKind::Commit => match intents.commit(tx, Generation::ZERO) {
            Ok(()) => Ok(()),
            Err(QdnfError::Conflict) => Ok(()),
            Err(e) => Err(e),
        },
        MarkerKind::Abort => match intents.abort(tx, Generation::ZERO) {
            Ok(()) => Ok(()),
            Err(QdnfError::Conflict) => Ok(()),
            Err(e) => Err(e),
        },
        MarkerKind::Intent => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::crash::disk_backend_crash_injected;
    use crate::net::peer::replication::operation::MAX_PARENTS;
    use crate::wal_intent::MARKER_LEN;

    fn sample(seq: u32) -> OperationDesc {
        OperationDesc {
            author: StrongDigest([0x11; 48]),
            epoch: 1,
            sequence: seq,
            scope: 7,
            contract: StrongDigest([0x22; 48]),
            purpose: StrongDigest([0x33; 48]),
            payload_digest: StrongDigest([0x44; 48]),
            parents: [StrongDigest::ZERO; MAX_PARENTS],
            parent_count: 0,
        }
    }

    fn encode(kind: MarkerKind, tx: TxId, digest: &StrongDigest) -> [u8; MARKER_LEN] {
        let mut buf = [0u8; MARKER_LEN];
        IntentTable::encode_marker(kind, tx, digest, &mut buf).unwrap();
        buf
    }

    #[test]
    fn empty_bytes_are_malformed() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        assert_eq!(
            stage_paired(
                &mut intents,
                &mut receipts,
                &mut ops,
                &sample(1),
                &[],
                9,
                Generation(1),
            ),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn same_op_id_different_effect_is_conflict() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let desc = sample(2);
        let gen = Generation(1);
        stage_paired(
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            b"alpha",
            4,
            gen,
        )
        .unwrap();
        assert_eq!(
            stage_paired(
                &mut intents,
                &mut receipts,
                &mut ops,
                &desc,
                b"beta!",
                4,
                gen,
            ),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn same_op_id_same_effect_is_idempotent() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let desc = sample(3);
        let gen = Generation(1);
        let first = stage_paired(
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            b"same-bytes",
            5,
            gen,
        )
        .unwrap();
        let second = stage_paired(
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            b"same-bytes",
            5,
            gen,
        )
        .unwrap();
        assert_eq!(first.op_id, second.op_id);
        assert_eq!(first.effect_digest, second.effect_digest);
        assert_eq!(first.receipt_class, ReceiptClass::Effect);
        assert_eq!(second.receipt_class, ReceiptClass::Effect);
    }

    #[test]
    fn transport_ack_after_stage_is_never_durable() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let desc = sample(4);
        let pairing = stage_paired(
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            b"effect",
            6,
            Generation(1),
        )
        .unwrap();
        let class =
            receipts::acknowledge(&mut receipts, pairing.op_id, DeliveryStage::TransportAck)
                .unwrap();
        assert_ne!(class, ReceiptClass::DurableReceipt);
        assert!(class == ReceiptClass::Identity || class == ReceiptClass::Effect);
        assert_eq!(class, ReceiptClass::Effect);
    }

    #[test]
    fn stale_generation_cannot_commit() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let desc = sample(5);
        let old = Generation(1);
        let pairing = stage_paired(
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            b"stale",
            9,
            old,
        )
        .unwrap();
        intents.withdraw_authority(9, Generation(2)).unwrap();
        assert_eq!(
            commit_pair(&mut intents, &mut receipts, pairing, old),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(
            receipts::class_of(&receipts, pairing.op_id).unwrap(),
            ReceiptClass::Effect
        );
        assert_ne!(
            receipts::class_of(&receipts, pairing.op_id).unwrap(),
            ReceiptClass::DurableReceipt
        );
    }

    #[test]
    fn recover_commit_marker_is_durable_abort_is_not() {
        assert!(!disk_backend_crash_injected());
        let desc = sample(6);
        let effect = b"recover-me";
        let effect_digest = sha384(effect);
        let op_id = operation_id(&desc);
        let tx = tx_from_operation_id(&op_id);

        let commit_log = encode(MarkerKind::Commit, tx, &effect_digest);
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let durable = recover_pair(
            &commit_log,
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            effect_digest,
        )
        .unwrap();
        assert_eq!(durable.receipt_class, ReceiptClass::DurableReceipt);
        assert_eq!(
            intents.commit(tx, Generation::ZERO),
            Err(QdnfError::Conflict)
        );

        let abort_log = encode(MarkerKind::Abort, tx, &effect_digest);
        let mut intents_a = IntentTable::new();
        let mut receipts_a = ReceiptTable::new();
        let mut ops_a = OpTable::new();
        let aborted = recover_pair(
            &abort_log,
            &mut intents_a,
            &mut receipts_a,
            &mut ops_a,
            &desc,
            effect_digest,
        )
        .unwrap();
        assert_ne!(aborted.receipt_class, ReceiptClass::DurableReceipt);
    }
}

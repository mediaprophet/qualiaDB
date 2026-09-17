//! Recipe D commit adapter: verify payload bytes, then pair+commit.
//!
//! Payload SHA-384 must equal `expected_digest` before any durable staging.
//! Mismatch is [`QdnfError::Conflict`]: nothing committed, receipt not
//! durable. Transport ACK is never a durable receipt.
//!
//! Recovery [`resume_from_log`] streams CORE-03 markers and deduplicates by
//! the 16-byte transaction key derived from operation identity. Markers do
//! not carry the 48-byte operation id; receipt replay keys by the marker
//! object digest (the effect digest pairing wrote). [`super::pair::recover_pair`]
//! rebinds the real operation id when the descriptor is available.

use crate::crypto::network::digest::sha384;
use crate::net::peer::replication::operation::{OpTable, OperationDesc};
use crate::net::peer::replication::pair::{commit_pair, stage_paired, DurablePairing};
use crate::net::peer::replication::receipts::{self, ReceiptTable};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::wal_intent::{ArtifactKind, IntentRecord, IntentTable, MarkerKind, MAX_INTENTS};

fn verify_payload(payload: &[u8], expected: StrongDigest) -> Result<(), QdnfError> {
    if payload.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if sha384(payload) != expected {
        Err(QdnfError::Conflict)
    } else {
        Ok(())
    }
}

/// Verify payload, stage the pairing, commit the intent, ACK durable applied.
pub fn commit_verified_block(
    intents: &mut IntentTable,
    receipts: &mut ReceiptTable,
    ops: &mut OpTable,
    desc: &OperationDesc,
    payload: &[u8],
    expected_digest: StrongDigest,
    writer_id: u64,
    generation: Generation,
) -> Result<DurablePairing, QdnfError> {
    verify_payload(payload, expected_digest)?;
    let pairing = stage_paired(intents, receipts, ops, desc, payload, writer_id, generation)?;
    commit_pair(intents, receipts, pairing, generation)
}

/// Stream markers, dedupe by operation identity (`TxId`), replay receipts.
///
/// Duplicate markers for the same tx keep the last kind (WAL order). Torn
/// trailing bytes are ignored by [`IntentTable::recover_into`].
pub fn resume_from_log(
    log_bytes: &[u8],
    intents: &mut IntentTable,
    receipts: &mut ReceiptTable,
    ops: &mut OpTable,
) -> Result<usize, QdnfError> {
    let mut recovered = [IntentRecord::EMPTY; MAX_INTENTS];
    let n = IntentTable::recover_into(log_bytes, &mut recovered)?;

    let mut unique = [IntentRecord::EMPTY; MAX_INTENTS];
    let mut unique_n = 0usize;
    let mut i = 0usize;
    while i < n {
        let rec = recovered[i];
        let mut found = None;
        let mut u = 0usize;
        while u < unique_n {
            if unique[u].tx.bytes == rec.tx.bytes {
                found = Some(u);
                break;
            }
            u += 1;
        }
        match found {
            Some(idx) => unique[idx] = rec,
            None => {
                if unique_n >= MAX_INTENTS {
                    return Err(QdnfError::Capacity);
                }
                unique[unique_n] = rec;
                unique_n += 1;
            }
        }
        i += 1;
    }

    let mut applied = 0usize;
    let mut u = 0usize;
    while u < unique_n {
        replay_unique(intents, receipts, ops, unique[u])?;
        applied += 1;
        u += 1;
    }
    Ok(applied)
}

fn replay_unique(
    intents: &mut IntentTable,
    receipts: &mut ReceiptTable,
    ops: &mut OpTable,
    rec: IntentRecord,
) -> Result<(), QdnfError> {
    let id = rec.object.digest;
    if id == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let _ = ops.get_committed(&id);

    let object = crate::wal_intent::ExactObjectRef {
        kind: ArtifactKind::Evidence,
        digest: id,
        byte_len: rec.object.byte_len,
    };
    match intents.stage_intent(rec.tx, object, 0, Generation::ZERO) {
        Ok(_) => {}
        Err(QdnfError::Conflict) => {}
        Err(e) => return Err(e),
    }
    match rec.kind {
        MarkerKind::Commit => match intents.commit(rec.tx, Generation::ZERO) {
            Ok(()) => {}
            Err(QdnfError::Conflict) => {}
            Err(e) => return Err(e),
        },
        MarkerKind::Abort => match intents.abort(rec.tx, Generation::ZERO) {
            Ok(()) => {}
            Err(QdnfError::Conflict) => {}
            Err(e) => return Err(e),
        },
        MarkerKind::Intent => {}
    }

    receipts::bind_identity(receipts, id)?;
    if rec.kind == MarkerKind::Commit {
        receipts::bind_effect_digest(receipts, id, id)?;
        let class = receipts::acknowledge(receipts, id, DeliveryStage::DurableApplied)?;
        if class != receipts::ReceiptClass::DurableReceipt {
            return Err(QdnfError::Incomplete);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::operation::{operation_id, MAX_PARENTS};
    use crate::net::peer::replication::receipts::ReceiptClass;
    use crate::wal_intent::{TxId, MARKER_LEN};

    fn sample(seq: u32) -> OperationDesc {
        OperationDesc {
            author: StrongDigest([0x21; 48]),
            epoch: 1,
            sequence: seq,
            scope: 9,
            contract: StrongDigest([0x32; 48]),
            purpose: StrongDigest([0x43; 48]),
            payload_digest: StrongDigest([0x54; 48]),
            parents: [StrongDigest::ZERO; MAX_PARENTS],
            parent_count: 0,
        }
    }

    #[test]
    fn matching_bytes_and_commit_are_durable_receipt() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let desc = sample(1);
        let payload = b"verified-block-bytes";
        let expected = sha384(payload);
        let gen = Generation(1);
        let pairing = commit_verified_block(
            &mut intents,
            &mut receipts,
            &mut ops,
            &desc,
            payload,
            expected,
            11,
            gen,
        )
        .unwrap();
        assert_eq!(pairing.receipt_class, ReceiptClass::DurableReceipt);
        assert_eq!(pairing.effect_digest, expected);
        assert_eq!(intents.commit(pairing.tx, gen), Err(QdnfError::Conflict));
        assert_eq!(
            receipts::class_of(&receipts, pairing.op_id).unwrap(),
            ReceiptClass::DurableReceipt
        );
    }

    #[test]
    fn tampered_payload_is_conflict_and_not_committed() {
        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let desc = sample(2);
        let good = b"canonical-bytes";
        let expected = sha384(good);
        let mut bad = *good;
        bad[0] ^= 0xFF;
        let gen = Generation(1);
        assert_eq!(
            commit_verified_block(
                &mut intents,
                &mut receipts,
                &mut ops,
                &desc,
                &bad,
                expected,
                12,
                gen,
            ),
            Err(QdnfError::Conflict)
        );
        let op_id = operation_id(&desc);
        assert_eq!(
            receipts::class_of(&receipts, op_id),
            Err(QdnfError::Unauthorized)
        );
        let tx = crate::net::peer::replication::operation::tx_from_operation_id(&op_id);
        assert_eq!(intents.commit(tx, gen), Err(QdnfError::Incomplete));
    }

    #[test]
    fn resume_from_log_dedupes_by_operation_identity() {
        let payload = b"log-effect";
        let digest = sha384(payload);
        let tx = TxId { bytes: [0x77; 16] };
        let mut stream = [0u8; MARKER_LEN * 3];
        IntentTable::encode_marker(MarkerKind::Intent, tx, &digest, &mut stream[..MARKER_LEN])
            .unwrap();
        IntentTable::encode_marker(
            MarkerKind::Commit,
            tx,
            &digest,
            &mut stream[MARKER_LEN..MARKER_LEN * 2],
        )
        .unwrap();
        IntentTable::encode_marker(
            MarkerKind::Commit,
            tx,
            &digest,
            &mut stream[MARKER_LEN * 2..MARKER_LEN * 3],
        )
        .unwrap();

        let mut intents = IntentTable::new();
        let mut receipts = ReceiptTable::new();
        let mut ops = OpTable::new();
        let n = resume_from_log(&stream, &mut intents, &mut receipts, &mut ops).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            receipts::class_of(&receipts, digest).unwrap(),
            ReceiptClass::DurableReceipt
        );
        assert_eq!(
            intents.commit(tx, Generation::ZERO),
            Err(QdnfError::Conflict)
        );
    }
}

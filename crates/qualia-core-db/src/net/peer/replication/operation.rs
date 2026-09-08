//! Bounded QSync operation descriptors and a 16-slot replay table.
//!
//! Operation identity is SHA-384 over a length-delimited encoding of the
//! descriptor. The digest is never truncated to SHA-256 or `q_hash`. A 16-byte
//! [`TxId`] is derived from the first 16 bytes of that digest for CORE-03
//! intent staging only; it is not the operation identity.

use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::wal_intent::{bind_exact_object, ArtifactKind, IntentTable, TxId};

pub const MAX_OPS: usize = 16;
pub const MAX_PARENTS: usize = 4;

const OP_ID_DOMAIN: &[u8] = b"qdnf:sync:op-id:v1-pq";

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationDesc {
    pub author: StrongDigest,
    pub epoch: u64,
    pub sequence: u32,
    pub scope: u64,
    pub contract: StrongDigest,
    pub purpose: StrongDigest,
    pub payload_digest: StrongDigest,
    pub parents: [StrongDigest; MAX_PARENTS],
    pub parent_count: u8,
}

impl OperationDesc {
    pub const EMPTY: Self = Self {
        author: StrongDigest::ZERO,
        epoch: 0,
        sequence: 0,
        scope: 0,
        contract: StrongDigest::ZERO,
        purpose: StrongDigest::ZERO,
        payload_digest: StrongDigest::ZERO,
        parents: [StrongDigest::ZERO; MAX_PARENTS],
        parent_count: 0,
    };
}

/// SHA-384 identity of `desc`. Parents beyond `parent_count` are omitted.
/// Bounded encoding cannot overflow [`Transcript`]; failure is a programming error.
pub fn operation_id(desc: &OperationDesc) -> StrongDigest {
    transcript_operation_id(desc).expect("bounded QSync operation transcript")
}

/// Local CORE-03 transaction key: first 16 bytes of the SHA-384 operation id.
/// This is not a truncated security identifier for QSync itself.
pub fn tx_from_operation_id(id: &StrongDigest) -> TxId {
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&id.0[..16]);
    TxId { bytes }
}

pub fn transport_ack_is_durable() -> bool {
    false
}

pub fn source_signature_reusable_after_redaction() -> bool {
    false
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    committed: bool,
    generation: Generation,
    id: StrongDigest,
    desc: OperationDesc,
}

pub struct OpTable {
    slots: [Slot; MAX_OPS],
}

impl OpTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                committed: false,
                generation: Generation::ZERO,
                id: StrongDigest::ZERO,
                desc: OperationDesc::EMPTY,
            }; MAX_OPS],
        }
    }

    pub fn submit(
        &mut self,
        desc: OperationDesc,
        intents: &mut IntentTable,
        writer_id: u64,
        gen: Generation,
    ) -> Result<StrongDigest, QdnfError> {
        if desc.sequence == u32::MAX {
            return Err(QdnfError::Range);
        }
        if desc.parent_count as usize > MAX_PARENTS {
            return Err(QdnfError::Range);
        }

        let id = operation_id(&desc);

        if let Some(idx) = self.find_id(&id) {
            // Identical authorized retry: keep the original slot.
            let _ = idx;
            return Ok(id);
        }

        if self.unauthorized_epoch_replacement(&desc) {
            return Err(QdnfError::Unauthorized);
        }

        let free = self.find_free().ok_or(QdnfError::Capacity)?;
        let tx = tx_from_operation_id(&id);
        let mut copy = [0u8; 512];
        let object = bind_exact_object(ArtifactKind::Operation, &id.0, &mut copy)?;
        intents.stage_intent(tx, object, writer_id, gen)?;

        self.slots[free] = Slot {
            occupied: true,
            committed: false,
            generation: gen,
            id,
            desc,
        };
        Ok(id)
    }

    pub fn commit(
        &mut self,
        id: StrongDigest,
        intents: &mut IntentTable,
        tx: TxId,
        gen: Generation,
    ) -> Result<(), QdnfError> {
        let idx = self.find_id(&id).ok_or(QdnfError::Incomplete)?;
        let expected = tx_from_operation_id(&id);
        if tx.bytes != expected.bytes {
            return Err(QdnfError::Malformed);
        }
        if self.slots[idx].committed {
            return Ok(());
        }
        intents.commit(tx, gen)?;
        self.slots[idx].committed = true;
        self.slots[idx].generation = gen;
        Ok(())
    }

    pub fn get_committed(&self, id: &StrongDigest) -> Result<OperationDesc, QdnfError> {
        match self.find_id(id) {
            Some(idx) if self.slots[idx].committed => Ok(self.slots[idx].desc),
            _ => Err(QdnfError::Incomplete),
        }
    }

    fn unauthorized_epoch_replacement(&self, desc: &OperationDesc) -> bool {
        let mut i = 0usize;
        while i < MAX_OPS {
            let slot = &self.slots[i];
            if slot.occupied
                && slot.desc.author == desc.author
                && slot.desc.scope == desc.scope
                && desc.epoch > slot.desc.epoch
                && desc.sequence < slot.desc.sequence
            {
                return true;
            }
            i += 1;
        }
        false
    }

    fn find_id(&self, id: &StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_OPS {
            if self.slots[i].occupied && self.slots[i].id == *id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_OPS {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

fn parent_count(desc: &OperationDesc) -> usize {
    core::cmp::min(desc.parent_count as usize, MAX_PARENTS)
}

fn transcript_operation_id(desc: &OperationDesc) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    let pc = parent_count(desc);
    t.append(b"v", OP_ID_DOMAIN)?;
    t.append(b"author", &desc.author.0)?;
    t.append(b"epoch", &desc.epoch.to_be_bytes())?;
    t.append(b"sequence", &desc.sequence.to_be_bytes())?;
    t.append(b"scope", &desc.scope.to_be_bytes())?;
    t.append(b"contract", &desc.contract.0)?;
    t.append(b"purpose", &desc.purpose.0)?;
    t.append(b"payload", &desc.payload_digest.0)?;
    t.append(b"parent_count", &[pc as u8])?;
    let mut i = 0usize;
    while i < pc {
        t.append(b"parent", &desc.parents[i].0)?;
        i += 1;
    }
    Ok(t.digest())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn distinct(seq: u32) -> OperationDesc {
        let mut desc = sample(seq);
        desc.payload_digest = StrongDigest([seq as u8; 48]);
        desc
    }

    #[test]
    fn identical_submit_after_commit_replays_same_id() {
        let mut table = OpTable::new();
        let mut intents = IntentTable::new();
        let gen = Generation(1);
        let desc = sample(1);
        let id = table.submit(desc, &mut intents, 9, gen).unwrap();
        assert_eq!(id.0.len(), 48);
        assert_ne!(id, StrongDigest::ZERO);
        table
            .commit(id, &mut intents, tx_from_operation_id(&id), gen)
            .unwrap();
        let again = table.submit(desc, &mut intents, 9, gen).unwrap();
        assert_eq!(again, id);
        assert_eq!(table.get_committed(&id).unwrap(), desc);
    }

    #[test]
    fn sequence_max_at_submit_is_range() {
        let mut table = OpTable::new();
        let mut intents = IntentTable::new();
        let mut desc = sample(0);
        desc.sequence = u32::MAX;
        assert_eq!(
            table.submit(desc, &mut intents, 1, Generation(1)),
            Err(QdnfError::Range)
        );
    }

    #[test]
    fn transport_ack_is_not_durable() {
        assert!(!transport_ack_is_durable());
    }

    #[test]
    fn source_signature_not_reusable_after_redaction() {
        assert!(!source_signature_reusable_after_redaction());
    }

    #[test]
    fn get_committed_before_commit_is_incomplete() {
        let mut table = OpTable::new();
        let mut intents = IntentTable::new();
        let gen = Generation(1);
        let id = table.submit(sample(2), &mut intents, 3, gen).unwrap();
        assert_eq!(table.get_committed(&id), Err(QdnfError::Incomplete));
    }

    #[test]
    fn higher_epoch_lower_sequence_is_unauthorized() {
        let mut table = OpTable::new();
        let mut intents = IntentTable::new();
        let gen = Generation(1);
        let mut first = sample(10);
        first.epoch = 1;
        table.submit(first, &mut intents, 4, gen).unwrap();
        let mut replacement = first;
        replacement.epoch = 2;
        replacement.sequence = 5;
        replacement.payload_digest = StrongDigest([0x55; 48]);
        assert_eq!(
            table.submit(replacement, &mut intents, 4, gen),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn seventeenth_distinct_op_is_capacity() {
        let mut table = OpTable::new();
        let mut intents = IntentTable::new();
        let gen = Generation(1);
        let mut seq = 0u32;
        while seq < MAX_OPS as u32 {
            table
                .submit(distinct(seq), &mut intents, 8, gen)
                .unwrap();
            seq += 1;
        }
        assert_eq!(
            table.submit(distinct(MAX_OPS as u32), &mut intents, 8, gen),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn operation_id_is_sha384_width_and_field_sensitive() {
        let a = sample(3);
        let mut b = a;
        b.sequence = 4;
        let id_a = operation_id(&a);
        let id_b = operation_id(&b);
        assert_eq!(id_a.0.len(), 48);
        assert_ne!(id_a, id_b);
        assert_eq!(id_a, transcript_operation_id(&a).unwrap());
    }
}

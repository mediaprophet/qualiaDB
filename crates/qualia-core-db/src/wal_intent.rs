//! Bounded in-memory intent/commit ledger (CORE-03 partial).
//!
//! Separate typed marker stream from [`crate::wal`] (packed NQuins + SHA-256
//! header). Does not claim remote atomicity or remote custody.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

pub const MAX_INTENTS: usize = 32;
pub const MAX_OBJECT_BYTES: usize = 512;
pub const MARKER_LEN: usize = 80;

const MAGIC: [u8; 4] = *b"QINT";

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Contract = 1,
    Proof = 2,
    Operation = 3,
    Receipt = 4,
    Evidence = 5,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Intent = 1,
    Commit = 2,
    Abort = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TxId {
    pub bytes: [u8; 16],
}

impl TxId {
    pub const ZERO: Self = Self { bytes: [0u8; 16] };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactObjectRef {
    pub kind: ArtifactKind,
    pub digest: StrongDigest,
    pub byte_len: u16,
}

impl ExactObjectRef {
    pub const EMPTY: Self = Self {
        kind: ArtifactKind::Contract,
        digest: StrongDigest::ZERO,
        byte_len: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntentRecord {
    pub tx: TxId,
    pub kind: MarkerKind,
    pub object: ExactObjectRef,
    pub writer_id: u64,
    pub generation: Generation,
}

impl IntentRecord {
    pub const EMPTY: Self = Self {
        tx: TxId::ZERO,
        kind: MarkerKind::Intent,
        object: ExactObjectRef::EMPTY,
        writer_id: 0,
        generation: Generation::ZERO,
    };
}

/// Preserve original signed bytes in a caller buffer. Digest is SHA-384 of
/// those exact bytes.
pub fn bind_exact_object(
    kind: ArtifactKind,
    bytes: &[u8],
    out_copy: &mut [u8],
) -> Result<ExactObjectRef, QdnfError> {
    if bytes.len() > MAX_OBJECT_BYTES || bytes.len() > out_copy.len() {
        return Err(QdnfError::Capacity);
    }
    out_copy[..bytes.len()].copy_from_slice(bytes);
    Ok(ExactObjectRef {
        kind,
        digest: sha384(bytes),
        byte_len: bytes.len() as u16,
    })
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    record: IntentRecord,
}

#[derive(Clone, Copy)]
struct WriterAuth {
    occupied: bool,
    writer_id: u64,
    live_generation: Generation,
}

/// Fixed 32-slot intent table with per-writer live generations.
pub struct IntentTable {
    slots: [Slot; MAX_INTENTS],
    writers: [WriterAuth; MAX_INTENTS],
}

impl IntentTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                record: IntentRecord::EMPTY,
            }; MAX_INTENTS],
            writers: [WriterAuth {
                occupied: false,
                writer_id: 0,
                live_generation: Generation::ZERO,
            }; MAX_INTENTS],
        }
    }

    /// Stage durable submission intent BEFORE any external dispatch.
    pub fn stage_intent(
        &mut self,
        tx: TxId,
        object: ExactObjectRef,
        writer_id: u64,
        authority_generation: Generation,
    ) -> Result<Generation, QdnfError> {
        if self.find_occupied(tx).is_some() {
            return Err(QdnfError::Conflict);
        }
        if let Some(idx) = self.find_writer(writer_id) {
            if authority_generation.0 < self.writers[idx].live_generation.0 {
                return Err(QdnfError::StaleGeneration);
            }
        }
        let free = self.find_free().ok_or(QdnfError::Capacity)?;
        self.upsert_writer(writer_id, authority_generation)?;
        self.slots[free] = Slot {
            occupied: true,
            record: IntentRecord {
                tx,
                kind: MarkerKind::Intent,
                object,
                writer_id,
                generation: authority_generation,
            },
        };
        Ok(authority_generation)
    }

    pub fn commit(&mut self, tx: TxId, authority_generation: Generation) -> Result<(), QdnfError> {
        self.transition(tx, authority_generation, MarkerKind::Commit)
    }

    pub fn abort(&mut self, tx: TxId, authority_generation: Generation) -> Result<(), QdnfError> {
        self.transition(tx, authority_generation, MarkerKind::Abort)
    }

    /// Pending intents for that writer cannot commit at the old generation.
    pub fn withdraw_authority(
        &mut self,
        writer_id: u64,
        new_generation: Generation,
    ) -> Result<(), QdnfError> {
        if let Some(idx) = self.find_writer(writer_id) {
            if new_generation.0 <= self.writers[idx].live_generation.0 {
                return Err(QdnfError::StaleGeneration);
            }
            self.writers[idx].live_generation = new_generation;
            return Ok(());
        }
        self.upsert_writer(writer_id, new_generation)
    }

    /// Encode a fixed marker into a caller buffer. Never claims remote atomicity.
    /// Artifact kind / byte_len are encoded as Contract / 0 (not in this signature).
    pub fn encode_marker(
        kind: MarkerKind,
        tx: TxId,
        digest: &StrongDigest,
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        if out.len() < MARKER_LEN {
            return Err(QdnfError::Capacity);
        }
        out[..4].copy_from_slice(&MAGIC);
        out[4] = kind as u8;
        out[5] = ArtifactKind::Contract as u8;
        out[6..8].copy_from_slice(&0u16.to_be_bytes());
        out[8..24].copy_from_slice(&tx.bytes);
        out[24..72].copy_from_slice(&digest.0);
        out[72..MARKER_LEN].fill(0);
        Ok(MARKER_LEN)
    }

    /// Recover concatenated markers. Torn trailing bytes are not applied.
    pub fn recover_into(
        markers: &[u8],
        out: &mut [IntentRecord; MAX_INTENTS],
    ) -> Result<usize, QdnfError> {
        let mut produced = 0usize;
        let mut offset = 0usize;
        while offset + MARKER_LEN <= markers.len() {
            if produced >= out.len() {
                return Err(QdnfError::Capacity);
            }
            out[produced] = decode_marker(&markers[offset..offset + MARKER_LEN])?;
            produced += 1;
            offset += MARKER_LEN;
        }
        Ok(produced)
    }

    pub fn local_pin_implies_remote_custody() -> bool {
        false
    }

    pub fn local_commit_is_remote_atomic() -> bool {
        false
    }

    fn transition(
        &mut self,
        tx: TxId,
        authority_generation: Generation,
        target: MarkerKind,
    ) -> Result<(), QdnfError> {
        let idx = self.find_occupied(tx).ok_or(QdnfError::Incomplete)?;
        let rec = &self.slots[idx].record;
        match (rec.kind, target) {
            (MarkerKind::Commit, MarkerKind::Commit) | (MarkerKind::Abort, MarkerKind::Abort) => {
                return Err(QdnfError::Conflict);
            }
            (MarkerKind::Abort, MarkerKind::Commit) | (MarkerKind::Commit, MarkerKind::Abort) => {
                return Err(QdnfError::Ambiguous);
            }
            (MarkerKind::Intent, _) => {}
            _ => return Err(QdnfError::Ambiguous),
        }
        if rec.generation != authority_generation {
            return Err(QdnfError::StaleGeneration);
        }
        if let Some(w) = self.find_writer(rec.writer_id) {
            if self.writers[w].live_generation != authority_generation {
                return Err(QdnfError::StaleGeneration);
            }
        }
        self.slots[idx].record.kind = target;
        Ok(())
    }

    fn find_occupied(&self, tx: TxId) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_INTENTS {
            if self.slots[i].occupied && self.slots[i].record.tx.bytes == tx.bytes {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_INTENTS {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_writer(&self, writer_id: u64) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_INTENTS {
            if self.writers[i].occupied && self.writers[i].writer_id == writer_id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn upsert_writer(&mut self, writer_id: u64, generation: Generation) -> Result<(), QdnfError> {
        if let Some(idx) = self.find_writer(writer_id) {
            if generation.0 > self.writers[idx].live_generation.0 {
                self.writers[idx].live_generation = generation;
            }
            return Ok(());
        }
        let mut i = 0usize;
        while i < MAX_INTENTS {
            if !self.writers[i].occupied {
                self.writers[i] = WriterAuth {
                    occupied: true,
                    writer_id,
                    live_generation: generation,
                };
                return Ok(());
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }
}

fn decode_marker(raw: &[u8]) -> Result<IntentRecord, QdnfError> {
    if raw.len() != MARKER_LEN {
        return Err(QdnfError::Truncated);
    }
    if raw[..4] != MAGIC || raw[72..MARKER_LEN] != [0u8; 8] {
        return Err(QdnfError::Malformed);
    }
    let kind = match raw[4] {
        1 => MarkerKind::Intent,
        2 => MarkerKind::Commit,
        3 => MarkerKind::Abort,
        _ => return Err(QdnfError::Malformed),
    };
    let artifact = match raw[5] {
        1 => ArtifactKind::Contract,
        2 => ArtifactKind::Proof,
        3 => ArtifactKind::Operation,
        4 => ArtifactKind::Receipt,
        5 => ArtifactKind::Evidence,
        _ => return Err(QdnfError::Malformed),
    };
    let byte_len = u16::from_be_bytes([raw[6], raw[7]]);
    let mut tx_bytes = [0u8; 16];
    tx_bytes.copy_from_slice(&raw[8..24]);
    let mut digest_bytes = [0u8; 48];
    digest_bytes.copy_from_slice(&raw[24..72]);
    Ok(IntentRecord {
        tx: TxId { bytes: tx_bytes },
        kind,
        object: ExactObjectRef {
            kind: artifact,
            digest: StrongDigest(digest_bytes),
            byte_len,
        },
        writer_id: 0,
        generation: Generation::ZERO,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(n: u8) -> TxId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        TxId { bytes }
    }

    fn bind(kind: ArtifactKind, bytes: &[u8]) -> ExactObjectRef {
        let mut copy = [0u8; MAX_OBJECT_BYTES];
        bind_exact_object(kind, bytes, &mut copy).expect("bind")
    }

    #[test]
    fn bind_exact_object_copies_and_digest_is_forty_eight() {
        let payload = b"signed-contract-bytes";
        let mut copy = [0u8; MAX_OBJECT_BYTES];
        let object = bind_exact_object(ArtifactKind::Proof, payload, &mut copy).unwrap();
        assert_eq!(&copy[..payload.len()], payload);
        assert_eq!(object.digest.0.len(), 48);
        assert_eq!(object.digest, sha384(payload));
        copy[0] ^= 0xFF;
        assert_ne!(object.digest, sha384(&copy[..payload.len()]));
        assert_eq!(object.digest, sha384(payload));
    }

    #[test]
    fn stage_then_commit_second_commit_is_conflict() {
        let mut table = IntentTable::new();
        let gen = Generation(1);
        table
            .stage_intent(tx(1), bind(ArtifactKind::Contract, b"op-1"), 7, gen)
            .unwrap();
        table.commit(tx(1), gen).unwrap();
        assert_eq!(table.commit(tx(1), gen), Err(QdnfError::Conflict));
    }

    #[test]
    fn abort_then_commit_is_ambiguous() {
        let mut table = IntentTable::new();
        let gen = Generation(1);
        table
            .stage_intent(tx(2), bind(ArtifactKind::Receipt, b"op-2"), 7, gen)
            .unwrap();
        table.abort(tx(2), gen).unwrap();
        assert_eq!(table.commit(tx(2), gen), Err(QdnfError::Ambiguous));
    }

    #[test]
    fn withdraw_then_commit_at_old_generation_is_stale() {
        let mut table = IntentTable::new();
        let old = Generation(1);
        table
            .stage_intent(tx(3), bind(ArtifactKind::Operation, b"op-3"), 9, old)
            .unwrap();
        table.withdraw_authority(9, Generation(2)).unwrap();
        assert_eq!(table.commit(tx(3), old), Err(QdnfError::StaleGeneration));
    }

    #[test]
    fn thirty_third_stage_is_capacity() {
        let mut table = IntentTable::new();
        let object = bind(ArtifactKind::Evidence, b"slot");
        let gen = Generation(1);
        let mut i = 0u8;
        while i < MAX_INTENTS as u8 {
            table.stage_intent(tx(i + 1), object, 1, gen).unwrap();
            i += 1;
        }
        assert_eq!(
            table.stage_intent(tx(0xFF), object, 1, gen),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn encode_marker_recover_into_round_trip_ignores_torn_tail() {
        let object = bind(ArtifactKind::Contract, b"marker-a");
        let mut stream = [0u8; MARKER_LEN * 2 + 10];
        assert_eq!(
            IntentTable::encode_marker(MarkerKind::Intent, tx(10), &object.digest, &mut stream[..MARKER_LEN])
                .unwrap(),
            MARKER_LEN
        );
        assert_eq!(
            IntentTable::encode_marker(
                MarkerKind::Commit,
                tx(11),
                &object.digest,
                &mut stream[MARKER_LEN..MARKER_LEN * 2],
            )
            .unwrap(),
            MARKER_LEN
        );
        stream[MARKER_LEN * 2..].fill(0xAA);
        let mut out = [IntentRecord::EMPTY; MAX_INTENTS];
        let intact = IntentTable::recover_into(&stream[..MARKER_LEN * 2], &mut out).unwrap();
        assert_eq!(intact, 2);
        assert_eq!(IntentTable::recover_into(&stream, &mut out).unwrap(), intact);
        assert_eq!(out[0].tx, tx(10));
        assert_eq!(out[0].kind, MarkerKind::Intent);
        assert_eq!(out[0].object.digest, object.digest);
        assert_eq!(out[1].tx, tx(11));
        assert_eq!(out[1].kind, MarkerKind::Commit);
        assert_eq!(out[1].object.digest, object.digest);
    }

    #[test]
    fn local_pin_and_local_commit_are_not_remote() {
        assert!(!IntentTable::local_pin_implies_remote_custody());
        assert!(!IntentTable::local_commit_is_remote_atomic());
    }

    #[test]
    fn identical_bytes_do_not_unify_distinct_authorities() {
        let payload = b"same-bytes";
        let a = bind(ArtifactKind::Contract, payload);
        let b = bind(ArtifactKind::Contract, payload);
        assert_eq!(a.digest, b.digest);
        let mut table = IntentTable::new();
        let gen = Generation(3);
        table.stage_intent(tx(20), a, 100, gen).unwrap();
        table.stage_intent(tx(21), b, 200, gen).unwrap();
        assert_eq!(
            table.stage_intent(tx(20), a, 100, gen),
            Err(QdnfError::Conflict)
        );
        table.commit(tx(20), gen).unwrap();
        table.commit(tx(21), gen).unwrap();
    }
}

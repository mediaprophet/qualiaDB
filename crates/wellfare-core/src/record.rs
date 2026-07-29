use serde::{Deserialize, Serialize};

/// FNV-1a hash matching the QualiaDB canonical q_hash
pub const fn q_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

pub const fn q_hash_str(s: &str) -> u64 {
    q_hash(s.as_bytes())
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct NQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

impl NQuin {
    pub fn new(s: u64, p: u64, o: u64, c: u64, m: u64) -> Self {
        let parity = s ^ p ^ o ^ c;
        Self {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpistemicStatus {
    Asserted,
    Hypothesis,
    Disputed,
    Refuted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    SelfReported,
    DeviceMeasured,
    ClinicianObserved,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitivityClass {
    Public,
    Restricted,
    Classified,
}

impl SensitivityClass {
    pub fn to_metadata_mask(&self) -> u64 {
        match self {
            SensitivityClass::Public => 0,
            SensitivityClass::Restricted => 1 << 56,
            SensitivityClass::Classified => 2 << 56,
        }
    }
}

/// A deterministic Record Envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordEnvelope {
    pub id: String,
    pub owner_did: String,
    pub author_did: String,
    pub proxy_did: Option<String>,
    pub epistemic_status: EpistemicStatus,
    pub evidence_type: EvidenceType,
    pub sensitivity: SensitivityClass,
    pub asserted_time_unix: u32,
    pub valid_time_start_unix: Option<u32>,
    pub valid_time_end_unix: Option<u32>,
    pub predecessor_id: Option<String>,
    pub blob_hash: Option<String>, // Document/Media reference
    pub tombstone: bool,
}

impl RecordEnvelope {
    /// Zero-allocation compiler: writes the envelope Quins into the provided slice.
    /// Returns the number of Quins written.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        let mut count = 0;
        let id_hash = q_hash_str(&self.id);
        let ctx = q_hash_str(&self.owner_did);
        let meta = self.sensitivity.to_metadata_mask();

        if count < out.len() {
            out[count] = NQuin::new(
                id_hash,
                q_hash_str("q42:hasAuthor"),
                q_hash_str(&self.author_did),
                ctx,
                meta,
            );
            count += 1;
        }

        if let Some(ref proxy) = self.proxy_did {
            if count < out.len() {
                out[count] = NQuin::new(
                    id_hash,
                    q_hash_str("q42:hasProxy"),
                    q_hash_str(proxy),
                    ctx,
                    meta,
                );
                count += 1;
            }
        }

        if let Some(ref pred) = self.predecessor_id {
            if count < out.len() {
                out[count] = NQuin::new(
                    id_hash,
                    q_hash_str("q42:precedes"),
                    q_hash_str(pred),
                    ctx,
                    meta,
                );
                count += 1;
            }
        }

        if let Some(ref blob) = self.blob_hash {
            if count < out.len() {
                out[count] = NQuin::new(
                    id_hash,
                    q_hash_str("q42:hasBlob"),
                    q_hash_str(blob),
                    ctx,
                    meta,
                );
                count += 1;
            }
        }

        if self.tombstone && count < out.len() {
            out[count] = NQuin::new(id_hash, q_hash_str("q42:isTombstone"), 1, ctx, meta);
            count += 1;
        }

        count
    }
}

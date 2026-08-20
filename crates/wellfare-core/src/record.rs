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

    /// Convert the coarse `asserted_time_unix: u32` to an `InstantBridge`
    /// with nanosecond resolution (T71 bridge).
    ///
    /// This is the bridge for the one-clock migration. New code should
    /// use this to obtain a high-resolution instant for composition with
    /// VibeScript time operations. The full migration (replacing the u32
    /// field) is deferred until all consumers are updated.
    pub fn asserted_instant(&self) -> InstantBridge {
        InstantBridge::unix(self.asserted_time_unix as i64, 0)
    }

    /// Convert valid_time_start to an InstantBridge (T71 bridge).
    pub fn valid_time_start_instant(&self) -> Option<InstantBridge> {
        self.valid_time_start_unix
            .map(|t| InstantBridge::unix(t as i64, 0))
    }

    /// Convert valid_time_end to an InstantBridge (T71 bridge).
    pub fn valid_time_end_instant(&self) -> Option<InstantBridge> {
        self.valid_time_end_unix
            .map(|t| InstantBridge::unix(t as i64, 0))
    }

    /// Create a RecordEnvelope with an InstantBridge for asserted time (T71 bridge).
    /// The InstantBridge is projected to Unix seconds for the u32 field.
    pub fn with_asserted_instant(
        id: &str,
        owner_did: &str,
        author_did: &str,
        epistemic_status: EpistemicStatus,
        evidence_type: EvidenceType,
        sensitivity: SensitivityClass,
        asserted: &InstantBridge,
        blob_hash: Option<String>,
    ) -> Self {
        let asserted_unix = asserted.secs as u32;
        Self {
            id: id.into(),
            owner_did: owner_did.into(),
            author_did: author_did.into(),
            proxy_did: None,
            epistemic_status,
            evidence_type,
            sensitivity,
            asserted_time_unix: asserted_unix,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash,
            tombstone: false,
        }
    }
}

/// Lightweight nanosecond-resolution instant bridge (T71).
///
/// This is a bridge type for the one-clock migration. It stores
/// Unix seconds + nanoseconds without pulling in `poet-vibe` as a
/// dependency. When the `qualia` feature is enabled, it can convert
/// to/from `poet_vibe::value::Instant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstantBridge {
    /// Unix seconds
    pub secs: i64,
    /// Nanoseconds within the second (0–999,999,999)
    pub nanos: u32,
}

impl InstantBridge {
    /// Create a Unix instant from seconds + nanoseconds.
    pub fn unix(secs: i64, nanos: u32) -> Self {
        Self { secs, nanos }
    }

    /// Project to Unix seconds (drops sub-second precision).
    pub fn to_unix_secs(&self) -> i64 {
        self.secs
    }

    /// Project to Unix nanoseconds.
    pub fn to_unix_nanos(&self) -> i64 {
        self.secs * 1_000_000_000 + self.nanos as i64
    }

    /// Convert from a coarse u32 Unix timestamp.
    pub fn from_coarse(unix_secs: u32) -> Self {
        Self::unix(unix_secs as i64, 0)
    }
}

#[cfg(feature = "qualia")]
impl From<InstantBridge> for poet_vibe::Instant {
    fn from(b: InstantBridge) -> Self {
        poet_vibe::Instant::unix(b.secs, b.nanos)
    }
}

#[cfg(feature = "qualia")]
impl From<&poet_vibe::Instant> for InstantBridge {
    fn from(i: &poet_vibe::Instant) -> Self {
        Self {
            secs: i.secs,
            nanos: i.nanos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t71_instant_bridge_basic() {
        let i = InstantBridge::unix(1_700_000_000, 500_000_000);
        assert_eq!(i.secs, 1_700_000_000);
        assert_eq!(i.nanos, 500_000_000);
    }

    #[test]
    fn t71_instant_bridge_from_coarse() {
        let i = InstantBridge::from_coarse(1_700_000_000);
        assert_eq!(i.secs, 1_700_000_000);
        assert_eq!(i.nanos, 0);
    }

    #[test]
    fn t71_instant_bridge_to_unix_nanos() {
        let i = InstantBridge::unix(1, 500_000_000);
        assert_eq!(i.to_unix_nanos(), 1_500_000_000);
    }

    #[test]
    fn t71_envelope_asserted_instant() {
        let env = RecordEnvelope {
            id: "test".into(),
            owner_did: "did:alice".into(),
            author_did: "did:alice".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Public,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        let instant = env.asserted_instant();
        assert_eq!(instant.secs, 1_700_000_000);
        assert_eq!(instant.nanos, 0);
    }

    #[test]
    fn t71_envelope_with_asserted_instant() {
        let instant = InstantBridge::unix(1_700_000_000, 123_456);
        let env = RecordEnvelope::with_asserted_instant(
            "test",
            "did:alice",
            "did:alice",
            EpistemicStatus::Asserted,
            EvidenceType::SelfReported,
            SensitivityClass::Public,
            &instant,
            None,
        );
        assert_eq!(env.asserted_time_unix, 1_700_000_000);
        let recovered = env.asserted_instant();
        assert_eq!(recovered.secs, 1_700_000_000);
    }

    #[test]
    fn t71_valid_time_instant_bridges() {
        let env = RecordEnvelope {
            id: "test".into(),
            owner_did: "did:alice".into(),
            author_did: "did:alice".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Public,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: Some(1_699_000_000),
            valid_time_end_unix: Some(1_701_000_000),
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        let start = env.valid_time_start_instant().unwrap();
        assert_eq!(start.secs, 1_699_000_000);
        let end = env.valid_time_end_instant().unwrap();
        assert_eq!(end.secs, 1_701_000_000);
    }
}

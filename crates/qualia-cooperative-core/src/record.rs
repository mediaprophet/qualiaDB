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
    /// High-resolution instant (T71 bridge). Preferred over `asserted_time_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub asserted_instant: Option<InstantBridge>,
    pub valid_time_start_unix: Option<u32>,
    /// High-resolution valid-time start (T71 bridge). Preferred over
    /// `valid_time_start_unix` when present.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub valid_time_start_instant: Option<InstantBridge>,
    pub valid_time_end_unix: Option<u32>,
    /// High-resolution valid-time end (T71 bridge). Preferred over
    /// `valid_time_end_unix` when present.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub valid_time_end_instant: Option<InstantBridge>,
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

    /// Resolve the asserted instant, preferring the high-resolution
    /// `InstantBridge` field when present and falling back to the coarse
    /// `u32` Unix seconds otherwise (T71 bridge).
    pub fn asserted_instant(&self) -> InstantBridge {
        self.asserted_instant
            .unwrap_or_else(|| InstantBridge::unix(self.asserted_time_unix as i64, 0))
    }

    /// Resolve the valid-time start instant, preferring the high-resolution
    /// `InstantBridge` field when present and falling back to the coarse
    /// `u32` Unix seconds otherwise (T71 bridge).
    pub fn valid_time_start_instant(&self) -> Option<InstantBridge> {
        self.valid_time_start_instant.or_else(|| {
            self.valid_time_start_unix
                .map(|t| InstantBridge::unix(t as i64, 0))
        })
    }

    /// Resolve the valid-time end instant, preferring the high-resolution
    /// `InstantBridge` field when present and falling back to the coarse
    /// `u32` Unix seconds otherwise (T71 bridge).
    pub fn valid_time_end_instant(&self) -> Option<InstantBridge> {
        self.valid_time_end_instant.or_else(|| {
            self.valid_time_end_unix
                .map(|t| InstantBridge::unix(t as i64, 0))
        })
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
            asserted_instant: Some(*asserted),
            valid_time_start_unix: None,
            valid_time_start_instant: None,
            valid_time_end_unix: None,
            valid_time_end_instant: None,
            predecessor_id: None,
            blob_hash,
            tombstone: false,
        }
    }

    /// Create a RecordEnvelope with InstantBridges for asserted and valid time
    /// (T71 bridge). The InstantBridges are projected to Unix seconds for the
    /// u32 fields.
    pub fn with_instants(
        id: &str,
        owner_did: &str,
        author_did: &str,
        epistemic_status: EpistemicStatus,
        evidence_type: EvidenceType,
        sensitivity: SensitivityClass,
        asserted: &InstantBridge,
        valid_start: Option<&InstantBridge>,
        valid_end: Option<&InstantBridge>,
        blob_hash: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            owner_did: owner_did.into(),
            author_did: author_did.into(),
            proxy_did: None,
            epistemic_status,
            evidence_type,
            sensitivity,
            asserted_time_unix: asserted.secs as u32,
            asserted_instant: Some(*asserted),
            valid_time_start_unix: valid_start.map(|i| i.secs as u32),
            valid_time_start_instant: valid_start.copied(),
            valid_time_end_unix: valid_end.map(|i| i.secs as u32),
            valid_time_end_instant: valid_end.copied(),
            predecessor_id: None,
            blob_hash,
            tombstone: false,
        }
    }

    /// The duration of the valid time window (T71 bridge).
    /// Returns `None` if either bound is missing.
    pub fn valid_time_duration(&self) -> Option<DurationBridge> {
        let start = self.valid_time_start_instant()?;
        let end = self.valid_time_end_instant()?;
        Some(end.duration_since(&start))
    }
}

/// Lightweight nanosecond-resolution instant bridge (T71).
///
/// This is a bridge type for the one-clock migration. It stores
/// Unix seconds + nanoseconds without pulling in `vibe` as a
/// dependency. When the `qualia` feature is enabled, it can convert
/// to/from `vibe::value::Instant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

    /// True if this instant is the zero value (epoch + 0 nanos).
    /// Used for serde `skip_serializing_if` on optional instant fields.
    pub fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
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

    /// The duration between two instants: `self - earlier`.
    /// Returns a positive `DurationBridge` if `self > earlier`.
    pub fn duration_since(&self, earlier: &InstantBridge) -> DurationBridge {
        let self_nanos = self.to_unix_nanos();
        let earlier_nanos = earlier.to_unix_nanos();
        DurationBridge::from_nanos(self_nanos - earlier_nanos)
    }

    /// Add a duration to this instant.
    pub fn add_duration(&self, d: &DurationBridge) -> InstantBridge {
        let total = self.to_unix_nanos() + d.total_nanos();
        InstantBridge::from_unix_nanos(total)
    }

    /// Subtract a duration from this instant.
    pub fn sub_duration(&self, d: &DurationBridge) -> InstantBridge {
        let total = self.to_unix_nanos() - d.total_nanos();
        InstantBridge::from_unix_nanos(total)
    }

    /// Create an InstantBridge from total Unix nanoseconds, handling the
    /// negative-remainder case so `nanos` is always in [0, 999_999_999).
    fn from_unix_nanos(total_nanos: i64) -> InstantBridge {
        let secs = total_nanos / 1_000_000_000;
        let rem = total_nanos % 1_000_000_000;
        if rem < 0 {
            InstantBridge::unix(secs - 1, (rem + 1_000_000_000) as u32)
        } else {
            InstantBridge::unix(secs, rem as u32)
        }
    }

    /// Whether this instant is before another.
    pub fn is_before(&self, other: &InstantBridge) -> bool {
        self.to_unix_nanos() < other.to_unix_nanos()
    }

    /// Whether this instant is after another.
    pub fn is_after(&self, other: &InstantBridge) -> bool {
        self.to_unix_nanos() > other.to_unix_nanos()
    }

    /// The earlier of two instants.
    pub fn min(self, other: InstantBridge) -> InstantBridge {
        if self.is_before(&other) { self } else { other }
    }

    /// The later of two instants.
    pub fn max(self, other: InstantBridge) -> InstantBridge {
        if self.is_after(&other) { self } else { other }
    }
}

impl Ord for InstantBridge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_unix_nanos().cmp(&other.to_unix_nanos())
    }
}

impl PartialOrd for InstantBridge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Lightweight duration bridge (T71).
///
/// Stores seconds + nanoseconds. Can be negative (for "earlier - later").
/// Converts to/from `vibe::Duration` when the `qualia` feature is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DurationBridge {
    /// Seconds (may be negative)
    pub secs: i64,
    /// Nanoseconds within the second (0–999,999,999)
    pub nanos: u32,
}

impl DurationBridge {
    /// Create a duration from seconds + nanoseconds.
    pub fn new(secs: i64, nanos: u32) -> Self {
        Self { secs, nanos }
    }

    /// Create a duration from total nanoseconds (may be negative).
    /// The nanos field is always in [0, 999_999_999); the sign is carried
    /// by the secs field.
    pub fn from_nanos(total_nanos: i64) -> Self {
        let secs = total_nanos / 1_000_000_000;
        let rem = total_nanos % 1_000_000_000;
        if rem < 0 {
            // Borrow from secs so nanos is positive
            Self {
                secs: secs - 1,
                nanos: (rem + 1_000_000_000) as u32,
            }
        } else {
            Self {
                secs,
                nanos: rem as u32,
            }
        }
    }

    /// Create a duration from seconds.
    pub fn from_secs(secs: i64) -> Self {
        Self { secs, nanos: 0 }
    }

    /// Total nanoseconds (may be negative).
    pub fn total_nanos(&self) -> i64 {
        self.secs * 1_000_000_000 + self.nanos as i64
    }

    /// Total milliseconds (may be negative, truncates sub-ms).
    pub fn total_millis(&self) -> i64 {
        self.total_nanos() / 1_000_000
    }

    /// Whether this duration is positive (self > 0).
    pub fn is_positive(&self) -> bool {
        self.total_nanos() > 0
    }

    /// Whether this duration is zero.
    pub fn is_zero(&self) -> bool {
        self.total_nanos() == 0
    }

    /// Absolute value of this duration.
    pub fn abs(self) -> Self {
        let n = self.total_nanos();
        if n < 0 { Self::from_nanos(-n) } else { self }
    }
}

#[cfg(feature = "qualia")]
impl From<InstantBridge> for vibe::Instant {
    fn from(b: InstantBridge) -> Self {
        vibe::Instant::unix(b.secs, b.nanos)
    }
}

#[cfg(feature = "qualia")]
impl From<&vibe::Instant> for InstantBridge {
    fn from(i: &vibe::Instant) -> Self {
        Self {
            secs: i.secs,
            nanos: i.nanos,
        }
    }
}

#[cfg(feature = "qualia")]
impl From<DurationBridge> for vibe::Duration {
    fn from(d: DurationBridge) -> Self {
        vibe::Duration {
            secs: d.secs,
            nanos: d.nanos,
        }
    }
}

#[cfg(feature = "qualia")]
impl From<&vibe::Duration> for DurationBridge {
    fn from(d: &vibe::Duration) -> Self {
        Self {
            secs: d.secs,
            nanos: d.nanos,
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
            asserted_instant: None,
            valid_time_start_unix: None,
            valid_time_start_instant: None,
            valid_time_end_unix: None,
            valid_time_end_instant: None,
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
            asserted_instant: None,
            valid_time_start_unix: Some(1_699_000_000),
            valid_time_start_instant: None,
            valid_time_end_unix: Some(1_701_000_000),
            valid_time_end_instant: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        let start = env.valid_time_start_instant().unwrap();
        assert_eq!(start.secs, 1_699_000_000);
        let end = env.valid_time_end_instant().unwrap();
        assert_eq!(end.secs, 1_701_000_000);
    }

    #[test]
    fn t71_instant_comparison() {
        let a = InstantBridge::unix(1_700_000_000, 0);
        let b = InstantBridge::unix(1_700_000_001, 0);
        assert!(a.is_before(&b));
        assert!(b.is_after(&a));
        assert!(!a.is_after(&b));
        assert!(!b.is_before(&a));
        assert_eq!(a.min(b), a);
        assert_eq!(a.max(b), b);
    }

    #[test]
    fn t71_instant_subsecond_comparison() {
        let a = InstantBridge::unix(1_700_000_000, 500_000_000);
        let b = InstantBridge::unix(1_700_000_000, 600_000_000);
        assert!(a.is_before(&b));
        assert!(b.is_after(&a));
    }

    #[test]
    fn t71_instant_ord() {
        let a = InstantBridge::unix(1, 0);
        let b = InstantBridge::unix(2, 0);
        let c = InstantBridge::unix(1, 500_000_000);
        let mut v = vec![b, c, a];
        v.sort();
        assert_eq!(v, vec![a, c, b]);
    }

    #[test]
    fn t71_duration_since() {
        let a = InstantBridge::unix(1_700_000_000, 0);
        let b = InstantBridge::unix(1_700_000_002, 500_000_000);
        let d = b.duration_since(&a);
        assert_eq!(d.secs, 2);
        assert_eq!(d.nanos, 500_000_000);
        assert_eq!(d.total_nanos(), 2_500_000_000);
    }

    #[test]
    fn t71_duration_since_negative() {
        let a = InstantBridge::unix(1_700_000_002, 0);
        let b = InstantBridge::unix(1_700_000_000, 0);
        let d = b.duration_since(&a);
        assert!(d.total_nanos() < 0);
        assert!(!d.is_positive());
    }

    #[test]
    fn t71_add_duration() {
        let a = InstantBridge::unix(1_700_000_000, 500_000_000);
        let d = DurationBridge::from_secs(2);
        let b = a.add_duration(&d);
        assert_eq!(b.secs, 1_700_000_002);
        assert_eq!(b.nanos, 500_000_000);
    }

    #[test]
    fn t71_sub_duration() {
        let a = InstantBridge::unix(1_700_000_002, 0);
        let d = DurationBridge::from_secs(2);
        let b = a.sub_duration(&d);
        assert_eq!(b.secs, 1_700_000_000);
    }

    #[test]
    fn t71_duration_from_nanos() {
        let d = DurationBridge::from_nanos(1_500_000_000);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn t71_duration_total_millis() {
        let d = DurationBridge::from_nanos(1_500_000_000);
        assert_eq!(d.total_millis(), 1500);
    }

    #[test]
    fn t71_duration_abs() {
        let d = DurationBridge::from_nanos(-1_500_000_000);
        assert!(d.total_nanos() < 0);
        let a = d.abs();
        assert_eq!(a.total_nanos(), 1_500_000_000);
    }

    #[test]
    fn t71_duration_is_zero() {
        assert!(DurationBridge::from_nanos(0).is_zero());
        assert!(!DurationBridge::from_nanos(1).is_zero());
    }

    #[test]
    fn t71_envelope_with_instants() {
        let asserted = InstantBridge::unix(1_700_000_000, 123_456);
        let start = InstantBridge::unix(1_699_000_000, 0);
        let end = InstantBridge::unix(1_701_000_000, 0);
        let env = RecordEnvelope::with_instants(
            "test",
            "did:alice",
            "did:alice",
            EpistemicStatus::Asserted,
            EvidenceType::SelfReported,
            SensitivityClass::Public,
            &asserted,
            Some(&start),
            Some(&end),
            None,
        );
        assert_eq!(env.asserted_time_unix, 1_700_000_000);
        assert_eq!(env.valid_time_start_unix, Some(1_699_000_000));
        assert_eq!(env.valid_time_end_unix, Some(1_701_000_000));
    }

    #[test]
    fn t71_envelope_valid_time_duration() {
        let env = RecordEnvelope {
            id: "test".into(),
            owner_did: "did:alice".into(),
            author_did: "did:alice".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Public,
            asserted_time_unix: 1_700_000_000,
            asserted_instant: None,
            valid_time_start_unix: Some(1_699_000_000),
            valid_time_start_instant: None,
            valid_time_end_unix: Some(1_701_000_000),
            valid_time_end_instant: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        let d = env.valid_time_duration().unwrap();
        assert_eq!(d.secs, 2_000_000);
        assert!(d.is_positive());
    }

    #[test]
    fn t71_envelope_valid_time_duration_none() {
        let env = RecordEnvelope {
            id: "test".into(),
            owner_did: "did:alice".into(),
            author_did: "did:alice".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Public,
            asserted_time_unix: 1_700_000_000,
            asserted_instant: None,
            valid_time_start_unix: None,
            valid_time_start_instant: None,
            valid_time_end_unix: None,
            valid_time_end_instant: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        assert!(env.valid_time_duration().is_none());
    }
}

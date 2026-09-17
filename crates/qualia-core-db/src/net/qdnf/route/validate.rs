//! E08.1 — admit topology records only after origin/scope/sequence/expiry checks.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::index::{insert_edge, AdjacencyIndex};

/// Handling profile P0..P3. Higher is stricter.
pub const PROFILE_P0: u8 = 0;
pub const PROFILE_P3: u8 = 3;

/// Compact node id 0 is reserved (not a topology origin or endpoint).
pub const MIN_COMPACT_NODE: u8 = 1;

/// Untrusted topology advertisement. Digest is checked only when bytes are supplied.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopologyRecord {
    pub origin: u8,
    pub origin_digest: StrongDigest,
    pub scope: u64,
    pub from: u8,
    pub to: u8,
    pub sequence: u32,
    pub expiry_unix: u64,
    pub withdrawn: bool,
    pub bidirectional: bool,
    pub cost: u16,
    pub latency_ms: u32,
    pub energy_uj: u32,
    pub energy_known: bool,
    pub realm_bit: u8,
    pub profile: u8,
    pub failure_domain: u8,
    pub digest: StrongDigest,
}

/// Edge that passed validation. Never a candidate while `withdrawn`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedEdge {
    pub origin: u8,
    pub from: u8,
    pub to: u8,
    pub sequence: u32,
    pub expiry_unix: u64,
    pub withdrawn: bool,
    pub bidirectional: bool,
    pub cost: u16,
    pub latency_ms: u32,
    pub energy_uj: u32,
    pub energy_known: bool,
    pub realm_bit: u8,
    pub profile: u8,
    pub failure_domain: u8,
    pub scope: u64,
}

impl ValidatedEdge {
    pub const EMPTY: Self = Self {
        origin: 0,
        from: 0,
        to: 0,
        sequence: 0,
        expiry_unix: 0,
        withdrawn: false,
        bidirectional: false,
        cost: 0,
        latency_ms: 0,
        energy_uj: 0,
        energy_known: false,
        realm_bit: 0,
        profile: 0,
        failure_domain: 0,
        scope: 0,
    };
}

fn compact_in_range(id: u8) -> bool {
    id >= MIN_COMPACT_NODE
}

/// Validate `rec` against `expected_scope`, clock, per-origin sequence, and optional bytes.
pub fn validate_record(
    rec: &TopologyRecord,
    expected_scope: u64,
    now_unix: u64,
    last_sequence: Option<u32>,
    original_bytes: Option<&[u8]>,
) -> Result<ValidatedEdge, QdnfError> {
    if rec.origin == 0 || rec.origin_digest.is_zero() {
        return Err(QdnfError::Range);
    }
    if !compact_in_range(rec.origin) {
        return Err(QdnfError::Range);
    }
    if rec.scope != expected_scope {
        return Err(QdnfError::Unauthorized);
    }
    if !compact_in_range(rec.from) || !compact_in_range(rec.to) {
        return Err(QdnfError::Range);
    }
    if rec.from == rec.to {
        return Err(QdnfError::Malformed);
    }
    if rec.realm_bit >= 64 {
        return Err(QdnfError::Range);
    }
    if rec.profile > PROFILE_P3 {
        return Err(QdnfError::UnknownProfile);
    }
    if now_unix >= rec.expiry_unix {
        return Err(QdnfError::Expired);
    }
    if let Some(prev) = last_sequence {
        if rec.sequence <= prev {
            return Err(QdnfError::StaleGeneration);
        }
    }
    if let Some(bytes) = original_bytes {
        if bytes.is_empty() || rec.digest.is_zero() {
            return Err(QdnfError::Malformed);
        }
        if sha384(bytes) != rec.digest {
            return Err(QdnfError::Conflict);
        }
    }
    Ok(ValidatedEdge {
        origin: rec.origin,
        from: rec.from,
        to: rec.to,
        sequence: rec.sequence,
        expiry_unix: rec.expiry_unix,
        withdrawn: rec.withdrawn,
        bidirectional: rec.bidirectional,
        cost: rec.cost,
        latency_ms: rec.latency_ms,
        energy_uj: rec.energy_uj,
        energy_known: rec.energy_known,
        realm_bit: rec.realm_bit,
        profile: rec.profile,
        failure_domain: rec.failure_domain,
        scope: rec.scope,
    })
}

/// Validate then insert. Never writes the index on failure.
pub fn insert(
    rec: &TopologyRecord,
    expected_scope: u64,
    now_unix: u64,
    original_bytes: Option<&[u8]>,
    index: &mut AdjacencyIndex,
) -> Result<(), QdnfError> {
    if index.scope() != expected_scope {
        return Err(QdnfError::Unauthorized);
    }
    let last = index.last_sequence(rec.origin);
    let edge = validate_record(rec, expected_scope, now_unix, last, original_bytes)?;
    insert_edge(index, edge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    pub(crate) fn rec(origin: u8, from: u8, to: u8, seq: u32, expiry: u64) -> TopologyRecord {
        TopologyRecord {
            origin,
            origin_digest: {
                let mut d = StrongDigest::ZERO;
                d.0[0] = origin;
                d.0[47] = 1;
                d
            },
            scope: 1,
            from,
            to,
            sequence: seq,
            expiry_unix: expiry,
            withdrawn: false,
            bidirectional: true,
            cost: 1,
            latency_ms: 1,
            energy_uj: 1,
            energy_known: true,
            realm_bit: 0,
            profile: PROFILE_P0,
            failure_domain: origin,
            digest: StrongDigest::ZERO,
        }
    }

    #[test]
    fn origin_zero_is_range() {
        let r = rec(0, 1, 2, 1, 10);
        assert_eq!(validate_record(&r, 1, 1, None, None), Err(QdnfError::Range));
    }

    #[test]
    fn scope_mismatch_is_unauthorized() {
        let r = rec(1, 1, 2, 1, 10);
        assert_eq!(
            validate_record(&r, 99, 1, None, None),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn endpoint_zero_is_range() {
        let r = rec(1, 0, 2, 1, 10);
        assert_eq!(validate_record(&r, 1, 1, None, None), Err(QdnfError::Range));
    }

    #[test]
    fn stale_sequence_is_rejected() {
        let r = rec(1, 1, 2, 3, 10);
        assert_eq!(
            validate_record(&r, 1, 1, Some(3), None),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(
            validate_record(&r, 1, 1, Some(4), None),
            Err(QdnfError::StaleGeneration)
        );
        assert!(validate_record(&r, 1, 1, Some(2), None).is_ok());
    }

    #[test]
    fn expired_record_is_rejected() {
        let r = rec(1, 1, 2, 1, 10);
        assert_eq!(
            validate_record(&r, 1, 10, None, None),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn digest_mismatch_is_conflict() {
        let mut r = rec(1, 1, 2, 1, 10);
        r.digest = sha384(b"lsa-v1");
        assert_eq!(
            validate_record(&r, 1, 1, None, Some(b"tampered")),
            Err(QdnfError::Conflict)
        );
        assert!(validate_record(&r, 1, 1, None, Some(b"lsa-v1")).is_ok());
    }

    #[test]
    fn insert_happens_only_after_validation() {
        let mut index = AdjacencyIndex::new(1);
        let bad = rec(1, 1, 2, 1, 10);
        assert_eq!(
            insert(&bad, 2, 1, None, &mut index),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(index.live_count(1), 0);
        insert(&rec(1, 1, 2, 1, 10), 1, 1, None, &mut index).unwrap();
        assert_eq!(index.live_count(1), 1);
    }

    #[test]
    fn withdrawal_is_not_kept_as_candidate() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 10), 1, 1, None, &mut index).unwrap();
        let mut w = rec(1, 1, 2, 2, 10);
        w.withdrawn = true;
        insert(&w, 1, 1, None, &mut index).unwrap();
        assert_eq!(index.live_count(1), 0);
        assert!(index.is_withdrawn(1, 2));
    }
}

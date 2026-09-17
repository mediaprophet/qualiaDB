//! Profile overhead budgets. Charge each component separately (Recipe F).
//!
//! Padding, relay copies, handshakes, cover traffic, retries, storage and
//! cryptographic work are never lumped into a single opaque total before
//! the checked add in [`OverheadBudget::total_bytes`].

use crate::crypto::network::types::{
    ED25519_SIG_LEN, ML_DSA_65_SIG_LEN, ML_KEM_768_CT_LEN, ML_KEM_768_PK_LEN, X25519_LEN,
};
use crate::net::peer::runtime::ResourceBudget;
use crate::net::qdnf::errors::QdnfError;

use super::catalog::{ControlPredicate, ControlSet, ProtectionProfile};

/// P1 pads application size up to the next 256-byte bucket.
pub const P1_PADDING_BUCKET: u64 = 256;
/// P2 default application record size (last-record padding uses this bucket).
pub const P2_RECORD_BYTES: u64 = 4096;
/// P3/P4 traffic-cell target. A 4 KiB record is not an MTU claim.
pub const P3_CELL_BYTES: u64 = 4096;
/// P3 initial optional cover budget: 4 KiB/s per direction.
pub const P3_COVER_BYTES_PER_SEC: u64 = 4096;
/// P3 approved-relay hop count when relays are selected (not isolated bearer).
pub const P3_RELAY_HOPS: u64 = 2;

/// Hybrid handshake byte count:
/// initiator (ML-KEM-768 pk + X25519 pk) + responder (ML-KEM-768 ct + X25519 pk)
/// + ML-DSA-65 signature + Ed25519 signature.
pub const fn handshake_bytes_hybrid() -> u64 {
    (ML_KEM_768_PK_LEN
        + X25519_LEN
        + ML_KEM_768_CT_LEN
        + X25519_LEN
        + ML_DSA_65_SIG_LEN
        + ED25519_SIG_LEN) as u64
}

/// Round `size` up to the next multiple of `bucket`.
///
/// Formula (`bucket > 0`, `size > 0`):
/// `((size + bucket - 1) / bucket) * bucket`
///
/// `size == 0` stays 0 (no payload to pad). P1 uses `bucket = 256`:
/// 1 → 256, 256 → 256, 257 → 512.
pub fn pad_to_next_bucket(size: u64, bucket: u64) -> Result<u64, QdnfError> {
    if bucket == 0 {
        return Err(QdnfError::Range);
    }
    if size == 0 {
        return Ok(0);
    }
    let rounded = size.checked_add(bucket - 1).ok_or(QdnfError::Range)?;
    rounded
        .checked_div(bucket)
        .ok_or(QdnfError::Range)?
        .checked_mul(bucket)
        .ok_or(QdnfError::Range)
}

pub fn record_bucket(profile: ProtectionProfile) -> u64 {
    match profile {
        ProtectionProfile::P0 => 0,
        ProtectionProfile::P1 => P1_PADDING_BUCKET,
        ProtectionProfile::P2 | ProtectionProfile::P3 | ProtectionProfile::P4 => P2_RECORD_BYTES,
    }
}

/// Per-component overhead. Fields are independent charges.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverheadBudget {
    pub padding_bytes: u64,
    pub relay_copies: u64,
    pub handshake_bytes: u64,
    pub cover_bytes: u64,
    pub retry_bytes: u64,
    pub storage_bytes: u64,
    pub crypto_work: u64,
}

impl OverheadBudget {
    pub const ZERO: Self = Self {
        padding_bytes: 0,
        relay_copies: 0,
        handshake_bytes: 0,
        cover_bytes: 0,
        retry_bytes: 0,
        storage_bytes: 0,
        crypto_work: 0,
    };

    pub fn total_bytes(&self) -> Result<u64, QdnfError> {
        self.padding_bytes
            .checked_add(self.relay_copies)
            .and_then(|n| n.checked_add(self.handshake_bytes))
            .and_then(|n| n.checked_add(self.cover_bytes))
            .and_then(|n| n.checked_add(self.retry_bytes))
            .and_then(|n| n.checked_add(self.storage_bytes))
            .ok_or(QdnfError::Range)
    }

    pub fn to_resource_budget(&self) -> Result<ResourceBudget, QdnfError> {
        Ok(ResourceBudget {
            bytes: self.total_bytes()?,
            work: self.crypto_work,
            io: if self.relay_copies > 0 {
                P3_RELAY_HOPS
            } else {
                1
            },
        })
    }
}

fn cover_bytes_for(duration_ms: u64, mandatory: bool) -> Result<u64, QdnfError> {
    if !mandatory {
        return Ok(0);
    }
    P3_COVER_BYTES_PER_SEC
        .checked_mul(duration_ms)
        .ok_or(QdnfError::Range)?
        .checked_div(1000)
        .ok_or(QdnfError::Range)
}

fn relay_hops(controls: &ControlSet) -> u64 {
    if controls.contains_predicate(ControlPredicate::IsolatedBearer) {
        0
    } else if controls.contains_predicate(ControlPredicate::ApprovedRelay) {
        P3_RELAY_HOPS
    } else {
        0
    }
}

/// Crypto work units: `1 << profile_rank` (P0=1, P1=2, P2=4, P3=8, P4=16).
pub fn crypto_work_units(profile: ProtectionProfile) -> u64 {
    1u64 << profile.floor_rank()
}

pub fn estimate_overhead(
    profile: ProtectionProfile,
    controls: &ControlSet,
    application_bytes: u64,
    session_duration_ms: u64,
    retry_count: u64,
    drop_cover: bool,
) -> Result<OverheadBudget, QdnfError> {
    let cover_mandatory = controls.contains_predicate(ControlPredicate::CoverTraffic);
    if drop_cover && cover_mandatory {
        return Err(QdnfError::Downgrade);
    }
    let cover_on = cover_mandatory && !drop_cover;

    if profile == ProtectionProfile::P0 && application_bytes != 0 {
        return Err(QdnfError::Denied);
    }

    let bucket = record_bucket(profile);
    let padded = if bucket == 0 {
        0
    } else {
        pad_to_next_bucket(application_bytes, bucket)?
    };
    let padding_bytes = padded
        .checked_sub(application_bytes)
        .ok_or(QdnfError::Range)?;
    let hops = relay_hops(controls);
    let relay_copies = hops.checked_mul(padded).ok_or(QdnfError::Range)?;
    let retry_bytes = retry_count.checked_mul(padded).ok_or(QdnfError::Range)?;

    Ok(OverheadBudget {
        padding_bytes,
        relay_copies,
        handshake_bytes: handshake_bytes_hybrid(),
        cover_bytes: cover_bytes_for(session_duration_ms, cover_on)?,
        retry_bytes,
        storage_bytes: padded,
        crypto_work: crypto_work_units(profile),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::profiles::catalog::{catalog_entry, control_set_from_predicates};

    #[test]
    fn padding_bucket_256() {
        assert_eq!(pad_to_next_bucket(1, P1_PADDING_BUCKET).unwrap(), 256);
        assert_eq!(pad_to_next_bucket(256, P1_PADDING_BUCKET).unwrap(), 256);
        assert_eq!(pad_to_next_bucket(257, P1_PADDING_BUCKET).unwrap(), 512);
        assert_eq!(pad_to_next_bucket(0, P1_PADDING_BUCKET).unwrap(), 0);
    }

    #[test]
    fn padding_formula_is_ceiling_not_a_lookup() {
        let bucket = P1_PADDING_BUCKET;
        let size = 257u64;
        let expected = ((size + bucket - 1) / bucket) * bucket;
        assert_eq!(expected, 512);
        assert_eq!(pad_to_next_bucket(size, bucket).unwrap(), expected);
    }

    #[test]
    fn p1_one_byte_charges_padding_and_storage_separately() {
        let controls = catalog_entry(ProtectionProfile::P1);
        let app = 1u64;
        let budget = estimate_overhead(ProtectionProfile::P1, &controls, app, 0, 0, false).unwrap();
        let padded = pad_to_next_bucket(app, P1_PADDING_BUCKET).unwrap();
        assert_eq!(padded, 256);
        assert_eq!(budget.padding_bytes, padded - app);
        assert_eq!(budget.padding_bytes, 255);
        assert_eq!(budget.storage_bytes, padded);
        assert_eq!(budget.relay_copies, 0);
        assert_eq!(budget.cover_bytes, 0);
        assert_eq!(budget.handshake_bytes, handshake_bytes_hybrid());
        assert_eq!(budget.retry_bytes, 0);
        let total = budget.padding_bytes
            + budget.relay_copies
            + budget.handshake_bytes
            + budget.cover_bytes
            + budget.retry_bytes
            + budget.storage_bytes;
        assert_eq!(budget.total_bytes().unwrap(), total);
        assert_eq!(total, 255 + handshake_bytes_hybrid() + 256);
        assert_eq!(budget.crypto_work, 2);
    }

    #[test]
    fn p2_default_record_is_4kib() {
        let controls = catalog_entry(ProtectionProfile::P2);
        let budget = estimate_overhead(ProtectionProfile::P2, &controls, 1, 0, 0, false).unwrap();
        assert_eq!(pad_to_next_bucket(1, P2_RECORD_BYTES).unwrap(), 4096);
        assert_eq!(budget.padding_bytes, 4095);
        assert_eq!(budget.storage_bytes, 4096);
        assert_eq!(budget.crypto_work, 4);
    }

    #[test]
    fn p3_relay_copies_are_two_hops_of_padded_record() {
        let controls = catalog_entry(ProtectionProfile::P3);
        let app = 1u64;
        let budget = estimate_overhead(ProtectionProfile::P3, &controls, app, 0, 0, false).unwrap();
        let padded = pad_to_next_bucket(app, P3_CELL_BYTES).unwrap();
        assert_eq!(padded, 4096);
        assert_eq!(budget.relay_copies, P3_RELAY_HOPS * padded);
        assert_eq!(budget.relay_copies, 8192);
        assert_eq!(budget.cover_bytes, 0);
        assert_eq!(budget.crypto_work, 8);
    }

    #[test]
    fn mandatory_cover_cannot_be_dropped() {
        let mut predicates = [ControlPredicate::Authenticity; 10];
        predicates[0] = ControlPredicate::Authenticity;
        predicates[1] = ControlPredicate::Provenance;
        predicates[2] = ControlPredicate::Integrity;
        predicates[3] = ControlPredicate::ResourceControls;
        predicates[4] = ControlPredicate::E2eConfidentiality;
        predicates[5] = ControlPredicate::PrivateContacts;
        predicates[6] = ControlPredicate::LabelInheritance;
        predicates[7] = ControlPredicate::Revocation;
        predicates[8] = ControlPredicate::ProtectedLocalKeys;
        predicates[9] = ControlPredicate::CoverTraffic;
        let controls = control_set_from_predicates(ProtectionProfile::P1, &predicates).unwrap();
        assert_eq!(
            estimate_overhead(ProtectionProfile::P1, &controls, 1, 1000, 0, true),
            Err(QdnfError::Downgrade)
        );
        let kept = estimate_overhead(ProtectionProfile::P1, &controls, 1, 1000, 0, false).unwrap();
        assert_eq!(kept.cover_bytes, P3_COVER_BYTES_PER_SEC);
    }

    #[test]
    fn optional_cover_may_be_dropped() {
        let controls = catalog_entry(ProtectionProfile::P1);
        assert!(!controls.contains_predicate(ControlPredicate::CoverTraffic));
        let dropped =
            estimate_overhead(ProtectionProfile::P1, &controls, 1, 1000, 0, true).unwrap();
        assert_eq!(dropped.cover_bytes, 0);
    }

    #[test]
    fn isolated_bearer_has_zero_relay_copies() {
        let controls = catalog_entry(ProtectionProfile::P4);
        let budget = estimate_overhead(ProtectionProfile::P4, &controls, 1, 0, 0, false).unwrap();
        assert_eq!(budget.relay_copies, 0);
        assert_eq!(budget.crypto_work, 16);
    }

    #[test]
    fn handshake_bytes_match_named_lengths() {
        let expected = (ML_KEM_768_PK_LEN
            + X25519_LEN
            + ML_KEM_768_CT_LEN
            + X25519_LEN
            + ML_DSA_65_SIG_LEN
            + ED25519_SIG_LEN) as u64;
        assert_eq!(handshake_bytes_hybrid(), expected);
        assert_eq!(handshake_bytes_hybrid(), 5709);
    }

    #[test]
    fn total_bytes_checked_add_overflows() {
        let budget = OverheadBudget {
            padding_bytes: u64::MAX,
            relay_copies: 1,
            handshake_bytes: 0,
            cover_bytes: 0,
            retry_bytes: 0,
            storage_bytes: 0,
            crypto_work: 0,
        };
        assert_eq!(budget.total_bytes(), Err(QdnfError::Range));
    }

    #[test]
    fn p0_rejects_application_payload() {
        let controls = catalog_entry(ProtectionProfile::P0);
        assert_eq!(
            estimate_overhead(ProtectionProfile::P0, &controls, 1, 0, 0, false),
            Err(QdnfError::Denied)
        );
        let empty = estimate_overhead(ProtectionProfile::P0, &controls, 0, 0, 0, false).unwrap();
        assert_eq!(empty.padding_bytes, 0);
        assert_eq!(empty.storage_bytes, 0);
        assert_eq!(empty.handshake_bytes, handshake_bytes_hybrid());
    }
}

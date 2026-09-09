//! Cover traffic, padding classes and relay separation (E16.3).
//!
//! Reuses [`super::budget`] padding buckets, cell size and the P3 cover
//! byte-rate. Cover that is mandatory (the `CoverTraffic` predicate) cannot
//! be dropped for cost ([`QdnfError::Downgrade`]).
//!
//! # Observer / collusion assumptions (not anonymity)
//!
//! This profile assumes **one network observer** on the path. Approved
//! relays are assumed **not colluding** with each other or that observer
//! under P3. P4 replaces relays with [`ControlPredicate::IsolatedBearer`]
//! because relay collusion is in-scope for compartmented missions.
//!
//! These are threat-model assumptions. They are **not** an anonymity claim.
//! Byte padding alone does not prove resistance to timing correlation.
//! [`correlation_resistance_measured`] is false until a separate measurement
//! harness reports otherwise.
//!
//! Remote wipe and attestation cannot guarantee safety after endpoint
//! compromise or seizure (E16.7).

use super::budget::{
    record_bucket, P1_PADDING_BUCKET, P2_RECORD_BYTES, P3_CELL_BYTES, P3_COVER_BYTES_PER_SEC,
};
use super::catalog::{catalog_entry, ControlPredicate, ControlSet, ProtectionProfile};
use crate::net::qdnf::errors::QdnfError;

/// Padded traffic class. `cover_bps` holds the P3 cover *byte* rate (4 KiB/s)
/// when the class budgets cover; it is not a bits/s anonymity parameter.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverClass {
    pub pad_bucket: u32,
    pub batch_max: u8,
    pub cover_bps: u64,
}

impl CoverClass {
    pub const NONE: Self = Self {
        pad_bucket: 0,
        batch_max: 1,
        cover_bps: 0,
    };
}

/// True when the named catalog profile itself lists `CoverTraffic`.
/// Catalog P0–P4 do not: P3 cover is optional unless the predicate is added.
pub fn cover_mandatory(profile: ProtectionProfile) -> bool {
    catalog_entry(profile).contains_predicate(ControlPredicate::CoverTraffic)
}

/// Cover required by the selected control set (Recipe F / S13).
pub fn cover_mandatory_controls(controls: &ControlSet) -> bool {
    controls.contains_predicate(ControlPredicate::CoverTraffic)
}

/// Dropping cover because of cost. Mandatory cover is a downgrade, not a
/// cheaper compatible profile.
pub fn drop_cover_for_cost(profile: ProtectionProfile) -> Result<(), QdnfError> {
    if cover_mandatory(profile) {
        Err(QdnfError::Downgrade)
    } else {
        Ok(())
    }
}

pub fn drop_cover_for_controls(controls: &ControlSet) -> Result<(), QdnfError> {
    if cover_mandatory_controls(controls) {
        Err(QdnfError::Downgrade)
    } else {
        Ok(())
    }
}

pub fn cover_class(profile: ProtectionProfile) -> CoverClass {
    let pad_bucket = record_bucket(profile) as u32;
    match profile {
        ProtectionProfile::P0 => CoverClass::NONE,
        ProtectionProfile::P1 => CoverClass {
            pad_bucket,
            batch_max: 1,
            cover_bps: 0,
        },
        ProtectionProfile::P2 => CoverClass {
            pad_bucket,
            batch_max: 4,
            cover_bps: 0,
        },
        ProtectionProfile::P3 | ProtectionProfile::P4 => CoverClass {
            pad_bucket,
            batch_max: 8,
            cover_bps: P3_COVER_BYTES_PER_SEC,
        },
    }
}

/// P3 lists ApprovedRelay. P4 uses IsolatedBearer instead of remote relays.
pub fn approved_relay_required(profile: ProtectionProfile) -> bool {
    catalog_entry(profile).contains_predicate(ControlPredicate::ApprovedRelay)
}

pub fn isolated_bearer_required(profile: ProtectionProfile) -> bool {
    catalog_entry(profile).contains_predicate(ControlPredicate::IsolatedBearer)
}

/// Honest: no correlation-resistance campaign has been measured here.
pub fn correlation_resistance_measured() -> bool {
    false
}

pub const fn p1_padding_bucket() -> u64 {
    P1_PADDING_BUCKET
}

pub const fn p3_cell_bytes() -> u64 {
    P3_CELL_BYTES
}

pub const fn p2_record_bytes() -> u64 {
    P2_RECORD_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::profiles::budget::estimate_overhead;
    use crate::net::qdnf::profiles::catalog::control_set_from_predicates;

    #[test]
    fn catalog_profiles_do_not_mandate_cover() {
        assert!(!cover_mandatory(ProtectionProfile::P0));
        assert!(!cover_mandatory(ProtectionProfile::P1));
        assert!(!cover_mandatory(ProtectionProfile::P2));
        assert!(!cover_mandatory(ProtectionProfile::P3));
        assert!(!cover_mandatory(ProtectionProfile::P4));
        assert_eq!(drop_cover_for_cost(ProtectionProfile::P3), Ok(()));
    }

    #[test]
    fn mandatory_cover_cannot_be_dropped_for_cost() {
        let controls = control_set_from_predicates(
            ProtectionProfile::P3,
            &[
                ControlPredicate::Authenticity,
                ControlPredicate::CoverTraffic,
            ],
        )
        .unwrap();
        assert!(cover_mandatory_controls(&controls));
        assert_eq!(
            drop_cover_for_controls(&controls),
            Err(QdnfError::Downgrade)
        );
        assert_eq!(
            estimate_overhead(ProtectionProfile::P3, &controls, 1, 1000, 0, true),
            Err(QdnfError::Downgrade)
        );
    }

    #[test]
    fn cover_class_reuses_budget_buckets() {
        let p1 = cover_class(ProtectionProfile::P1);
        assert_eq!(p1.pad_bucket, P1_PADDING_BUCKET as u32);
        assert_eq!(p1.batch_max, 1);
        assert_eq!(p1.cover_bps, 0);
        let p3 = cover_class(ProtectionProfile::P3);
        assert_eq!(p3.pad_bucket, P3_CELL_BYTES as u32);
        assert_eq!(p3.batch_max, 8);
        assert_eq!(p3.cover_bps, P3_COVER_BYTES_PER_SEC);
        let p4 = cover_class(ProtectionProfile::P4);
        assert_eq!(p4.pad_bucket, p3.pad_bucket);
        assert_eq!(p4.cover_bps, P3_COVER_BYTES_PER_SEC);
    }

    #[test]
    fn p3_requires_approved_relay_p4_isolated() {
        assert!(approved_relay_required(ProtectionProfile::P3));
        assert!(!isolated_bearer_required(ProtectionProfile::P3));
        assert!(!approved_relay_required(ProtectionProfile::P4));
        assert!(isolated_bearer_required(ProtectionProfile::P4));
    }

    #[test]
    fn correlation_resistance_is_unmeasured() {
        assert!(!correlation_resistance_measured());
    }
}

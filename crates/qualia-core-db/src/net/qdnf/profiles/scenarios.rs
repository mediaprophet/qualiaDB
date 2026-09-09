//! Generic protected-transport fixtures for hostile-environment profiles (E16.6).
//!
//! Clinical application scenarios belong in E14/E21. These cases use only
//! profile, vault, cover, offline and gateway APIs.
//!
//! S12 (P3 required, only P1 available) lives in `negotiate.rs` and is
//! re-exercised here so the table stays complete. S13/S14 are cover/observer
//! cases, not clinical metadata claims.
//!
//! Remote wipe and attestation cannot guarantee safety after endpoint
//! compromise or seizure (E16.7).

use super::catalog::{
    catalog_entry, control_set_from_predicates, ControlPredicate, ProtectionProfile,
};
use super::cover::{correlation_resistance_measured, cover_class, drop_cover_for_controls};
use super::gateway::{gateway_transfer, GatewayMedia};
use super::negotiate::{negotiate, negotiate_outcome, CostPreference, NegotiateOutcome};
use super::offline::{
    activate_capability, queue_offline, release_queued, OfflinePackage, OfflineQueue,
    ScopedCapability,
};
use super::vault::{
    crash_dump_redacted, hardware_key_ref_supported, load_ref, lock, require_hardware_backed_p4,
    store_ref, LocalVault,
};
use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{
    encode_label_into, verify_label, Confidentiality, LabelFields, NO_TRAINING,
};
use crate::net::qdnf::types::{Generation, StrongDigest};

fn digest(b: u8) -> StrongDigest {
    let mut d = StrongDigest::ZERO;
    d.0[0] = b;
    d.0[47] = 11;
    d
}

fn fat_ledger() -> ReservationLedger {
    let cap = ResourceBudget {
        bytes: 1 << 20,
        work: 1 << 20,
        io: 1 << 10,
    };
    ReservationLedger::new(cap, cap, cap, cap, cap)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    S12MaliciousAp,
    Seizure,
    HostileRelay,
    ClockRollback,
    KeyServiceOutage,
    CoercedAccess,
    S13MandatoryCover,
    S14Observer,
    InsiderMisuse,
}

fn run(case: Case) -> Result<(), QdnfError> {
    match case {
        Case::S12MaliciousAp => {
            let required = catalog_entry(ProtectionProfile::P3);
            let available = catalog_entry(ProtectionProfile::P1);
            let mut ledger = fat_ledger();
            match negotiate_outcome(
                &required,
                &available,
                &available,
                CostPreference::LeastCost,
                false,
                &mut ledger,
            )? {
                NegotiateOutcome::Unavailable => Ok(()),
                NegotiateOutcome::Selected(_) => Err(QdnfError::Downgrade),
            }
        }
        Case::Seizure => {
            let mut vault = LocalVault::new();
            let slot = store_ref(&mut vault, digest(1), 10, 100)?;
            lock(&mut vault);
            match load_ref(&mut vault, slot, 11) {
                Err(QdnfError::Denied) if crash_dump_redacted(&vault) => Ok(()),
                Err(e) => Err(e),
                Ok(_) => Err(QdnfError::Denied),
            }
        }
        Case::HostileRelay => {
            let required = catalog_entry(ProtectionProfile::P3);
            let available = catalog_entry(ProtectionProfile::P2);
            let mut ledger = fat_ledger();
            match negotiate(
                &required,
                &available,
                &available,
                CostPreference::LeastCost,
                false,
                &mut ledger,
            ) {
                Err(QdnfError::UnknownProfile) | Err(QdnfError::Denied) => Ok(()),
                Err(e) => Err(e),
                Ok(_) => Err(QdnfError::Downgrade),
            }
        }
        Case::ClockRollback => {
            let mut q = OfflineQueue::new();
            let pkg = OfflinePackage {
                recipient: digest(2),
                expiry_unix: 100,
                ciphertext_len: 32,
                digest: digest(3),
            };
            let slot = queue_offline(&mut q, pkg, 50)?;
            let exp = q.get(slot)?.expiry_unix;
            match release_queued(&mut q, slot, 40, Generation(1), Generation(1)) {
                Err(QdnfError::Conflict) if exp == 100 => Ok(()),
                Err(QdnfError::Expired) if exp == 100 => Ok(()),
                Err(e) => Err(e),
                Ok(()) => Err(QdnfError::Downgrade),
            }
        }
        Case::KeyServiceOutage => {
            if hardware_key_ref_supported() {
                return Err(QdnfError::Downgrade);
            }
            match require_hardware_backed_p4() {
                Err(QdnfError::UnknownProfile) => Ok(()),
                Err(e) => Err(e),
                Ok(()) => Err(QdnfError::Downgrade),
            }
        }
        Case::CoercedAccess => {
            let mut vault = LocalVault::new();
            let slot = store_ref(&mut vault, digest(4), 1, 80)?;
            lock(&mut vault);
            lock(&mut vault);
            if !vault.is_locked() {
                return Err(QdnfError::Downgrade);
            }
            match (
                load_ref(&mut vault, slot, 2),
                store_ref(&mut vault, digest(5), 3, 80),
            ) {
                (Err(QdnfError::Denied), Err(QdnfError::Denied)) => Ok(()),
                (Err(e), _) | (_, Err(e)) => Err(e),
                (Ok(_), Ok(_)) => Err(QdnfError::Denied),
            }
        }
        Case::S13MandatoryCover => {
            // P3 catalog is already MAX_CONTROLS; extra CoverTraffic is a
            // selected predicate, not a 17th catalog slot.
            let set = control_set_from_predicates(
                ProtectionProfile::P3,
                &[
                    ControlPredicate::ApprovedRelay,
                    ControlPredicate::CoverTraffic,
                ],
            )?;
            match drop_cover_for_controls(&set) {
                Err(QdnfError::Downgrade) => Ok(()),
                Err(e) => Err(e),
                Ok(()) => Err(QdnfError::Downgrade),
            }
        }
        Case::S14Observer => {
            let class = cover_class(ProtectionProfile::P3);
            if correlation_resistance_measured() {
                return Err(QdnfError::Downgrade);
            }
            if class.cover_bps == 0 || class.pad_bucket == 0 {
                return Err(QdnfError::Range);
            }
            Ok(())
        }
        Case::InsiderMisuse => {
            let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
            fields.audience = digest(1);
            fields.restriction_bits = NO_TRAINING;
            let mut buf = [0u8; 256];
            let n = encode_label_into(&fields, &mut buf)?;
            let src = verify_label(fields, &buf[..n])?;
            let dst = src;
            let mut proposed = *src.fields();
            proposed.restriction_bits = 0;
            match gateway_transfer(
                &src,
                &dst,
                &proposed,
                digest(1),
                GatewayMedia::Network,
                ProtectionProfile::P3,
            ) {
                Err(QdnfError::Denied) => Ok(()),
                Err(e) => Err(e),
                Ok(()) => Err(QdnfError::Denied),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::profiles::negotiate::negotiate;
    use crate::net::qdnf::profiles::vault::notification_preview_max_bytes;

    const TABLE: &[(Case, &str)] = &[
        (Case::S12MaliciousAp, "S12/malicious-ap"),
        (Case::Seizure, "seizure"),
        (Case::HostileRelay, "hostile-relay"),
        (Case::ClockRollback, "clock-rollback"),
        (Case::KeyServiceOutage, "key-service-outage"),
        (Case::CoercedAccess, "coerced-access"),
        (Case::S13MandatoryCover, "S13-mandatory-cover"),
        (Case::S14Observer, "S14-observer"),
        (Case::InsiderMisuse, "insider-misuse"),
    ];

    #[test]
    fn hostile_environment_table() {
        let mut i = 0usize;
        while i < TABLE.len() {
            let (case, name) = TABLE[i];
            run(case).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            i += 1;
        }
    }

    #[test]
    fn s12_negotiate_still_unknown_profile() {
        let required = catalog_entry(ProtectionProfile::P3);
        let available = catalog_entry(ProtectionProfile::P1);
        let mut ledger = fat_ledger();
        assert_eq!(
            negotiate(
                &required,
                &available,
                &available,
                CostPreference::LeastCost,
                false,
                &mut ledger
            )
            .unwrap_err(),
            QdnfError::UnknownProfile
        );
    }

    #[test]
    fn capability_deadline_independent_of_copy() {
        let cap = ScopedCapability {
            expires_unix: 5,
            purpose: digest(8),
            recipient: digest(9),
        };
        let copy = cap;
        assert_eq!(activate_capability(&copy, 5), Err(QdnfError::Expired));
    }

    #[test]
    fn seizure_preview_is_not_full_payload() {
        assert!(notification_preview_max_bytes() <= 32);
    }

    #[test]
    fn key_outage_does_not_select_software_p4_as_hardware() {
        assert!(!hardware_key_ref_supported());
        assert_eq!(require_hardware_backed_p4(), Err(QdnfError::UnknownProfile));
        let p4 = catalog_entry(ProtectionProfile::P4);
        let mut ledger = fat_ledger();
        let selected =
            negotiate(&p4, &p4, &p4, CostPreference::LeastCost, false, &mut ledger).unwrap();
        assert_eq!(selected.profile, ProtectionProfile::P4);
        assert!(!hardware_key_ref_supported());
    }
}

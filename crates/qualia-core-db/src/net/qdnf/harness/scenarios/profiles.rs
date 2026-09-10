//! Profile, cover, gateway, offline and routing fixtures.

use super::common::{digest, expect_err, verified};
use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{encode_label_into, verify_label, Confidentiality, LabelFields};
use crate::net::qdnf::profiles::{
    catalog_entry, correlation_resistance_measured, cover_class, drop_cover_for_controls,
    gateway_transfer, negotiate, queue_offline, release_queued, control_set_from_predicates,
    CostPreference, ControlPredicate, GatewayMedia, OfflinePackage, OfflineQueue, ProtectionProfile,
};
use crate::net::qdnf::route::{plan_routes, AdjacencyIndex, CandidatePath, PathConstraint};
use crate::net::qdnf::types::Generation;

fn fat_ledger() -> ReservationLedger {
    let cap = ResourceBudget {
        bytes: 1 << 20,
        work: 1 << 20,
        io: 1 << 10,
    };
    ReservationLedger::new(cap, cap, cap, cap, cap)
}

pub fn s12_no_downgrade() -> Result<(), QdnfError> {
    let required = catalog_entry(ProtectionProfile::P3);
    let available = catalog_entry(ProtectionProfile::P1);
    let mut ledger = fat_ledger();
    match negotiate(
        &required,
        &available,
        &available,
        CostPreference::LeastCost,
        false,
        &mut ledger,
    ) {
        Err(QdnfError::UnknownProfile) | Err(QdnfError::Downgrade) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Downgrade),
    }
}

pub fn s13_mandatory_cover() -> Result<(), QdnfError> {
    let set = control_set_from_predicates(
        ProtectionProfile::P3,
        &[
            ControlPredicate::ApprovedRelay,
            ControlPredicate::CoverTraffic,
        ],
    )?;
    expect_err(drop_cover_for_controls(&set), QdnfError::Downgrade)
}

pub fn s14_observer() -> Result<(), QdnfError> {
    if correlation_resistance_measured() {
        return Err(QdnfError::Downgrade);
    }
    let class = cover_class(ProtectionProfile::P3);
    if class.cover_bps == 0 || class.pad_bucket == 0 {
        return Err(QdnfError::Range);
    }
    Ok(())
}

pub fn s21_clock_rollback() -> Result<(), QdnfError> {
    let mut q = OfflineQueue::new();
    let pkg = OfflinePackage {
        recipient: digest(2),
        expiry_unix: 100,
        ciphertext_len: 32,
        digest: digest(3),
    };
    let slot = queue_offline(&mut q, pkg, 50)?;
    match release_queued(&mut q, slot, 40, Generation(1), Generation(1)) {
        Err(QdnfError::Conflict) | Err(QdnfError::Expired) => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Err(QdnfError::Downgrade),
    }
}

pub fn s25_gateway_mapping() -> Result<(), QdnfError> {
    let src = verified(Confidentiality::C2Sensitive, digest(1))?;
    let mut dst_fields = LabelFields::request(Confidentiality::C0Public, digest(2));
    dst_fields.audience = digest(2);
    let mut buf = [0u8; 256];
    let n = encode_label_into(&dst_fields, &mut buf)?;
    let dst = verify_label(dst_fields, &buf[..n])?;
    let mut proposed = *src.fields();
    proposed.confidentiality = Confidentiality::C0Public;
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

pub fn s35_no_routes() -> Result<(), QdnfError> {
    let index = AdjacencyIndex::new(1);
    let mut out = [CandidatePath::EMPTY; 3];
    match plan_routes(&index, 1, 2, &PathConstraint::UNRESTRICTED, 1, &mut out) {
        Err(QdnfError::NoRoute) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::NoRoute),
    }
}

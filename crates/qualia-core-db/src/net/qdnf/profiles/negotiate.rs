//! Profile negotiation: compatible control sets only. Never downgrade for cost.

use crate::crypto::network::transcript::Transcript;
use crate::net::peer::runtime::{ReservationHandle, ReservationLedger};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::budget::{estimate_overhead, OverheadBudget};
use super::catalog::{catalog_entry, ControlId, ControlPredicate, ControlSet, ProtectionProfile};

const CANDIDATES: [ProtectionProfile; 5] = [
    ProtectionProfile::P0,
    ProtectionProfile::P1,
    ProtectionProfile::P2,
    ProtectionProfile::P3,
    ProtectionProfile::P4,
];

const DEFAULT_APP_BYTES_PRIVATE: u64 = 1;
const DEFAULT_DURATION_MS: u64 = 1000;
const DEFAULT_RETRIES: u64 = 0;

/// Organisational cost ranking among *feasible* configs. Cannot lower the
/// required floor. `PaidTier` is the S13 case: same feasible set as `LeastCost`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostPreference {
    LeastCost = 0,
    PaidTier = 1,
}

#[derive(Debug)]
pub enum NegotiateOutcome {
    Selected(SelectedProfile),
    Unavailable,
}

#[derive(Debug)]
pub struct SelectedProfile {
    pub profile: ProtectionProfile,
    pub controls: ControlSet,
    pub overhead: OverheadBudget,
    pub version_digest: StrongDigest,
    pub reservation: ReservationHandle,
}

struct Candidate {
    profile: ProtectionProfile,
    controls: ControlSet,
    overhead: OverheadBudget,
}

fn approved_relay_id() -> ControlId {
    ControlId::from_predicate(ControlPredicate::ApprovedRelay)
}

fn isolated_bearer_id() -> ControlId {
    ControlId::from_predicate(ControlPredicate::IsolatedBearer)
}

/// `IsolatedBearer` may replace `ApprovedRelay` (P4 isolated local bearer).
fn control_satisfied(available: &ControlSet, need: ControlId) -> bool {
    if available.contains(need) {
        return true;
    }
    need == approved_relay_id() && available.contains(isolated_bearer_id())
}

fn satisfies(available: &ControlSet, required: &ControlSet) -> bool {
    let mut i = 0usize;
    while i < required.control_count as usize {
        if !control_satisfied(available, required.controls[i]) {
            return false;
        }
        i += 1;
    }
    true
}

fn union_required_onto_catalog(
    catalog: &ControlSet,
    required: &ControlSet,
) -> Result<ControlSet, QdnfError> {
    let mut out = *catalog;
    let mut i = 0usize;
    while i < required.control_count as usize {
        let id = required.controls[i];
        if id == approved_relay_id() && catalog.contains(isolated_bearer_id()) {
            i += 1;
            continue;
        }
        out.try_push(id)?;
        i += 1;
    }
    Ok(out)
}

/// SHA-384 transcript of selected profile id + control-set digest.
pub fn bind_version_digest(
    profile: ProtectionProfile,
    controls_digest: StrongDigest,
) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"profile", &[profile.to_u8()])?;
    t.append(b"controls", controls_digest.as_bytes())?;
    Ok(t.digest())
}

fn application_bytes(profile: ProtectionProfile) -> u64 {
    match profile {
        ProtectionProfile::P0 => 0,
        _ => DEFAULT_APP_BYTES_PRIVATE,
    }
}

fn collect_feasible(
    required: &ControlSet,
    peer_available: &ControlSet,
    local_available: &ControlSet,
    out: &mut [Option<Candidate>; 5],
) -> Result<usize, QdnfError> {
    if !satisfies(peer_available, required) || !satisfies(local_available, required) {
        return Err(QdnfError::UnknownProfile);
    }
    let floor = required.profile.floor_rank();
    let mut n = 0usize;
    let mut i = 0usize;
    while i < CANDIDATES.len() {
        let profile = CANDIDATES[i];
        if profile.floor_rank() < floor {
            i += 1;
            continue;
        }
        let catalog = catalog_entry(profile);
        if !peer_available.contains_all(&catalog) || !local_available.contains_all(&catalog) {
            i += 1;
            continue;
        }
        let controls = if profile == required.profile {
            *required
        } else {
            union_required_onto_catalog(&catalog, required)?
        };
        if !satisfies(peer_available, &controls) || !satisfies(local_available, &controls) {
            i += 1;
            continue;
        }
        let overhead = estimate_overhead(
            profile,
            &controls,
            application_bytes(profile),
            DEFAULT_DURATION_MS,
            DEFAULT_RETRIES,
            false,
        )?;
        out[n] = Some(Candidate {
            profile,
            controls,
            overhead,
        });
        n += 1;
        i += 1;
    }
    if n == 0 {
        return Err(QdnfError::UnknownProfile);
    }
    Ok(n)
}

fn pick_candidate(
    slots: &[Option<Candidate>; 5],
    count: usize,
    extra_protection: bool,
) -> Result<usize, QdnfError> {
    if count == 0 {
        return Err(QdnfError::UnknownProfile);
    }
    if extra_protection {
        return Ok(count - 1);
    }
    let mut best = 0usize;
    let mut best_bytes = slots[0]
        .as_ref()
        .ok_or(QdnfError::UnknownProfile)?
        .overhead
        .total_bytes()?;
    let mut i = 1usize;
    while i < count {
        let bytes = slots[i]
            .as_ref()
            .ok_or(QdnfError::UnknownProfile)?
            .overhead
            .total_bytes()?;
        if bytes < best_bytes {
            best = i;
            best_bytes = bytes;
        }
        i += 1;
    }
    Ok(best)
}

pub fn negotiate(
    required: &ControlSet,
    peer_available: &ControlSet,
    local_available: &ControlSet,
    cost_preference: CostPreference,
    extra_protection: bool,
    ledger: &mut ReservationLedger,
) -> Result<SelectedProfile, QdnfError> {
    let _ = cost_preference;
    let mut slots: [Option<Candidate>; 5] = [None, None, None, None, None];
    let count = collect_feasible(required, peer_available, local_available, &mut slots)?;
    let idx = pick_candidate(&slots, count, extra_protection)?;
    let chosen = slots[idx].take().ok_or(QdnfError::UnknownProfile)?;
    let resource = chosen.overhead.to_resource_budget()?;
    let reservation = match ledger.reserve(resource, true) {
        Ok(handle) => handle,
        Err(QdnfError::BudgetExhausted) => return Err(QdnfError::BudgetExhausted),
        Err(e) => return Err(e),
    };
    let version_digest = bind_version_digest(chosen.profile, chosen.controls.digest)?;
    Ok(SelectedProfile {
        profile: chosen.profile,
        controls: chosen.controls,
        overhead: chosen.overhead,
        version_digest,
        reservation,
    })
}

/// Maps profile-incompatibility to [`NegotiateOutcome::Unavailable`].
/// Budget and other failures stay typed on [`negotiate`].
pub fn negotiate_outcome(
    required: &ControlSet,
    peer_available: &ControlSet,
    local_available: &ControlSet,
    cost_preference: CostPreference,
    extra_protection: bool,
    ledger: &mut ReservationLedger,
) -> Result<NegotiateOutcome, QdnfError> {
    match negotiate(
        required,
        peer_available,
        local_available,
        cost_preference,
        extra_protection,
        ledger,
    ) {
        Ok(selected) => Ok(NegotiateOutcome::Selected(selected)),
        Err(QdnfError::UnknownProfile) | Err(QdnfError::Denied) => {
            Ok(NegotiateOutcome::Unavailable)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
    use crate::net::qdnf::policy_labels::Confidentiality;
    use crate::net::qdnf::profiles::catalog::catalog_entry;
    use crate::net::qdnf::profiles::requirements::{
        required_control_set, AssessedEnvironment, RequirementContext, TaskClass,
    };

    fn fat_ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: 1 << 20,
            work: 1 << 20,
            io: 1 << 10,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    fn tight_ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: 1,
            work: 1,
            io: 1,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    #[test]
    fn p3_required_only_p1_available_is_unknown_profile() {
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
        assert_eq!(ledger.used().host.bytes, 0);
        match negotiate_outcome(
            &required,
            &available,
            &available,
            CostPreference::LeastCost,
            false,
            &mut ledger,
        )
        .unwrap()
        {
            NegotiateOutcome::Unavailable => {}
            NegotiateOutcome::Selected(_) => panic!("downgraded to P1"),
        }
    }

    #[test]
    fn cost_preference_cannot_select_p1_when_c2_requires_p2() {
        let ctx = RequirementContext::new(
            Confidentiality::C2Sensitive,
            TaskClass::KnownPeerMessage,
            AssessedEnvironment::Friendly,
        );
        let required = required_control_set(&ctx).unwrap();
        assert_eq!(required.profile, ProtectionProfile::P2);
        let available = catalog_entry(ProtectionProfile::P2);
        let mut ledger = fat_ledger();
        let selected = negotiate(
            &required,
            &available,
            &available,
            CostPreference::PaidTier,
            false,
            &mut ledger,
        )
        .unwrap();
        assert_eq!(selected.profile, ProtectionProfile::P2);
        assert!(selected
            .controls
            .contains_predicate(ControlPredicate::ProtectedEndpoint));
        assert_ne!(selected.profile, ProtectionProfile::P1);
    }

    #[test]
    fn extra_protection_raises_p1_to_p2_when_both_available() {
        let required = catalog_entry(ProtectionProfile::P1);
        let available = catalog_entry(ProtectionProfile::P2);
        let mut ledger = fat_ledger();
        let plain = negotiate(
            &required,
            &available,
            &available,
            CostPreference::LeastCost,
            false,
            &mut ledger,
        )
        .unwrap();
        assert_eq!(plain.profile, ProtectionProfile::P1);
        let raised = negotiate(
            &required,
            &available,
            &available,
            CostPreference::LeastCost,
            true,
            &mut ledger,
        )
        .unwrap();
        assert_eq!(raised.profile, ProtectionProfile::P2);
        assert!(raised
            .controls
            .contains_predicate(ControlPredicate::ProtectedEndpoint));
    }

    #[test]
    fn missing_approved_relay_when_p3_required_fails_closed() {
        let required = catalog_entry(ProtectionProfile::P3);
        let available = catalog_entry(ProtectionProfile::P2);
        assert!(!available.contains_predicate(ControlPredicate::ApprovedRelay));
        let mut ledger = fat_ledger();
        let err = negotiate(
            &required,
            &available,
            &available,
            CostPreference::LeastCost,
            false,
            &mut ledger,
        )
        .unwrap_err();
        assert!(err == QdnfError::UnknownProfile || err == QdnfError::Denied);
        assert_eq!(ledger.used().host.bytes, 0);
    }

    #[test]
    fn budget_exhausted_does_not_select_anyway() {
        let required = catalog_entry(ProtectionProfile::P1);
        let available = catalog_entry(ProtectionProfile::P1);
        let mut ledger = tight_ledger();
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
            QdnfError::BudgetExhausted
        );
        assert_eq!(ledger.used().host.bytes, 0);
        assert_eq!(ledger.used().host.work, 0);
    }

    #[test]
    fn selected_digest_binds_profile_id_and_controls() {
        let required = catalog_entry(ProtectionProfile::P1);
        let available = catalog_entry(ProtectionProfile::P1);
        let mut ledger = fat_ledger();
        let selected = negotiate(
            &required,
            &available,
            &available,
            CostPreference::LeastCost,
            false,
            &mut ledger,
        )
        .unwrap();
        let expected = bind_version_digest(selected.profile, selected.controls.digest).unwrap();
        assert_eq!(selected.version_digest, expected);
        let other = bind_version_digest(ProtectionProfile::P2, selected.controls.digest).unwrap();
        assert_ne!(selected.version_digest, other);
        let other_controls = catalog_entry(ProtectionProfile::P2);
        let other_bind = bind_version_digest(selected.profile, other_controls.digest).unwrap();
        assert_ne!(selected.version_digest, other_bind);
    }

    #[test]
    fn no_feasible_config_does_not_fall_back_to_p0() {
        let required = catalog_entry(ProtectionProfile::P2);
        let available = catalog_entry(ProtectionProfile::P0);
        let mut ledger = fat_ledger();
        assert_eq!(
            negotiate(
                &required,
                &available,
                &available,
                CostPreference::LeastCost,
                true,
                &mut ledger
            )
            .unwrap_err(),
            QdnfError::UnknownProfile
        );
    }
}

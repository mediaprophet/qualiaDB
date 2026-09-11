//! E08.5 — hold-down, hysteresis, and repair. Alternating ads must not flap.

use crate::net::qdnf::errors::QdnfError;

use super::constraints::{path_feasible, PathConstraint};
use super::select::CandidatePath;

/// Replacement policy. Hold-down is in unix seconds; improve deltas are strict.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HysteresisPolicy {
    pub hold_down_unix: u64,
    /// Replacement cost must be at least this much lower.
    pub improve_cost_threshold: u16,
    /// Replacement additive latency must be at least this many ms lower.
    pub improve_latency_ms: u32,
}

impl HysteresisPolicy {
    pub const STRICT: Self = Self {
        hold_down_unix: 60,
        improve_cost_threshold: 1,
        improve_latency_ms: 1,
    };
}

fn usable(path: &CandidatePath) -> bool {
    path.hop_len > 0 && path.is_simple()
}

fn meets_improve(
    current: &CandidatePath,
    candidate: &CandidatePath,
    policy: &HysteresisPolicy,
) -> bool {
    let cost_delta = current.metrics.cost.saturating_sub(candidate.metrics.cost);
    let lat_delta = current
        .metrics
        .latency_ms
        .saturating_sub(candidate.metrics.latency_ms);
    let cost_ok = candidate.metrics.cost < current.metrics.cost
        && cost_delta >= policy.improve_cost_threshold;
    let lat_ok = candidate.metrics.latency_ms < current.metrics.latency_ms
        && lat_delta >= policy.improve_latency_ms;
    cost_ok || lat_ok
}

/// Keep `current` unless it is unusable or a replacement meets threshold after hold-down.
pub fn should_replace(
    current: &CandidatePath,
    candidate: &CandidatePath,
    policy: &HysteresisPolicy,
    now_unix: u64,
    last_change_unix: u64,
) -> bool {
    if !usable(candidate) {
        return false;
    }
    if !usable(current) {
        return true;
    }
    if current.first_hop == candidate.first_hop && current.dest == candidate.dest {
        return false;
    }
    if now_unix.saturating_sub(last_change_unix) < policy.hold_down_unix {
        return false;
    }
    meets_improve(current, candidate, policy)
}

/// Constraint-aware repair: infeasible current is replaced immediately.
pub fn decide_replace(
    current: &CandidatePath,
    candidate: &CandidatePath,
    constraint: &PathConstraint,
    policy: &HysteresisPolicy,
    now_unix: u64,
    last_change_unix: u64,
) -> bool {
    let current_ok = usable(current) && path_feasible(&current.metrics, constraint).is_ok();
    let cand_ok = usable(candidate) && path_feasible(&candidate.metrics, constraint).is_ok();
    if !current_ok {
        return cand_ok;
    }
    should_replace(current, candidate, policy, now_unix, last_change_unix)
}

/// After [`super::plan::plan_routes`], keep `current` unless repair/hysteresis switches.
pub fn hold_planned_routes(
    current: &CandidatePath,
    planned: &[CandidatePath],
    constraint: &PathConstraint,
    policy: &HysteresisPolicy,
    now_unix: u64,
    last_change_unix: u64,
    out: &mut [CandidatePath],
) -> Result<usize, QdnfError> {
    if out.is_empty() {
        return Err(QdnfError::Capacity);
    }
    let mut alt = CandidatePath::EMPTY;
    let mut have_alt = false;
    let mut i = 0usize;
    while i < planned.len() {
        let p = planned[i];
        if usable(&p)
            && (p.first_hop != current.first_hop || p.dest != current.dest)
            && path_feasible(&p.metrics, constraint).is_ok()
        {
            alt = p;
            have_alt = true;
            break;
        }
        i += 1;
    }
    let switch = if have_alt {
        decide_replace(
            current,
            &alt,
            constraint,
            policy,
            now_unix,
            last_change_unix,
        )
    } else {
        !usable(current) || path_feasible(&current.metrics, constraint).is_err()
    };
    let mut n = 0usize;
    if switch && have_alt {
        out[0] = alt;
        n = 1;
        i = 0;
        while i < planned.len() && n < out.len() {
            let p = planned[i];
            if usable(&p) && (p.first_hop != alt.first_hop || p.dest != alt.dest) {
                out[n] = p;
                n += 1;
            }
            i += 1;
        }
        return Ok(n);
    }
    if usable(current) && path_feasible(&current.metrics, constraint).is_ok() {
        out[0] = *current;
        n = 1;
        i = 0;
        while i < planned.len() && n < out.len() {
            let p = planned[i];
            if usable(&p) && (p.first_hop != current.first_hop || p.dest != current.dest) {
                out[n] = p;
                n += 1;
            }
            i += 1;
        }
        return Ok(n);
    }
    i = 0;
    while i < planned.len() && n < out.len() {
        if usable(&planned[i]) {
            out[n] = planned[i];
            n += 1;
        }
        i += 1;
    }
    if n == 0 {
        Err(QdnfError::NoRoute)
    } else {
        Ok(n)
    }
}

/// Compact next-hop that withdrew. `last_down_unix == 0` means not held.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HopHoldDown {
    pub hop: u8,
    pub last_down_unix: u64,
}

impl HopHoldDown {
    pub const EMPTY: Self = Self {
        hop: 0,
        last_down_unix: 0,
    };
}

/// True while `now` is still inside the hold-down window after withdrawal.
pub fn hop_is_held_down(entry: &HopHoldDown, now_unix: u64, policy: &HysteresisPolicy) -> bool {
    if entry.hop == 0 || entry.last_down_unix == 0 {
        return false;
    }
    now_unix.saturating_sub(entry.last_down_unix) < policy.hold_down_unix
}

fn first_hop_held(
    first_hop: u8,
    held: &[HopHoldDown],
    now_unix: u64,
    policy: &HysteresisPolicy,
) -> bool {
    let mut i = 0usize;
    while i < held.len() {
        if held[i].hop == first_hop && hop_is_held_down(&held[i], now_unix, policy) {
            return true;
        }
        i += 1;
    }
    false
}

/// Drop planned paths whose first hop is still in hold-down after a flap.
pub fn suppress_held_hops(
    planned: &[CandidatePath],
    held: &[HopHoldDown],
    policy: &HysteresisPolicy,
    now_unix: u64,
    out: &mut [CandidatePath],
) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < planned.len() && n < out.len() {
        let p = planned[i];
        if usable(&p) && !first_hop_held(p.first_hop, held, now_unix, policy) {
            out[n] = p;
            n += 1;
        }
        i += 1;
    }
    n
}

/// Partition heal: suppress held-down first hops, then apply replacement hysteresis.
///
/// A flapping hop cannot rejoin merely because `current` became unusable. After
/// the timer, [`hold_planned_routes`] may admit it again.
pub fn admit_healed_routes(
    current: &CandidatePath,
    planned: &[CandidatePath],
    held: &[HopHoldDown],
    constraint: &PathConstraint,
    policy: &HysteresisPolicy,
    now_unix: u64,
    last_change_unix: u64,
    out: &mut [CandidatePath],
) -> Result<usize, QdnfError> {
    if out.is_empty() {
        return Err(QdnfError::Capacity);
    }
    let mut filtered = [CandidatePath::EMPTY; 16];
    let n_f = suppress_held_hops(planned, held, policy, now_unix, &mut filtered);
    let effective = if first_hop_held(current.first_hop, held, now_unix, policy) {
        CandidatePath::EMPTY
    } else {
        *current
    };
    if n_f == 0 {
        if usable(&effective) && path_feasible(&effective.metrics, constraint).is_ok() {
            out[0] = effective;
            return Ok(1);
        }
        return Err(QdnfError::NoRoute);
    }
    hold_planned_routes(
        &effective,
        &filtered[..n_f],
        constraint,
        policy,
        now_unix,
        last_change_unix,
        out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::route::select::{pareto_select, PathMetrics};

    fn cand(
        first: u8,
        dest: u8,
        cost: u16,
        lat: u32,
        energy: u32,
        known: bool,
        dom: u8,
    ) -> CandidatePath {
        let mut p = CandidatePath::EMPTY;
        p.first_hop = first;
        p.dest = dest;
        p.hop_len = 1;
        p.hops[0] = first;
        p.metrics = PathMetrics {
            cost,
            latency_ms: lat,
            energy_uj: energy,
            energy_known: known,
            failure_domain: dom,
        };
        p
    }

    fn cheap() -> CandidatePath {
        cand(2, 9, 1, 100, 1, true, 1)
    }

    fn fast() -> CandidatePath {
        cand(3, 9, 100, 1, 1, true, 2)
    }

    #[test]
    fn hold_down_prevents_oscillation() {
        let policy = HysteresisPolicy::STRICT;
        let mut current = cheap();
        let mut last = 0u64;
        let mut t = 1u64;
        while t < 60 {
            let next = if t % 2 == 1 { fast() } else { cheap() };
            if should_replace(&current, &next, &policy, t, last) {
                current = next;
                last = t;
            }
            t += 1;
        }
        assert_eq!(current.first_hop, cheap().first_hop);
        assert_eq!(last, 0);
        assert!(!should_replace(&cheap(), &fast(), &policy, 59, 0));
        assert!(should_replace(&cheap(), &fast(), &policy, 60, 0));
    }

    #[test]
    fn infeasible_current_is_replaced_immediately() {
        let policy = HysteresisPolicy::STRICT;
        let current = cand(2, 9, 100, 100, 1, true, 1);
        let candidate = cand(3, 9, 10, 10, 1, true, 2);
        let constraint = PathConstraint {
            permitted_realms: u64::MAX,
            min_profile: 0,
            max_cost: 20,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 0,
        };
        assert_eq!(
            path_feasible(&current.metrics, &constraint),
            Err(QdnfError::Denied)
        );
        assert!(path_feasible(&candidate.metrics, &constraint).is_ok());
        assert!(!should_replace(&current, &candidate, &policy, 1, 0));
        assert!(decide_replace(
            &current,
            &candidate,
            &constraint,
            &policy,
            1,
            0
        ));
        let mut out = [CandidatePath::EMPTY; 3];
        let n = hold_planned_routes(&current, &[candidate], &constraint, &policy, 1, 0, &mut out)
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
    }

    #[test]
    fn distinct_failure_domain_preferred() {
        let cands = [
            cand(2, 9, 10, 40, 1, true, 1),
            cand(3, 9, 40, 10, 1, true, 1),
            cand(4, 9, 20, 30, 1, true, 2),
        ];
        let mut out = [CandidatePath::EMPTY; 2];
        let n = pareto_select(&cands, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_ne!(out[0].metrics.failure_domain, out[1].metrics.failure_domain);
    }

    #[test]
    fn held_down_first_hop_cannot_rejoin_during_timer() {
        let policy = HysteresisPolicy::STRICT;
        let current = CandidatePath::EMPTY;
        let flap = cheap();
        let held = [HopHoldDown {
            hop: flap.first_hop,
            last_down_unix: 10,
        }];
        let constraint = PathConstraint::UNRESTRICTED;
        let mut out = [CandidatePath::EMPTY; 3];
        assert!(hop_is_held_down(&held[0], 11, &policy));
        assert_eq!(
            admit_healed_routes(
                &current, &[flap], &held, &constraint, &policy, 11, 10, &mut out
            ),
            Err(QdnfError::NoRoute)
        );
        assert!(!hop_is_held_down(&held[0], 70, &policy));
        let n = admit_healed_routes(
            &current, &[flap], &held, &constraint, &policy, 70, 10, &mut out,
        )
        .unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, flap.first_hop);
    }

    #[test]
    fn alternate_not_held_is_used_while_flap_is_suppressed() {
        let policy = HysteresisPolicy::STRICT;
        let current = CandidatePath::EMPTY;
        let flap = cheap();
        let alt = fast();
        let held = [HopHoldDown {
            hop: flap.first_hop,
            last_down_unix: 1,
        }];
        let mut out = [CandidatePath::EMPTY; 3];
        let n = admit_healed_routes(
            &current,
            &[flap, alt],
            &held,
            &PathConstraint::UNRESTRICTED,
            &policy,
            2,
            1,
            &mut out,
        )
        .unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, alt.first_hop);
        assert_ne!(out[0].first_hop, flap.first_hop);
    }
}

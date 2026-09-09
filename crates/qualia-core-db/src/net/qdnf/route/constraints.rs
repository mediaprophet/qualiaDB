//! E08.3/E08.4 — hard filters before ranking. Unknown energy is not zero.

use crate::net::qdnf::errors::QdnfError;

use super::select::PathMetrics;
use super::validate::ValidatedEdge;

/// Hard path constraints. Feasibility never trades for a cheaper advertisement.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathConstraint {
    /// Bitmask of permitted realm ids 0..63. Zero means none permitted → Denied.
    pub permitted_realms: u64,
    /// 0=P0 .. 3=P3. Path/edge profile below this is Denied.
    pub min_profile: u8,
    /// Path/edge cost cap. `u16::MAX` means unlimited.
    pub max_cost: u16,
    /// Additive latency cap in milliseconds. `0` means no deadline.
    pub deadline_ms: u32,
    pub require_loop_free: bool,
    /// Energy cap in microjoules. `0` means no energy bound is required.
    pub max_energy_uj: u32,
}

impl PathConstraint {
    pub const UNRESTRICTED: Self = Self {
        permitted_realms: u64::MAX,
        min_profile: 0,
        max_cost: u16::MAX,
        deadline_ms: 0,
        require_loop_free: true,
        max_energy_uj: 0,
    };
}

/// Hard filter for a single admitted edge. Runs before any ranking.
pub fn edge_feasible(edge: &ValidatedEdge, c: &PathConstraint) -> Result<(), QdnfError> {
    if edge.withdrawn {
        return Err(QdnfError::Revoked);
    }
    if c.permitted_realms == 0 {
        return Err(QdnfError::Denied);
    }
    if edge.realm_bit >= 64 {
        return Err(QdnfError::Range);
    }
    let bit = 1u64 << edge.realm_bit;
    if c.permitted_realms & bit == 0 {
        return Err(QdnfError::Denied);
    }
    if edge.profile < c.min_profile {
        return Err(QdnfError::Denied);
    }
    if c.max_cost != u16::MAX && edge.cost > c.max_cost {
        return Err(QdnfError::Denied);
    }
    if c.deadline_ms != 0 && edge.latency_ms > c.deadline_ms {
        return Err(QdnfError::Denied);
    }
    if c.max_energy_uj != 0 {
        if !edge.energy_known {
            return Err(QdnfError::Incomplete);
        }
        if edge.energy_uj > c.max_energy_uj {
            return Err(QdnfError::Denied);
        }
    }
    Ok(())
}

/// Path-level deadline/budget check. Latency here is an additive bound, not p95.
pub fn path_feasible(metrics: &PathMetrics, c: &PathConstraint) -> Result<(), QdnfError> {
    if c.permitted_realms == 0 {
        return Err(QdnfError::Denied);
    }
    if c.max_cost != u16::MAX && metrics.cost > c.max_cost {
        return Err(QdnfError::Denied);
    }
    if c.deadline_ms != 0 && metrics.latency_ms > c.deadline_ms {
        return Err(QdnfError::Denied);
    }
    if c.max_energy_uj != 0 {
        if !metrics.energy_known {
            return Err(QdnfError::Incomplete);
        }
        if metrics.energy_uj > c.max_energy_uj {
            return Err(QdnfError::Denied);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::route::index::AdjacencyIndex;
    use crate::net::qdnf::route::plan::plan_routes;
    use crate::net::qdnf::route::select::CandidatePath;
    use crate::net::qdnf::route::validate::{insert, TopologyRecord, PROFILE_P0};
    use crate::net::qdnf::types::StrongDigest;

    fn rec(
        origin: u8,
        from: u8,
        to: u8,
        seq: u32,
        cost: u16,
        realm: u8,
        profile: u8,
    ) -> TopologyRecord {
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
            expiry_unix: 100,
            withdrawn: false,
            bidirectional: true,
            cost,
            latency_ms: 1,
            energy_uj: 1,
            energy_known: true,
            realm_bit: realm,
            profile,
            failure_domain: from,
            digest: StrongDigest::ZERO,
        }
    }

    fn edge_from(r: TopologyRecord) -> ValidatedEdge {
        ValidatedEdge {
            origin: r.origin,
            from: r.from,
            to: r.to,
            sequence: r.sequence,
            expiry_unix: r.expiry_unix,
            withdrawn: r.withdrawn,
            bidirectional: r.bidirectional,
            cost: r.cost,
            latency_ms: r.latency_ms,
            energy_uj: r.energy_uj,
            energy_known: r.energy_known,
            realm_bit: r.realm_bit,
            profile: r.profile,
            failure_domain: r.failure_domain,
            scope: r.scope,
        }
    }

    #[test]
    fn route_c_cheapest_forbidden_realm_not_selected() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 1, 1, PROFILE_P0), 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 50, 0, PROFILE_P0), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 50, 0, PROFILE_P0), 1, 1, None, &mut index).unwrap();
        let c = PathConstraint {
            permitted_realms: 1u64 << 0,
            min_profile: PROFILE_P0,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 0,
        };
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &c, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
        assert_eq!(out[0].metrics.cost, 100);
    }

    #[test]
    fn route_c_cheapest_forbidden_profile_not_selected() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 1, 0, 0), 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 9, 0, 2), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 9, 0, 2), 1, 1, None, &mut index).unwrap();
        let c = PathConstraint {
            permitted_realms: u64::MAX,
            min_profile: 2,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 0,
        };
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &c, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
        assert!(edge_feasible(&edge_from(rec(1, 1, 2, 1, 1, 0, 0)), &c).is_err());
    }

    #[test]
    fn unknown_energy_with_required_bound_is_infeasible() {
        let mut e = edge_from(rec(1, 1, 2, 1, 1, 0, 0));
        e.energy_known = false;
        e.energy_uj = 0;
        let c = PathConstraint {
            permitted_realms: u64::MAX,
            min_profile: 0,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 10,
        };
        assert_eq!(edge_feasible(&e, &c), Err(QdnfError::Incomplete));
        e.energy_known = true;
        e.energy_uj = 0;
        assert!(edge_feasible(&e, &c).is_ok());
    }

    #[test]
    fn malicious_zero_cost_still_fails_if_realm_forbidden() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 0, 5, PROFILE_P0), 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 40, 0, PROFILE_P0), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 40, 0, PROFILE_P0), 1, 1, None, &mut index).unwrap();
        let c = PathConstraint {
            permitted_realms: 1u64 << 0,
            min_profile: PROFILE_P0,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 0,
        };
        let cheap = edge_from(rec(1, 1, 2, 1, 0, 5, PROFILE_P0));
        assert_eq!(edge_feasible(&cheap, &c), Err(QdnfError::Denied));
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &c, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
        assert_ne!(out[0].metrics.cost, 0);
    }

    #[test]
    fn zero_permitted_realms_is_denied() {
        let e = edge_from(rec(1, 1, 2, 1, 1, 0, 0));
        let mut c = PathConstraint::UNRESTRICTED;
        c.permitted_realms = 0;
        assert_eq!(edge_feasible(&e, &c), Err(QdnfError::Denied));
    }
}

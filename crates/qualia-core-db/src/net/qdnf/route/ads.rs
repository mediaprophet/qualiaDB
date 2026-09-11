//! E08.6 — verified topology facts vs untrusted price/energy/latency adverts.

use crate::net::qdnf::errors::QdnfError;

use super::constraints::{edge_feasible, path_feasible, PathConstraint};
use super::select::{CandidatePath, PathMetrics, MAX_PARETO_IN};
use super::validate::ValidatedEdge;

/// At most this many untrusted adverts are explored, and only among feasible paths.
pub const MAX_EXPLORE: usize = 4;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricProvenance {
    VerifiedTopology = 0,
    UntrustedAdvert = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdvertisedMetric {
    pub provenance: MetricProvenance,
    pub cost: u16,
    pub latency_ms: u32,
    pub energy_uj: u32,
    pub energy_known: bool,
}

/// Advert bound to a planned first hop. Untrusted copies never admit a new path.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathAdvert {
    pub first_hop: u8,
    pub dest: u8,
    pub metric: AdvertisedMetric,
}

fn overlay_energy(verified: &PathMetrics, advert: &AdvertisedMetric) -> (u32, bool) {
    if advert.energy_known {
        (advert.energy_uj, true)
    } else if verified.energy_known {
        (verified.energy_uj, true)
    } else {
        (0, false)
    }
}

/// Overlay advert metrics. Unknown energy stays unknown, never a free zero.
pub fn overlay_metrics(verified: &PathMetrics, advert: &AdvertisedMetric) -> PathMetrics {
    let (energy_uj, energy_known) = overlay_energy(verified, advert);
    PathMetrics {
        cost: advert.cost,
        latency_ms: advert.latency_ms,
        energy_uj,
        energy_known,
        failure_domain: verified.failure_domain,
    }
}

fn apply_metric_fields(edge: &ValidatedEdge, advert: &AdvertisedMetric) -> ValidatedEdge {
    let verified = PathMetrics {
        cost: edge.cost,
        latency_ms: edge.latency_ms,
        energy_uj: edge.energy_uj,
        energy_known: edge.energy_known,
        failure_domain: edge.failure_domain,
    };
    let m = overlay_metrics(&verified, advert);
    let mut e = *edge;
    e.cost = m.cost;
    e.latency_ms = m.latency_ms;
    e.energy_uj = m.energy_uj;
    e.energy_known = m.energy_known;
    e
}

/// Untrusted ads cannot make an infeasible verified edge feasible.
pub fn admit_advert(
    edge: &ValidatedEdge,
    advert: &AdvertisedMetric,
    constraint: &PathConstraint,
) -> Result<ValidatedEdge, QdnfError> {
    match advert.provenance {
        MetricProvenance::VerifiedTopology => {
            let e = apply_metric_fields(edge, advert);
            edge_feasible(&e, constraint)?;
            Ok(e)
        }
        MetricProvenance::UntrustedAdvert => {
            edge_feasible(edge, constraint)?;
            Ok(apply_metric_fields(edge, advert))
        }
    }
}

fn matches(path: &CandidatePath, advert: &PathAdvert) -> bool {
    path.first_hop == advert.first_hop && path.dest == advert.dest
}

fn apply_to_path(path: &mut CandidatePath, advert: &PathAdvert) {
    path.metrics = overlay_metrics(&path.metrics, &advert.metric);
}

/// Among already-feasible paths only, consider at most [`MAX_EXPLORE`] untrusted ads.
pub fn explore_untrusted(
    feasible: &[CandidatePath],
    adverts: &[PathAdvert],
    constraint: &PathConstraint,
    out: &mut [CandidatePath],
) -> Result<usize, QdnfError> {
    let n_in = feasible.len();
    if n_in > MAX_PARETO_IN {
        return Err(QdnfError::Capacity);
    }
    if out.is_empty() && n_in > 0 {
        return Err(QdnfError::Capacity);
    }
    let mut tmp = [CandidatePath::EMPTY; MAX_PARETO_IN];
    let mut n = 0usize;
    let mut src = 0usize;
    while src < n_in {
        if path_feasible(&feasible[src].metrics, constraint).is_ok() {
            tmp[n] = feasible[src];
            n += 1;
        }
        src += 1;
    }
    let mut explored = 0usize;
    let mut a = 0usize;
    while a < adverts.len() {
        let adv = adverts[a];
        a += 1;
        let mut hit = None;
        let mut i = 0usize;
        while i < n {
            if matches(&tmp[i], &adv) {
                hit = Some(i);
                break;
            }
            i += 1;
        }
        let idx = match hit {
            Some(i) => i,
            None => continue,
        };
        match adv.metric.provenance {
            MetricProvenance::VerifiedTopology => apply_to_path(&mut tmp[idx], &adv),
            MetricProvenance::UntrustedAdvert => {
                if explored >= MAX_EXPLORE {
                    continue;
                }
                apply_to_path(&mut tmp[idx], &adv);
                explored += 1;
            }
        }
    }
    let copy = n.min(out.len());
    let mut i = 0usize;
    while i < copy {
        out[i] = tmp[i];
        i += 1;
    }
    Ok(copy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::route::index::AdjacencyIndex;
    use crate::net::qdnf::route::plan::plan_routes;
    use crate::net::qdnf::route::validate::{insert, TopologyRecord, PROFILE_P0};
    use crate::net::qdnf::types::StrongDigest;

    fn rec(origin: u8, from: u8, to: u8, seq: u32, cost: u16, realm: u8) -> TopologyRecord {
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
            profile: PROFILE_P0,
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

    fn path(first: u8, dest: u8, cost: u16, lat: u32) -> CandidatePath {
        let mut p = CandidatePath::EMPTY;
        p.first_hop = first;
        p.dest = dest;
        p.hop_len = 1;
        p.hops[0] = first;
        p.metrics = PathMetrics {
            cost,
            latency_ms: lat,
            energy_uj: 1,
            energy_known: true,
            failure_domain: first,
        };
        p
    }

    #[test]
    fn untrusted_zero_cost_cannot_bypass_forbidden_realm() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 0, 5), 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 40, 0), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 40, 0), 1, 1, None, &mut index).unwrap();
        let c = PathConstraint {
            permitted_realms: 1u64 << 0,
            min_profile: PROFILE_P0,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 0,
        };
        let cheap = edge_from(rec(1, 1, 2, 1, 0, 5));
        let advert = AdvertisedMetric {
            provenance: MetricProvenance::UntrustedAdvert,
            cost: 0,
            latency_ms: 0,
            energy_uj: 0,
            energy_known: true,
        };
        assert_eq!(admit_advert(&cheap, &advert, &c), Err(QdnfError::Denied));
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &c, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
        assert_ne!(out[0].metrics.cost, 0);
    }

    #[test]
    fn untrusted_cannot_make_infeasible_cost_feasible() {
        let edge = edge_from(rec(1, 1, 2, 1, 80, 0));
        let c = PathConstraint {
            permitted_realms: u64::MAX,
            min_profile: 0,
            max_cost: 10,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 0,
        };
        assert_eq!(edge_feasible(&edge, &c), Err(QdnfError::Denied));
        let advert = AdvertisedMetric {
            provenance: MetricProvenance::UntrustedAdvert,
            cost: 0,
            latency_ms: 1,
            energy_uj: 1,
            energy_known: true,
        };
        assert_eq!(admit_advert(&edge, &advert, &c), Err(QdnfError::Denied));
    }

    #[test]
    fn unknown_energy_is_not_zero() {
        let verified = PathMetrics {
            cost: 4,
            latency_ms: 4,
            energy_uj: 0,
            energy_known: false,
            failure_domain: 1,
        };
        let advert = AdvertisedMetric {
            provenance: MetricProvenance::UntrustedAdvert,
            cost: 1,
            latency_ms: 1,
            energy_uj: 0,
            energy_known: false,
        };
        let over = overlay_metrics(&verified, &advert);
        assert!(!over.energy_known);
        assert_eq!(over.energy_uj, 0);
        let c = PathConstraint {
            permitted_realms: u64::MAX,
            min_profile: 0,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 10,
        };
        assert_eq!(path_feasible(&over, &c), Err(QdnfError::Incomplete));
    }

    #[test]
    fn explore_caps_untrusted_at_max_explore() {
        let mut feasible = [CandidatePath::EMPTY; 5];
        let mut adverts = [PathAdvert {
            first_hop: 0,
            dest: 9,
            metric: AdvertisedMetric {
                provenance: MetricProvenance::UntrustedAdvert,
                cost: 0,
                latency_ms: 0,
                energy_uj: 1,
                energy_known: true,
            },
        }; 5];
        let mut i = 0u8;
        while i < 5 {
            feasible[i as usize] = path(i + 1, 9, 10, 10);
            adverts[i as usize].first_hop = i + 1;
            adverts[i as usize].metric.cost = i as u16;
            i += 1;
        }
        let mut out = [CandidatePath::EMPTY; 5];
        let n = explore_untrusted(&feasible, &adverts, &PathConstraint::UNRESTRICTED, &mut out)
            .unwrap();
        assert_eq!(n, 5);
        assert_eq!(out[0].metrics.cost, 0);
        assert_eq!(out[3].metrics.cost, 3);
        assert_eq!(out[4].metrics.cost, 10);
    }
}

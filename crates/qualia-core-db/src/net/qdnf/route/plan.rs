//! Production route planner. Feasibility first; never auto-publishes forwarding.

use crate::net::qdnf::errors::QdnfError;

use super::constraints::{edge_feasible, path_feasible, PathConstraint};
use super::index::{AdjacencyIndex, MAX_ADMITTED_EDGES};
use super::select::{pareto_select, CandidatePath, PathMetrics, MAX_PARETO_IN, MAX_PATH_HOPS};
use super::validate::ValidatedEdge;

const MAX_NODES: usize = 256;

/// Plan at most three feasible next hops from `origin` to `dest`.
///
/// Does not publish [`super::forwarding::ForwardingGeneration`]. Callers publish
/// only after this returns successfully.
pub fn plan_routes(
    index: &AdjacencyIndex,
    origin: u8,
    dest: u8,
    constraint: &PathConstraint,
    now_unix: u64,
    out: &mut [CandidatePath],
) -> Result<usize, QdnfError> {
    if out.is_empty() {
        return Err(QdnfError::Capacity);
    }
    if origin == 0 || dest == 0 {
        return Err(QdnfError::Range);
    }
    if origin == dest {
        out[0] = CandidatePath::EMPTY;
        out[0].dest = dest;
        out[0].first_hop = origin;
        return Ok(1);
    }

    let mut live = [ValidatedEdge::EMPTY; MAX_ADMITTED_EDGES];
    let n_live = index.live_edges(now_unix, &mut live);
    if n_live == 0 {
        return Err(QdnfError::NoRoute);
    }

    let mut topo_reach = false;
    let mut gen = [CandidatePath::EMPTY; MAX_PARETO_IN];
    let mut n_gen = 0usize;

    let mut neigh = 1u8;
    loop {
        if n_gen >= MAX_PARETO_IN {
            break;
        }
        if let Some(edge) = directed_edge(&live[..n_live], origin, neigh) {
            match first_hop_path(
                &live[..n_live],
                origin,
                neigh,
                dest,
                edge,
                constraint,
                now_unix,
            ) {
                Ok(path) => {
                    topo_reach = true;
                    gen[n_gen] = path;
                    n_gen += 1;
                }
                Err(QdnfError::NoRoute) => {}
                Err(QdnfError::Denied) | Err(QdnfError::Incomplete) => {
                    if reachable(&live[..n_live], origin, neigh, dest, now_unix) {
                        topo_reach = true;
                    }
                }
                Err(e) => return Err(e),
            }
        }
        if neigh == 255 {
            break;
        }
        neigh += 1;
    }

    if n_gen == 0 {
        return Err(if topo_reach {
            QdnfError::Denied
        } else {
            QdnfError::NoRoute
        });
    }

    let n = pareto_select(&gen[..n_gen], out)?;
    if n == 0 {
        if topo_reach {
            Err(QdnfError::Denied)
        } else {
            Err(QdnfError::NoRoute)
        }
    } else {
        Ok(n)
    }
}

fn directed_edge(live: &[ValidatedEdge], from: u8, to: u8) -> Option<ValidatedEdge> {
    let mut i = 0usize;
    while i < live.len() {
        let e = live[i];
        if e.from == from && e.to == to {
            return Some(e);
        }
        if e.bidirectional && e.to == from && e.from == to {
            return Some(e);
        }
        i += 1;
    }
    None
}

fn first_hop_path(
    live: &[ValidatedEdge],
    origin: u8,
    hop: u8,
    dest: u8,
    first: ValidatedEdge,
    constraint: &PathConstraint,
    now_unix: u64,
) -> Result<CandidatePath, QdnfError> {
    if now_unix >= first.expiry_unix || first.withdrawn {
        return Err(QdnfError::NoRoute);
    }
    match edge_feasible(&first, constraint) {
        Ok(()) => {}
        Err(QdnfError::Denied) | Err(QdnfError::Incomplete) | Err(QdnfError::Revoked) => {
            return Err(QdnfError::Denied);
        }
        Err(e) => return Err(e),
    }
    let mut path = CandidatePath::EMPTY;
    path.dest = dest;
    path.first_hop = hop;
    path.hops[0] = hop;
    path.hop_len = 1;
    path.metrics = PathMetrics {
        cost: first.cost,
        latency_ms: first.latency_ms,
        energy_uj: first.energy_uj,
        energy_known: first.energy_known,
        failure_domain: first.failure_domain,
    };
    if hop == dest {
        path_feasible(&path.metrics, constraint).map_err(map_budget)?;
        return Ok(path);
    }
    dijkstra_continue(live, origin, hop, dest, constraint, now_unix, &mut path)?;
    path_feasible(&path.metrics, constraint).map_err(map_budget)?;
    if constraint.require_loop_free && !path.is_simple() {
        return Err(QdnfError::Denied);
    }
    Ok(path)
}

fn map_budget(err: QdnfError) -> QdnfError {
    match err {
        QdnfError::Incomplete => QdnfError::Incomplete,
        _ => QdnfError::Denied,
    }
}

fn dijkstra_continue(
    live: &[ValidatedEdge],
    origin: u8,
    start: u8,
    dest: u8,
    constraint: &PathConstraint,
    now_unix: u64,
    path: &mut CandidatePath,
) -> Result<(), QdnfError> {
    let mut dist = [u16::MAX; MAX_NODES];
    let mut lat = [u32::MAX; MAX_NODES];
    let mut energy = [u32::MAX; MAX_NODES];
    let mut known = [false; MAX_NODES];
    let mut prev = [u8::MAX; MAX_NODES];
    let mut used = [false; MAX_NODES];
    dist[start as usize] = 0;
    lat[start as usize] = 0;
    energy[start as usize] = 0;
    known[start as usize] = true;

    let mut step = 0usize;
    while step < MAX_NODES {
        let mut best = u16::MAX;
        let mut u = usize::MAX;
        let mut i = 0usize;
        while i < MAX_NODES {
            if !used[i] && dist[i] < best {
                best = dist[i];
                u = i;
            } else if !used[i] && dist[i] == best && u != usize::MAX && i < u {
                u = i;
            }
            i += 1;
        }
        if u == usize::MAX {
            break;
        }
        used[u] = true;
        if u as u8 == dest {
            break;
        }
        let mut e = 0usize;
        while e < live.len() {
            let edge = live[e];
            e += 1;
            if now_unix >= edge.expiry_unix || edge.withdrawn {
                continue;
            }
            let (from, to) = match directed_pair(&edge, u as u8) {
                Some(p) => p,
                None => continue,
            };
            if from != u as u8 {
                continue;
            }
            if to == origin || to == start {
                continue;
            }
            if edge_feasible(&edge, constraint).is_err() {
                continue;
            }
            let v = to as usize;
            let cand = match dist[u].checked_add(edge.cost) {
                Some(c) => c,
                None => continue,
            };
            let lat_c = match lat[u].checked_add(edge.latency_ms) {
                Some(c) => c,
                None => continue,
            };
            let e_known = known[u] && edge.energy_known;
            let e_c = if e_known {
                match energy[u].checked_add(edge.energy_uj) {
                    Some(c) => c,
                    None => continue,
                }
            } else {
                energy[u]
            };
            if cand < dist[v] || (cand == dist[v] && (u as u8) < prev[v]) {
                dist[v] = cand;
                lat[v] = lat_c;
                energy[v] = e_c;
                known[v] = e_known;
                prev[v] = u as u8;
            }
        }
        step += 1;
    }
    if dist[dest as usize] == u16::MAX {
        return Err(QdnfError::NoRoute);
    }
    let mut stack = [0u8; MAX_PATH_HOPS];
    let mut sn = 0usize;
    let mut cur = dest;
    let mut guard = 0usize;
    while cur != start && guard < MAX_PATH_HOPS {
        if sn >= MAX_PATH_HOPS {
            return Err(QdnfError::Capacity);
        }
        stack[sn] = cur;
        sn += 1;
        let p = prev[cur as usize];
        if p == u8::MAX {
            return Err(QdnfError::NoRoute);
        }
        cur = p;
        guard += 1;
    }
    if cur != start {
        return Err(QdnfError::NoRoute);
    }
    let mut hop_len = 1u8;
    let mut k = sn;
    while k > 0 {
        k -= 1;
        if hop_len as usize >= MAX_PATH_HOPS {
            return Err(QdnfError::Capacity);
        }
        path.hops[hop_len as usize] = stack[k];
        hop_len += 1;
    }
    path.hop_len = hop_len;
    let base = path.metrics;
    path.metrics.cost = match base.cost.checked_add(dist[dest as usize]) {
        Some(c) => c,
        None => return Err(QdnfError::Range),
    };
    path.metrics.latency_ms = match base.latency_ms.checked_add(lat[dest as usize]) {
        Some(c) => c,
        None => return Err(QdnfError::Range),
    };
    path.metrics.energy_known = base.energy_known && known[dest as usize];
    if path.metrics.energy_known {
        path.metrics.energy_uj = match base.energy_uj.checked_add(energy[dest as usize]) {
            Some(c) => c,
            None => return Err(QdnfError::Range),
        };
    } else {
        path.metrics.energy_uj = 0;
    }
    Ok(())
}

fn directed_pair(edge: &ValidatedEdge, u: u8) -> Option<(u8, u8)> {
    if edge.from == u {
        Some((edge.from, edge.to))
    } else if edge.bidirectional && edge.to == u {
        Some((edge.to, edge.from))
    } else {
        None
    }
}

fn reachable(live: &[ValidatedEdge], origin: u8, start: u8, dest: u8, now_unix: u64) -> bool {
    if start == dest {
        return true;
    }
    let mut seen = [false; MAX_NODES];
    let mut stack = [0u8; MAX_NODES];
    let mut sp = 1usize;
    stack[0] = start;
    seen[start as usize] = true;
    seen[origin as usize] = true;
    while sp > 0 {
        sp -= 1;
        let u = stack[sp];
        let mut i = 0usize;
        while i < live.len() {
            let e = live[i];
            i += 1;
            if now_unix >= e.expiry_unix || e.withdrawn {
                continue;
            }
            let to = if e.from == u {
                e.to
            } else if e.bidirectional && e.to == u {
                e.from
            } else {
                continue;
            };
            if to == dest {
                return true;
            }
            if !seen[to as usize] {
                seen[to as usize] = true;
                if sp < MAX_NODES {
                    stack[sp] = to;
                    sp += 1;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::route::index::AdjacencyIndex;
    use crate::net::qdnf::route::validate::{insert, TopologyRecord, PROFILE_P0};
    use crate::net::qdnf::types::StrongDigest;

    fn rec(origin: u8, from: u8, to: u8, seq: u32, cost: u16) -> TopologyRecord {
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
            realm_bit: 0,
            profile: PROFILE_P0,
            failure_domain: from,
            digest: StrongDigest::ZERO,
        }
    }

    #[test]
    fn route_b_disconnected_dest_is_noroute() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 1), 1, 1, None, &mut index).unwrap();
        let mut out = [CandidatePath::EMPTY; 3];
        assert_eq!(
            plan_routes(&index, 1, 5, &PathConstraint::UNRESTRICTED, 1, &mut out),
            Err(QdnfError::NoRoute)
        );
    }

    #[test]
    fn route_b_equal_costs_are_deterministic() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 2), 1, 1, None, &mut index).unwrap();
        insert(&rec(2, 2, 4, 1, 2), 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 2), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 4, 1, 2), 1, 1, None, &mut index).unwrap();
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 4, &PathConstraint::UNRESTRICTED, 1, &mut out).unwrap();
        assert!(n >= 1);
        assert_eq!(out[0].first_hop, 2);
        assert_eq!(out[0].metrics.cost, 4);
    }

    #[test]
    fn expired_edge_is_not_used() {
        let mut index = AdjacencyIndex::new(1);
        let mut stale = rec(1, 1, 2, 1, 1);
        stale.expiry_unix = 5;
        insert(&stale, 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 9), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 9), 1, 1, None, &mut index).unwrap();
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &PathConstraint::UNRESTRICTED, 5, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
    }

    #[test]
    fn withdrawn_edge_is_not_used() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 1), 1, 1, None, &mut index).unwrap();
        let mut w = rec(1, 1, 2, 2, 1);
        w.withdrawn = true;
        insert(&w, 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 3, 8), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 8), 1, 1, None, &mut index).unwrap();
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &PathConstraint::UNRESTRICTED, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
        assert_eq!(index.live_count(1), 2);
    }

    #[test]
    fn unknown_energy_required_bound_not_selected() {
        let mut index = AdjacencyIndex::new(1);
        let mut unknown = rec(1, 1, 2, 1, 1);
        unknown.energy_known = false;
        unknown.energy_uj = 0;
        insert(&unknown, 1, 1, None, &mut index).unwrap();
        insert(&rec(1, 1, 3, 2, 20), 1, 1, None, &mut index).unwrap();
        insert(&rec(3, 3, 2, 1, 20), 1, 1, None, &mut index).unwrap();
        let c = PathConstraint {
            permitted_realms: u64::MAX,
            min_profile: 0,
            max_cost: u16::MAX,
            deadline_ms: 0,
            require_loop_free: true,
            max_energy_uj: 100,
        };
        let mut out = [CandidatePath::EMPTY; 3];
        let n = plan_routes(&index, 1, 2, &c, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 3);
        assert!(out[0].metrics.energy_known);
    }

    #[test]
    fn plan_does_not_publish_forwarding() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1, 1), 1, 1, None, &mut index).unwrap();
        let mut out = [CandidatePath::EMPTY; 3];
        plan_routes(&index, 1, 2, &PathConstraint::UNRESTRICTED, 1, &mut out).unwrap();
        assert_eq!(index.published().lookup_next(2), Err(QdnfError::Incomplete));
    }
}

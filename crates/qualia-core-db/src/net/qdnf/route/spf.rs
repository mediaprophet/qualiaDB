//! Bounded SPF over authenticated adjacencies. At most three next hops.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::DniCoordinate;

pub const MAX_NODES: usize = 16;
pub const MAX_NEXT_HOPS: usize = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkMetric {
    pub from: u8,
    pub to: u8,
    pub cost: u16,
    pub bidirectional: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NextHop {
    pub node: u8,
    pub cost: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpfTable {
    pub dest: [u8; MAX_NODES],
    pub hops: [[NextHop; MAX_NEXT_HOPS]; MAX_NODES],
    pub hop_counts: [u8; MAX_NODES],
    pub nodes: usize,
    pub origin: u8,
    /// Bellman-Ford / Dijkstra dual shortest-path distance potentials `d(u)` (§6.2.4).
    pub potentials: [u16; MAX_NODES],
}

impl SpfTable {
    pub const EMPTY: Self = Self {
        dest: [0; MAX_NODES],
        hops: [[NextHop { node: 0, cost: 0 }; MAX_NEXT_HOPS]; MAX_NODES],
        hop_counts: [0; MAX_NODES],
        nodes: 0,
        origin: 0,
        potentials: [u16::MAX; MAX_NODES],
    };

    /// Return the Bellman-Ford dual shortest-path optimality witness potentials `d(u)`.
    #[inline]
    pub const fn dual_witness(&self) -> SpfDualWitness {
        SpfDualWitness {
            origin: self.origin,
            potentials: self.potentials,
        }
    }
}

/// Bellman-Ford dual distance potential witness `d(u)` (§6.2.4).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpfDualWitness {
    pub origin: u8,
    pub potentials: [u16; MAX_NODES],
}

/// Deterministic Dijkstra with stable node-id tie-break. One-sided links are ignored.
pub fn compute_spf(origin: u8, links: &[LinkMetric], out: &mut SpfTable) -> Result<(), QdnfError> {
    *out = SpfTable::EMPTY;
    let mut dist = [u16::MAX; MAX_NODES];
    let mut prev = [u8::MAX; MAX_NODES];
    let mut used = [false; MAX_NODES];
    if (origin as usize) >= MAX_NODES {
        return Err(QdnfError::Range);
    }
    dist[origin as usize] = 0;
    for _ in 0..MAX_NODES {
        let mut best = u16::MAX;
        let mut u = usize::MAX;
        for (i, (d, taken)) in dist.iter().zip(used.iter()).enumerate() {
            if !taken && *d < best {
                best = *d;
                u = i;
            } else if !taken && *d == best && u != usize::MAX && i < u {
                u = i;
            }
        }
        if u == usize::MAX {
            break;
        }
        used[u] = true;
        for link in links {
            if !link.bidirectional {
                continue;
            }
            if link.from as usize != u && link.to as usize != u {
                continue;
            }
            let v = if link.from as usize == u {
                link.to as usize
            } else {
                link.from as usize
            };
            if v >= MAX_NODES {
                return Err(QdnfError::Range);
            }
            let cand = match dist[u].checked_add(link.cost) {
                Some(c) => c,
                None => return Err(QdnfError::Range),
            };
            if cand < dist[v] || (cand == dist[v] && (u as u8) < prev[v]) {
                dist[v] = cand;
                prev[v] = u as u8;
            }
        }
    }
    let mut n = 0usize;
    for (i, d) in dist.iter().enumerate() {
        if *d == u16::MAX {
            continue;
        }
        out.dest[n] = i as u8;
        if i as u8 == origin {
            out.hop_counts[n] = 0;
        } else {
            let mut hop = i as u8;
            let mut guard = 0;
            while prev[hop as usize] != origin && prev[hop as usize] != u8::MAX && guard < MAX_NODES
            {
                hop = prev[hop as usize];
                guard += 1;
            }
            out.hops[n][0] = NextHop {
                node: hop,
                cost: *d,
            };
            out.hop_counts[n] = 1;
        }
        n += 1;
    }
    out.nodes = n;
    out.origin = origin;
    out.potentials = dist;
    Ok(())
}

/// Verify a Bellman-Ford dual shortest-path optimality witness in `O(|E|)` (§6.2.4):
/// 1. `d(src) == 0`;
/// 2. Dual feasibility `d(v) <= d(u) + w(u, v)` on every eligible bidirectional snapshot edge;
/// 3. Tightness `d(v) == d(u) + w(u, v)` along every consecutive hop of `selected_path`.
pub fn verify_spf_dual_witness(
    origin: u8,
    dest: u8,
    links: &[LinkMetric],
    potentials: &[u16; MAX_NODES],
    selected_path: &[u8],
) -> Result<(), QdnfError> {
    if (origin as usize) >= MAX_NODES || (dest as usize) >= MAX_NODES {
        return Err(QdnfError::Range);
    }
    if selected_path.is_empty()
        || selected_path[0] != origin
        || selected_path[selected_path.len() - 1] != dest
    {
        return Err(QdnfError::Malformed);
    }
    if potentials[origin as usize] != 0 {
        return Err(QdnfError::Conflict);
    }
    // 1. Check dual feasibility d(v) <= d(u) + w(u, v) across all bidirectional edges in O(|E|)
    let mut e = 0usize;
    while e < links.len() {
        let link = links[e];
        if link.bidirectional {
            let u = link.from as usize;
            let v = link.to as usize;
            if u >= MAX_NODES || v >= MAX_NODES {
                return Err(QdnfError::Range);
            }
            if potentials[u] != u16::MAX {
                let bound = potentials[u]
                    .checked_add(link.cost)
                    .ok_or(QdnfError::Range)?;
                if potentials[v] > bound {
                    return Err(QdnfError::Conflict);
                }
            }
            if potentials[v] != u16::MAX {
                let bound = potentials[v]
                    .checked_add(link.cost)
                    .ok_or(QdnfError::Range)?;
                if potentials[u] > bound {
                    return Err(QdnfError::Conflict);
                }
            }
        }
        e += 1;
    }
    // 2. Check tight equality d(v) == d(u) + w(u, v) along the selected path
    let mut k = 0usize;
    while k + 1 < selected_path.len() {
        let u = selected_path[k];
        let v = selected_path[k + 1];
        if (u as usize) >= MAX_NODES || (v as usize) >= MAX_NODES {
            return Err(QdnfError::Range);
        }
        let mut min_w = u16::MAX;
        let mut found = false;
        let mut j = 0usize;
        while j < links.len() {
            let link = links[j];
            if link.bidirectional
                && ((link.from == u && link.to == v) || (link.from == v && link.to == u))
                && (!found || link.cost < min_w)
            {
                min_w = link.cost;
                found = true;
            }
            j += 1;
        }
        if !found {
            return Err(QdnfError::NoRoute);
        }
        let du = potentials[u as usize];
        let dv = potentials[v as usize];
        if du == u16::MAX || dv == u16::MAX {
            return Err(QdnfError::Conflict);
        }
        let expected_dv = du.checked_add(min_w).ok_or(QdnfError::Range)?;
        if dv != expected_dv {
            return Err(QdnfError::Conflict);
        }
        k += 1;
    }
    Ok(())
}

/// DNI node marker.
pub fn dni_node(coord: DniCoordinate) -> u8 {
    (coord.node_id & 0xff) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_node_path_a_b_c() {
        let links = [
            LinkMetric {
                from: 0,
                to: 1,
                cost: 1,
                bidirectional: true,
            },
            LinkMetric {
                from: 1,
                to: 2,
                cost: 1,
                bidirectional: true,
            },
        ];
        let mut table = SpfTable::EMPTY;
        compute_spf(0, &links, &mut table).unwrap();
        let dest_c = table.dest.iter().position(|d| *d == 2).unwrap();
        assert_eq!(out_hop(&table, dest_c), 1);
    }

    fn out_hop(table: &SpfTable, idx: usize) -> u8 {
        table.hops[idx][0].node
    }

    #[test]
    fn one_sided_link_is_ignored() {
        let links = [LinkMetric {
            from: 0,
            to: 1,
            cost: 1,
            bidirectional: false,
        }];
        let mut table = SpfTable::EMPTY;
        compute_spf(0, &links, &mut table).unwrap();
        assert!(table.dest.iter().take(table.nodes).all(|d| *d == 0));
    }

    #[test]
    fn bellman_ford_dual_witness_verifies_optimal_path_and_rejects_suboptimal() {
        let links = [
            LinkMetric {
                from: 0,
                to: 1,
                cost: 2,
                bidirectional: true,
            },
            LinkMetric {
                from: 1,
                to: 2,
                cost: 3,
                bidirectional: true,
            },
            LinkMetric {
                from: 0,
                to: 2,
                cost: 9,
                bidirectional: true,
            },
        ];
        let mut table = SpfTable::EMPTY;
        compute_spf(0, &links, &mut table).unwrap();
        let w = table.dual_witness();
        assert_eq!(w.potentials[0], 0);
        assert_eq!(w.potentials[1], 2);
        assert_eq!(w.potentials[2], 5);

        // Optimal path 0 -> 1 -> 2 satisfies d(v) == d(u) + w(u,v)
        assert_eq!(
            verify_spf_dual_witness(0, 2, &links, &w.potentials, &[0, 1, 2]),
            Ok(())
        );
        // Suboptimal direct edge 0 -> 2 (cost 9 != 5) is rejected by tightness check
        assert_eq!(
            verify_spf_dual_witness(0, 2, &links, &w.potentials, &[0, 2]),
            Err(QdnfError::Conflict)
        );
        // Dishonest potential (claiming d(2) = 9 so 0 -> 2 looks tight) violates dual feasibility on edge (1, 2): 9 > 2 + 3
        let mut dishonest = w.potentials;
        dishonest[2] = 9;
        assert_eq!(
            verify_spf_dual_witness(0, 2, &links, &dishonest, &[0, 2]),
            Err(QdnfError::Conflict)
        );
    }
}

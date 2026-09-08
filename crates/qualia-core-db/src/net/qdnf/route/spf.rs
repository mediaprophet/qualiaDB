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
}

impl SpfTable {
    pub const EMPTY: Self = Self {
        dest: [0; MAX_NODES],
        hops: [[NextHop { node: 0, cost: 0 }; MAX_NEXT_HOPS]; MAX_NODES],
        hop_counts: [0; MAX_NODES],
        nodes: 0,
    };
}

/// Deterministic Dijkstra with stable node-id tie-break. One-sided links are ignored.
pub fn compute_spf(
    origin: u8,
    links: &[LinkMetric],
    out: &mut SpfTable,
) -> Result<(), QdnfError> {
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
    Ok(())
}

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
}

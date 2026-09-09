//! Pareto selection of at most three next hops. Metrics stay independently typed.

use crate::net::qdnf::errors::QdnfError;

pub const MAX_PATH_HOPS: usize = 16;
pub const MAX_SELECTED: usize = 3;
pub const MAX_PARETO_IN: usize = 16;

/// Independently typed path metrics. Do not collapse these into one scalar.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathMetrics {
    pub cost: u16,
    /// Additive latency bound. Not an end-to-end p95.
    pub latency_ms: u32,
    /// `0` with `energy_known == false` is unknown, not free.
    pub energy_uj: u32,
    pub energy_known: bool,
    pub failure_domain: u8,
}

impl PathMetrics {
    pub const EMPTY: Self = Self {
        cost: 0,
        latency_ms: 0,
        energy_uj: 0,
        energy_known: true,
        failure_domain: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidatePath {
    pub dest: u8,
    pub first_hop: u8,
    pub hops: [u8; MAX_PATH_HOPS],
    pub hop_len: u8,
    pub metrics: PathMetrics,
}

impl CandidatePath {
    pub const EMPTY: Self = Self {
        dest: 0,
        first_hop: 0,
        hops: [0; MAX_PATH_HOPS],
        hop_len: 0,
        metrics: PathMetrics::EMPTY,
    };

    pub fn is_simple(&self) -> bool {
        let n = self.hop_len as usize;
        if n > MAX_PATH_HOPS {
            return false;
        }
        let mut i = 0usize;
        while i < n {
            let mut j = i + 1;
            while j < n {
                if self.hops[i] == self.hops[j] {
                    return false;
                }
                j += 1;
            }
            i += 1;
        }
        true
    }
}

fn dominates(a: &CandidatePath, b: &CandidatePath) -> bool {
    let mut strict = false;
    if a.metrics.cost > b.metrics.cost {
        return false;
    }
    if a.metrics.cost < b.metrics.cost {
        strict = true;
    }
    if a.metrics.latency_ms > b.metrics.latency_ms {
        return false;
    }
    if a.metrics.latency_ms < b.metrics.latency_ms {
        strict = true;
    }
    if a.metrics.energy_known && b.metrics.energy_known {
        if a.metrics.energy_uj > b.metrics.energy_uj {
            return false;
        }
        if a.metrics.energy_uj < b.metrics.energy_uj {
            strict = true;
        }
    }
    strict
}

fn better_tie(a: &CandidatePath, b: &CandidatePath) -> bool {
    if a.first_hop != b.first_hop {
        return a.first_hop < b.first_hop;
    }
    a.dest < b.dest
}

/// Remove B only when A is no worse on every comparable metric and strictly better
/// on at least one. At most three results; prefer distinct failure domains.
pub fn pareto_select(
    candidates: &[CandidatePath],
    out: &mut [CandidatePath],
) -> Result<usize, QdnfError> {
    if out.is_empty() {
        return Err(QdnfError::Capacity);
    }
    let n_in = candidates.len();
    if n_in > MAX_PARETO_IN {
        return Err(QdnfError::Capacity);
    }
    let mut keep = [false; MAX_PARETO_IN];
    let mut i = 0usize;
    while i < n_in {
        if !candidates[i].is_simple() {
            i += 1;
            continue;
        }
        let mut dominated = false;
        let mut j = 0usize;
        while j < n_in {
            if i != j && candidates[j].is_simple() && dominates(&candidates[j], &candidates[i]) {
                dominated = true;
                break;
            }
            j += 1;
        }
        keep[i] = !dominated;
        i += 1;
    }
    let mut tmp = [CandidatePath::EMPTY; MAX_PARETO_IN];
    let mut m = 0usize;
    i = 0;
    while i < n_in {
        if keep[i] {
            tmp[m] = candidates[i];
            m += 1;
        }
        i += 1;
    }
    i = 0;
    while i < m {
        let mut best = i;
        let mut k = i + 1;
        while k < m {
            if better_tie(&tmp[k], &tmp[best]) {
                best = k;
            }
            k += 1;
        }
        tmp.swap(i, best);
        i += 1;
    }
    let cap = out.len().min(MAX_SELECTED);
    let mut used_domain = [false; 256];
    let mut n = 0usize;
    i = 0;
    while i < m && n < cap {
        let d = tmp[i].metrics.failure_domain as usize;
        if !used_domain[d] {
            out[n] = tmp[i];
            used_domain[d] = true;
            n += 1;
        }
        i += 1;
    }
    i = 0;
    while i < m && n < cap {
        let mut already = false;
        let mut k = 0usize;
        while k < n {
            if out[k].first_hop == tmp[i].first_hop && out[k].dest == tmp[i].dest {
                already = true;
                break;
            }
            k += 1;
        }
        if !already {
            out[n] = tmp[i];
            n += 1;
        }
        i += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn route_d_dominated_candidate_dropped() {
        let cands = [
            cand(2, 9, 10, 10, 10, true, 1),
            cand(3, 9, 20, 20, 20, true, 2),
            cand(4, 9, 5, 5, 5, true, 3),
        ];
        let mut out = [CandidatePath::EMPTY; 4];
        let n = pareto_select(&cands, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 4);
    }

    #[test]
    fn route_d_at_most_three_results() {
        let cands = [
            cand(1, 9, 10, 40, 1, true, 1),
            cand(2, 9, 20, 30, 1, true, 2),
            cand(3, 9, 30, 20, 1, true, 3),
            cand(4, 9, 40, 10, 1, true, 4),
        ];
        let mut out = [CandidatePath::EMPTY; 8];
        let n = pareto_select(&cands, &mut out).unwrap();
        assert_eq!(n, 3);
        assert_eq!(out[0].first_hop, 1);
        assert_eq!(out[1].first_hop, 2);
        assert_eq!(out[2].first_hop, 3);
    }

    #[test]
    fn equal_metrics_tie_break_lower_first_hop() {
        let cands = [
            cand(5, 9, 10, 10, 10, true, 1),
            cand(2, 9, 10, 10, 10, true, 2),
        ];
        let mut out = [CandidatePath::EMPTY; 3];
        let n = pareto_select(&cands, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out[0].first_hop, 2);
        assert_eq!(out[1].first_hop, 5);
    }

    #[test]
    fn looping_path_is_dropped() {
        let mut looped = cand(2, 9, 1, 1, 1, true, 1);
        looped.hop_len = 3;
        looped.hops = [2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let ok = cand(4, 9, 8, 8, 8, true, 2);
        let mut out = [CandidatePath::EMPTY; 3];
        let n = pareto_select(&[looped, ok], &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].first_hop, 4);
    }

    #[test]
    fn empty_out_is_capacity() {
        let cands = [cand(1, 2, 1, 1, 1, true, 1)];
        let mut out: [CandidatePath; 0] = [];
        assert_eq!(pareto_select(&cands, &mut out), Err(QdnfError::Capacity));
    }
}

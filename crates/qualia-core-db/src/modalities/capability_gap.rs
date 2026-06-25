//! Gap analysis & capability logic (§24, legal_logic.md) — anti-deficit / RPL.
//!
//! Computes what is *present* versus what is *lacking* by set difference over capabilities —
//! the engine for Recognition of Prior Learning (RPL) and the "Peace-Infrastructure" deployment
//! strategy (deploy to the *computed gap*, not by assumption). Experiential/traditional
//! knowledge counts as held when an authoritative `skos:closeMatch` links it to the required
//! formal capability. Zero-heap (caller-supplied `out`).

/// Is `cap` held — directly, or via an experiential `equivalences` pair `(required, held)` that
/// recognises a held capability as equivalent to the required one?
fn holds(cap: u64, held: &[u64], equivalences: &[(u64, u64)]) -> bool {
    held.contains(&cap)
        || equivalences
            .iter()
            .any(|&(req, h)| req == cap && held.contains(&h))
}

/// The **computable gap**: the `required` capabilities that are NOT held (directly or by
/// recognised equivalence), written to `out`. Returns the count. `Gap = Req \ Holds`.
pub fn capability_gap(
    required: &[u64],
    held: &[u64],
    equivalences: &[(u64, u64)],
    out: &mut [u64],
) -> usize {
    let mut n = 0usize;
    for &cap in required {
        if !holds(cap, held, equivalences) {
            if n >= out.len() {
                break;
            }
            out[n] = cap;
            n += 1;
        }
    }
    n
}

/// Are all requirements met (the gap is empty)?
pub fn requirements_met(required: &[u64], held: &[u64], equivalences: &[(u64, u64)]) -> bool {
    required.iter().all(|&c| holds(c, held, equivalences))
}

/// RPL competency level: a held competency at `held_level` satisfies a `required_level`
/// requirement iff it is at least as high (e.g. AQF/EQF level mapping).
#[inline]
pub fn meets_competency_level(held_level: u8, required_level: u8) -> bool {
    held_level >= required_level
}

// ─── A* / Dijkstra learning-path (shortest path to close a gap) ───────────────────

/// Bound on capability nodes in one pathfinding query.
pub const MAX_CAP_NODES: usize = 64;

/// Shortest-cost educational path to acquire `goal` from the `held` capabilities, over a
/// prerequisite graph `edges = (from, to, cost)` ("from `from` you can learn `to` at `cost`").
/// Returns the minimum total cost, or `None` if `goal` is unreachable. Bounded Dijkstra (A* with
/// an admissible zero heuristic over a non-negative-cost graph); zero-heap (fixed arrays).
/// `nodes` enumerates the capability ids (the index space, ≤ [`MAX_CAP_NODES`]).
pub fn learning_path_cost(nodes: &[u64], edges: &[(u64, u64, u32)], held: &[u64], goal: u64) -> Option<u32> {
    let n = nodes.len();
    if n == 0 || n > MAX_CAP_NODES {
        return None;
    }
    let idx = |id: u64| nodes.iter().position(|&x| x == id);
    let goal_i = idx(goal)?;
    let mut dist = [u32::MAX; MAX_CAP_NODES];
    let mut done = [false; MAX_CAP_NODES];
    // Sources: every already-held capability that is in the node set starts at cost 0.
    for &h in held {
        if let Some(hi) = idx(h) {
            dist[hi] = 0;
        }
    }
    for _ in 0..n {
        // Pick the unvisited node with the smallest tentative distance.
        let mut u = usize::MAX;
        let mut best = u32::MAX;
        for i in 0..n {
            if !done[i] && dist[i] < best {
                best = dist[i];
                u = i;
            }
        }
        if u == usize::MAX {
            break; // remaining nodes unreachable
        }
        done[u] = true;
        if u == goal_i {
            return Some(dist[u]);
        }
        // Relax outgoing edges from nodes[u].
        for &(from, to, cost) in edges {
            if from == nodes[u] {
                if let Some(ti) = idx(to) {
                    let nd = dist[u].saturating_add(cost);
                    if nd < dist[ti] {
                        dist[ti] = nd;
                    }
                }
            }
        }
    }
    if dist[goal_i] == u32::MAX {
        None
    } else {
        Some(dist[goal_i])
    }
}

// ─── Probabilistic (Bayesian) capability estimation ───────────────────────────────

/// Bayesian estimate of `P(holds capability | related achievements)`: a posterior from a `prior`
/// and the fraction of `related` achievements that are `present`. Returns the prior unchanged if
/// there is no related evidence. `P = prior·L / (prior·L + (1−prior)·(1−L))`, `L = present/total`.
pub fn estimate_capability(prior: f32, present: u32, total: u32) -> f32 {
    if total == 0 {
        return prior;
    }
    let l = present as f32 / total as f32;
    let num = prior * l;
    let den = num + (1.0 - prior) * (1.0 - l);
    if den < 1e-9 {
        prior
    } else {
        num / den
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn competency_levels_and_bayesian_estimate() {
        assert!(meets_competency_level(5, 3));
        assert!(!meets_competency_level(2, 3));
        // No evidence → prior unchanged.
        assert!((estimate_capability(0.4, 0, 0) - 0.4).abs() < 1e-6);
        // Strong related evidence raises the posterior above the prior; weak lowers it.
        assert!(estimate_capability(0.5, 9, 10) > 0.5);
        assert!(estimate_capability(0.5, 1, 10) < 0.5);
    }

    #[test]
    fn a_star_finds_the_shortest_learning_path() {
        // welding(held) → fabrication(2) → robotics(3); welding → robotics directly (10).
        let (welding, fab, robotics) = (q_hash("cap:welding"), q_hash("cap:fabrication"), q_hash("cap:robotics"));
        let nodes = [welding, fab, robotics];
        let edges = [(welding, fab, 2u32), (fab, robotics, 3u32), (welding, robotics, 10u32)];
        // Shortest path welding→fab→robotics = 5 (beats the direct 10).
        assert_eq!(learning_path_cost(&nodes, &edges, &[welding], robotics), Some(5));
        // Already held → cost 0.
        assert_eq!(learning_path_cost(&nodes, &edges, &[robotics], robotics), Some(0));
        // Unreachable goal.
        let isolated = q_hash("cap:isolated");
        assert_eq!(learning_path_cost(&[welding, isolated], &edges, &[welding], isolated), None);
    }

    #[test]
    fn gap_is_the_set_difference() {
        let (welding, wiring, plumbing) = (q_hash("cap:welding"), q_hash("cap:wiring"), q_hash("cap:plumbing"));
        let required = [welding, wiring, plumbing];
        let held = [welding];
        let mut out = [0u64; 8];
        let n = capability_gap(&required, &held, &[], &mut out);
        assert_eq!(n, 2);
        assert!(out[..n].contains(&wiring) && out[..n].contains(&plumbing));
        assert!(!requirements_met(&required, &held, &[]));
    }

    #[test]
    fn experiential_equivalence_closes_the_gap() {
        let formal = q_hash("cap:formalDegree");
        let experiential = q_hash("cap:apprenticeship");
        let required = [formal];
        let held = [experiential];
        // Without recognition → a gap.
        assert_eq!(capability_gap(&required, &held, &[], &mut [0u64; 4]), 1);
        // With an authoritative closeMatch (formal ≈ experiential) → gap closed.
        let equiv = [(formal, experiential)];
        assert_eq!(capability_gap(&required, &held, &equiv, &mut [0u64; 4]), 0);
        assert!(requirements_met(&required, &held, &equiv));
    }
}

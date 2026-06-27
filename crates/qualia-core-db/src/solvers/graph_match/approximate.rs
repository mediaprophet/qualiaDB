//! Approximate fuzzy subgraph matching (Ma, Li & Ma ch 5.3) — find the mapping of a
//! query pattern's nodes onto a data graph that best matches it, tolerantly and
//! ranked by a fuzzy score. This is the **machine-proposes** half of "machine
//! proposes `closeMatch`, signed human ratifies `exactMatch`": it returns a mapping
//! **and a degree**, never a resolved identity (the out-of-band-remainder invariant).
//!
//! The search reuses `optimization::metaheuristics::hill_climbing` over node
//! assignments (no new optimizer). Kernel-class `Divergent`.

use crate::solvers::graph_match::fuzzy_similarity::FuzzyTriple;
use crate::solvers::optimization::metaheuristics::{hill_climbing, Rng};
use std::collections::HashMap;

/// A proposed (never asserted) correspondence: which data node each pattern node
/// maps to, and the fuzzy match score that earned it.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    /// `mapping[pattern_node] = data_node`.
    pub mapping: Vec<usize>,
    /// Total fuzzy match score (sum of matched-triple `t-norm` degrees). Higher is
    /// a stronger correspondence; the caller treats it as a `closeMatch` *proposal*.
    pub score: f64,
}

/// Index data triples for O(1) `(s,p,o) → degree` lookup.
fn index(data: &[FuzzyTriple]) -> HashMap<(usize, usize, usize), f64> {
    let mut m = HashMap::new();
    for t in data {
        let e = m.entry((t.s, t.p, t.o)).or_insert(0.0);
        if t.degree > *e {
            *e = t.degree;
        }
    }
    m
}

/// Fuzzy match score of a `pattern_node → data_node` mapping: for each pattern
/// triple `(s,p,o,d_p)`, if the data graph has `(map[s], p, map[o])` with degree
/// `d_d`, add `d_p · d_d` (product t-norm).
fn score(pattern: &[FuzzyTriple], idx: &HashMap<(usize, usize, usize), f64>, mapping: &[usize]) -> f64 {
    let mut s = 0.0;
    for tr in pattern {
        let key = (mapping[tr.s], tr.p, mapping[tr.o]);
        if let Some(&dd) = idx.get(&key) {
            s += tr.degree * dd;
        }
    }
    s
}

/// Find the best correspondence of `n_pattern_nodes` pattern nodes onto
/// `n_data_nodes` data nodes by `restarts` hill-climbing runs from random seeds.
/// Returns the highest-scoring mapping. `None` on a degenerate problem.
pub fn approximate_match(
    pattern: &[FuzzyTriple],
    data: &[FuzzyTriple],
    n_pattern_nodes: usize,
    n_data_nodes: usize,
    restarts: usize,
    seed: u64,
) -> Option<MatchResult> {
    if n_pattern_nodes == 0 || n_data_nodes == 0 {
        return None;
    }
    let idx = index(data);
    let objective = |m: &Vec<usize>| -score(pattern, &idx, m); // minimize negative score
    let neighbors = |m: &Vec<usize>| {
        let mut out = Vec::new();
        for i in 0..m.len() {
            for d in 0..n_data_nodes {
                if d != m[i] {
                    let mut c = m.clone();
                    c[i] = d;
                    out.push(c);
                }
            }
        }
        out
    };

    let mut rng = Rng(seed ^ 0x9E3779B97F4A7C15);
    let mut best: Option<MatchResult> = None;
    for _ in 0..restarts.max(1) {
        let init: Vec<usize> = (0..n_pattern_nodes).map(|_| rng.below(n_data_nodes)).collect();
        let (m, neg) = hill_climbing(init, &neighbors, &objective, 200);
        let sc = -neg;
        if best.as_ref().map(|b| sc > b.score).unwrap_or(true) {
            best = Some(MatchResult { mapping: m, score: sc });
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: usize, p: usize, o: usize, d: f64) -> FuzzyTriple {
        FuzzyTriple { s, p, o, degree: d }
    }

    #[test]
    fn finds_an_embedded_pattern() {
        // Data: a "knows" chain  A-knows->B-knows->C  (predicate 0), strong degrees.
        // Pattern: x-knows->y-knows->z. The best mapping is x→A, y→B, z→C.
        let data = [t(10, 0, 11, 0.9), t(11, 0, 12, 0.9), t(99, 0, 98, 0.2)];
        let pattern = [t(0, 0, 1, 1.0), t(1, 0, 2, 1.0)]; // 3 pattern nodes 0,1,2
        let r = approximate_match(&pattern, &data, 3, 13, 8, 1).unwrap();
        assert!(r.score > 1.5, "score {}", r.score);
        // Pattern node 0→10, 1→11, 2→12 (or an equally-scoring relabel).
        assert_eq!(r.mapping[0], 10);
        assert_eq!(r.mapping[1], 11);
        assert_eq!(r.mapping[2], 12);
    }

    #[test]
    fn partial_match_scores_lower_than_full() {
        let data = [t(10, 0, 11, 0.9)]; // only one edge
        let pattern = [t(0, 0, 1, 1.0), t(1, 0, 2, 1.0)]; // wants two edges
        let r = approximate_match(&pattern, &data, 3, 12, 6, 2).unwrap();
        // At most one pattern edge can match → score ≤ ~0.9.
        assert!(r.score <= 0.9 + 1e-9 && r.score > 0.0);
    }

    #[test]
    fn returns_a_degree_not_an_assertion() {
        // The result is a score-bearing proposal; the API never claims identity.
        let data = [t(0, 0, 1, 0.5)];
        let pattern = [t(0, 0, 1, 0.5)];
        let r = approximate_match(&pattern, &data, 2, 2, 4, 3).unwrap();
        assert!(r.score > 0.0 && r.score <= 1.0);
        assert_eq!(r.mapping.len(), 2);
    }

    #[test]
    fn guards() {
        assert!(approximate_match(&[], &[], 0, 0, 1, 0).is_none());
    }
}

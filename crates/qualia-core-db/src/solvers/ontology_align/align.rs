//! Ontology alignment as optimization (CI-SKM ch 4) — given a similarity matrix
//! between two ontologies' entities, find the injective correspondence set that
//! maximizes total alignment quality. Greedy initialization refined by
//! hill-climbing (reusing `optimization::metaheuristics`, no new optimizer).
//!
//! The output is a set of graded **`closeMatch` proposals → `RequiresHumanReview`**
//! ([`super::correspondence`]); it never asserts `exactMatch`. Kernel-class
//! `Divergent`.

use crate::solvers::ontology_align::correspondence::{Alignment, Correspondence};
use crate::solvers::optimization::metaheuristics::hill_climbing;

/// Total matched similarity of an injective assignment `map[source] = target`
/// (`-1` = unmatched). Row-major `sim` is `n_source × n_target`.
fn quality(sim: &[f64], n_target: usize, map: &[i64]) -> f64 {
    let mut q = 0.0;
    for (i, &t) in map.iter().enumerate() {
        if t >= 0 {
            q += sim[i * n_target + t as usize];
        }
    }
    q
}

/// Align two ontologies from their `n_source × n_target` similarity matrix. Only
/// correspondences with degree ≥ `threshold` are proposed. Returns the alignment as
/// review-required `closeMatch` proposals.
pub fn align(sim: &[f64], n_source: usize, n_target: usize, threshold: f64) -> Option<Alignment> {
    if n_source == 0 || n_target == 0 || sim.len() != n_source * n_target {
        return None;
    }

    // Greedy initialization: take the strongest available pairs first.
    let mut pairs: Vec<(usize, usize, f64)> = Vec::with_capacity(n_source * n_target);
    for i in 0..n_source {
        for j in 0..n_target {
            pairs.push((i, j, sim[i * n_target + j]));
        }
    }
    pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
    let mut map = vec![-1i64; n_source];
    let mut target_used = vec![false; n_target];
    for (i, j, s) in pairs {
        if s >= threshold && map[i] < 0 && !target_used[j] {
            map[i] = j as i64;
            target_used[j] = true;
        }
    }

    // Hill-climb: for each source, try reassigning to a free target (or unmatch),
    // keeping the assignment injective.
    let neighbors = |m: &Vec<i64>| {
        let mut used = vec![false; n_target];
        for &t in m {
            if t >= 0 {
                used[t as usize] = true;
            }
        }
        let mut out = Vec::new();
        for i in 0..n_source {
            // Unmatch.
            if m[i] >= 0 {
                let mut c = m.clone();
                c[i] = -1;
                out.push(c);
            }
            for j in 0..n_target {
                let free = !used[j] || m[i] == j as i64;
                if free && sim[i * n_target + j] >= threshold && m[i] != j as i64 {
                    let mut c = m.clone();
                    c[i] = j as i64;
                    out.push(c);
                }
            }
        }
        out
    };
    let objective = |m: &Vec<i64>| -quality(sim, n_target, m);
    let (best, neg_q) = hill_climbing(map, &neighbors, &objective, 200);

    let correspondences: Vec<Correspondence> = best
        .iter()
        .enumerate()
        .filter_map(|(i, &t)| {
            if t >= 0 {
                let j = t as usize;
                Some(Correspondence::propose(i, j, sim[i * n_target + j]))
            } else {
                None
            }
        })
        .collect();

    Some(Alignment { correspondences, quality: -neg_q })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_the_diagonal_correspondence() {
        // 3×3 similarity, strong on the diagonal → align i↔i.
        let sim = [
            0.9, 0.1, 0.2,
            0.1, 0.8, 0.3,
            0.2, 0.2, 0.95,
        ];
        let a = align(&sim, 3, 3, 0.5).unwrap();
        assert_eq!(a.correspondences.len(), 3);
        for c in &a.correspondences {
            assert_eq!(c.source, c.target, "expected diagonal match");
        }
        assert!(a.quality > 2.5);
        // The guardrail: every correspondence is a review-required close-match.
        assert!(a.all_require_review());
    }

    #[test]
    fn weak_pairs_below_threshold_are_not_proposed() {
        // Only one strong pair; the rest are below threshold.
        let sim = [0.9, 0.1, 0.1, 0.1];
        let a = align(&sim, 2, 2, 0.5).unwrap();
        assert_eq!(a.correspondences.len(), 1);
        assert_eq!((a.correspondences[0].source, a.correspondences[0].target), (0, 0));
    }

    #[test]
    fn never_asserts_exact_match() {
        // No matter the input, the engine only emits review-required proposals.
        let sim = [1.0, 1.0, 1.0, 1.0];
        let a = align(&sim, 2, 2, 0.0).unwrap();
        assert!(a.all_require_review());
        assert!(a.correspondences.iter().all(|c| c.degree <= 1.0));
    }

    #[test]
    fn guards() {
        assert!(align(&[], 0, 0, 0.5).is_none());
        assert!(align(&[1.0, 2.0], 2, 2, 0.5).is_none()); // wrong length
    }
}

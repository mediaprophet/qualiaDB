//! Spreading activation (Kornai, *Vector Semantics* ch 7.4) — propagate an
//! activation level from seed concepts through the semantic-network edges, decaying
//! with distance. The classic associative-retrieval mechanism: given a query's seed
//! concepts, it ranks which graph regions are most relevant.
//!
//! Mission fit: a natural engine for the 10D→5D NQuin **relevance router**
//! ("streaming lives in retrieval, not attention; we own the cache") — complementary
//! to PCA and mutual information on the same router. Kernel-class `Reduction` (the
//! per-hop weighted sums); bounded iteration, CPU reference.

use std::collections::HashMap;

/// A directed weighted edge `from → to` with non-negative `weight`.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

/// Spread activation from `seeds` (node, initial activation) over `n_nodes` through
/// `edges`, decaying by `decay ∈ (0,1]` each hop and pruning contributions below
/// `threshold`. Runs at most `max_hops`. Returns total accumulated activation per
/// node (the relevance score).
pub fn spreading_activation(
    n_nodes: usize,
    edges: &[Edge],
    seeds: &[(usize, f64)],
    decay: f64,
    threshold: f64,
    max_hops: usize,
) -> Vec<f64> {
    let mut total = vec![0.0; n_nodes];
    if n_nodes == 0 {
        return total;
    }
    // Outgoing adjacency.
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_nodes];
    for e in edges {
        if e.from < n_nodes && e.to < n_nodes && e.weight > 0.0 {
            adj[e.from].push((e.to, e.weight));
        }
    }
    // Wavefront of "newly activated this hop".
    let mut current: HashMap<usize, f64> = HashMap::new();
    for &(node, a) in seeds {
        if node < n_nodes && a > 0.0 {
            *current.entry(node).or_insert(0.0) += a;
            total[node] += a;
        }
    }
    let decay = decay.clamp(0.0, 1.0);
    for _ in 0..max_hops {
        if current.is_empty() {
            break;
        }
        let mut next: HashMap<usize, f64> = HashMap::new();
        for (&node, &a) in &current {
            for &(to, w) in &adj[node] {
                let contrib = a * w * decay;
                if contrib > threshold {
                    *next.entry(to).or_insert(0.0) += contrib;
                }
            }
        }
        for (&node, &a) in &next {
            total[node] += a;
        }
        current = next;
    }
    total
}

/// Indices of the top-`k` most-activated nodes (relevance ranking), highest first.
pub fn top_k(activation: &[f64], k: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..activation.len()).collect();
    idx.sort_by(|&a, &b| activation[b].partial_cmp(&activation[a]).unwrap_or(core::cmp::Ordering::Equal));
    idx.truncate(k.min(activation.len()));
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(from: usize, to: usize, w: f64) -> Edge {
        Edge { from, to, weight: w }
    }

    #[test]
    fn activation_decays_along_a_chain() {
        // 0 → 1 → 2 → 3, all weight 1. Seed node 0.
        let edges = [e(0, 1, 1.0), e(1, 2, 1.0), e(2, 3, 1.0)];
        let act = spreading_activation(4, &edges, &[(0, 1.0)], 0.5, 1e-6, 10);
        // Strictly decreasing activation with distance from the seed.
        assert!(act[0] > act[1] && act[1] > act[2] && act[2] > act[3]);
        assert!(act[3] > 0.0);
    }

    #[test]
    fn closer_nodes_rank_higher() {
        // Star: 0 → {1,2,3}; and 1 → 4 (one hop further).
        let edges = [e(0, 1, 1.0), e(0, 2, 1.0), e(0, 3, 1.0), e(1, 4, 1.0)];
        let act = spreading_activation(5, &edges, &[(0, 1.0)], 0.6, 1e-9, 10);
        // Direct neighbours outrank the two-hop node.
        assert!(act[1] > act[4]);
        let ranking = top_k(&act, 3);
        assert_eq!(ranking[0], 0); // the seed itself is most active
    }

    #[test]
    fn weights_bias_the_spread() {
        // 0 → 1 (strong), 0 → 2 (weak): node 1 should out-activate node 2.
        let edges = [e(0, 1, 0.9), e(0, 2, 0.1)];
        let act = spreading_activation(3, &edges, &[(0, 1.0)], 1.0, 1e-9, 5);
        assert!(act[1] > act[2]);
    }

    #[test]
    fn empty_and_threshold() {
        assert_eq!(spreading_activation(0, &[], &[], 0.5, 0.0, 5).len(), 0);
        // A high threshold prunes everything beyond the seed.
        let edges = [e(0, 1, 0.01)];
        let act = spreading_activation(2, &edges, &[(0, 1.0)], 0.5, 0.1, 5);
        assert_eq!(act[1], 0.0);
    }
}

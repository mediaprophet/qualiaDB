//! Hierarchical / fractal shortest-path decomposition (Riehl-Hespanha, *Fractal
//! Graph Optimization*) — solve shortest paths by splitting the graph into clusters
//! and composing a **high-level portal problem** with **independent intra-cluster
//! subproblems**. The subproblems are independent, so they map naturally onto the
//! engine's independent 512 MB fractal-swarm worker cells (affordability: less
//! coupled compute, distributable).
//!
//! With the full border set the decomposition is **exact** (it matches plain
//! Dijkstra — any inter-cluster path must cross a border), which the tests verify.
//! [`dijkstra`] is the always-present CPU reference. Kernel-class `Reduction`.

use std::collections::HashMap;

/// Plain Dijkstra (the exact reference). Returns shortest distance from `source` to
/// every node over the directed weighted graph; unreachable nodes are `f64::INFINITY`.
/// `edges_of[i]` lists `(neighbour, weight)`.
pub fn dijkstra(n: usize, edges_of: &[Vec<(usize, f64)>], source: usize) -> Vec<f64> {
    let mut dist = vec![f64::INFINITY; n];
    if source >= n {
        return dist;
    }
    dist[source] = 0.0;
    let mut visited = vec![false; n];
    for _ in 0..n {
        // O(V²) selection (graphs here are user-side small).
        let mut u = usize::MAX;
        let mut best = f64::INFINITY;
        for i in 0..n {
            if !visited[i] && dist[i] < best {
                best = dist[i];
                u = i;
            }
        }
        if u == usize::MAX {
            break;
        }
        visited[u] = true;
        for &(v, w) in &edges_of[u] {
            if dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
            }
        }
    }
    dist
}

/// Shortest distance from `source` to `target` using **only** edges whose both
/// endpoints are in `cluster_of == cluster` (an intra-cluster subproblem). `∞` if no
/// in-cluster path. This is the independent, cell-distributable piece.
fn intra_distance(
    edges_of: &[Vec<(usize, f64)>],
    cluster_of: &[usize],
    cluster: usize,
    source: usize,
    target: usize,
) -> f64 {
    let n = edges_of.len();
    let mut dist = vec![f64::INFINITY; n];
    dist[source] = 0.0;
    let mut visited = vec![false; n];
    loop {
        let mut u = usize::MAX;
        let mut best = f64::INFINITY;
        for i in 0..n {
            if !visited[i] && cluster_of[i] == cluster && dist[i] < best {
                best = dist[i];
                u = i;
            }
        }
        if u == usize::MAX {
            break;
        }
        visited[u] = true;
        for &(v, w) in &edges_of[u] {
            if cluster_of[v] == cluster && dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
            }
        }
    }
    dist[target]
}

/// Hierarchical shortest distance from `s` to `t` using the cluster decomposition in
/// `cluster_of`. Builds a small portal graph over the border nodes (+ `s`, `t`), so
/// the only global work is on borders; everything else is independent intra-cluster
/// subproblems. Returns the exact shortest distance.
pub fn hierarchical_shortest_path(
    n: usize,
    edges_of: &[Vec<(usize, f64)>],
    cluster_of: &[usize],
    s: usize,
    t: usize,
) -> f64 {
    if s >= n || t >= n {
        return f64::INFINITY;
    }
    if cluster_of[s] == cluster_of[t] {
        // Same cluster: but the optimal path could leave and re-enter, so fall
        // through to the portal construction unless it's trivially intra-only.
        let intra = intra_distance(edges_of, cluster_of, cluster_of[s], s, t);
        // Still build the portal graph and take the min (handles leave-and-return).
        let portal = portal_distance(n, edges_of, cluster_of, s, t);
        return intra.min(portal);
    }
    portal_distance(n, edges_of, cluster_of, s, t)
}

/// Build the border portal graph (+ s, t as temporary nodes) and Dijkstra over it.
fn portal_distance(
    n: usize,
    edges_of: &[Vec<(usize, f64)>],
    cluster_of: &[usize],
    s: usize,
    t: usize,
) -> f64 {
    // Border nodes: have an edge crossing clusters (either direction).
    let mut is_border = vec![false; n];
    for u in 0..n {
        for &(v, _) in &edges_of[u] {
            if cluster_of[u] != cluster_of[v] {
                is_border[u] = true;
                is_border[v] = true;
            }
        }
    }
    // Portal node set: borders ∪ {s, t}.
    let mut nodes: Vec<usize> = (0..n).filter(|&i| is_border[i]).collect();
    if !nodes.contains(&s) {
        nodes.push(s);
    }
    if !nodes.contains(&t) {
        nodes.push(t);
    }
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, &orig)| (orig, i)).collect();
    let m = nodes.len();
    let mut portal_edges: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];

    // Intra-cluster shortest distances between every pair of portal nodes in the
    // same cluster (the independent subproblems).
    for &a in &nodes {
        for &b in &nodes {
            if a != b && cluster_of[a] == cluster_of[b] {
                let d = intra_distance(edges_of, cluster_of, cluster_of[a], a, b);
                if d.is_finite() {
                    portal_edges[index[&a]].push((index[&b], d));
                }
            }
        }
    }
    // Cross edges (connect borders of different clusters) carried as-is.
    for u in 0..n {
        for &(v, w) in &edges_of[u] {
            if cluster_of[u] != cluster_of[v] {
                if let (Some(&iu), Some(&iv)) = (index.get(&u), index.get(&v)) {
                    portal_edges[iu].push((iv, w));
                }
            }
        }
    }

    let dist = dijkstra(m, &portal_edges, index[&s]);
    dist[index[&t]]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an adjacency list from undirected weighted edges.
    fn graph(n: usize, edges: &[(usize, usize, f64)]) -> Vec<Vec<(usize, f64)>> {
        let mut adj = vec![Vec::new(); n];
        for &(a, b, w) in edges {
            adj[a].push((b, w));
            adj[b].push((a, w));
        }
        adj
    }

    #[test]
    fn dijkstra_matches_known_distances() {
        // 0-1(1) 1-2(2) 0-2(4): shortest 0→2 is via 1 = 3.
        let adj = graph(3, &[(0, 1, 1.0), (1, 2, 2.0), (0, 2, 4.0)]);
        let d = dijkstra(3, &adj, 0);
        assert!((d[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn hierarchical_equals_exact_across_clusters() {
        // Two clusters {0,1,2} and {3,4,5}, joined by 2-3.
        let edges = [
            (0, 1, 1.0), (1, 2, 1.0), (0, 2, 3.0), // cluster A
            (3, 4, 1.0), (4, 5, 1.0), (3, 5, 3.0), // cluster B
            (2, 3, 2.0),                            // bridge
        ];
        let adj = graph(6, &edges);
        let cluster = [0, 0, 0, 1, 1, 1];
        // Check every pair against exact Dijkstra.
        for s in 0..6 {
            let exact = dijkstra(6, &adj, s);
            for t in 0..6 {
                let h = hierarchical_shortest_path(6, &adj, &cluster, s, t);
                assert!((h - exact[t]).abs() < 1e-9, "s={s} t={t}: hier {h} vs exact {}", exact[t]);
            }
        }
    }

    #[test]
    fn hierarchical_handles_leave_and_return() {
        // Within-cluster nodes whose best path goes through the other cluster.
        let edges = [
            (0, 1, 10.0), // expensive intra link
            (0, 2, 1.0), (2, 3, 1.0), (3, 1, 1.0), // cheap detour via cluster B
        ];
        let adj = graph(4, &edges);
        let cluster = [0, 0, 1, 1];
        let exact = dijkstra(4, &adj, 0);
        let h = hierarchical_shortest_path(4, &adj, &cluster, 0, 1);
        assert!((h - exact[1]).abs() < 1e-9, "leave-and-return: {h} vs {}", exact[1]);
        assert!(h < 10.0, "should take the cheap detour");
    }
}

// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Spatial Network Routing, Turn Restrictions, Elevation Penalties, Isochrones,
//! TSP 2-Opt, Kruskal MST, Edmonds-Karp Max-Flow Min-Cut, and Centrality.
//!
//! Allocation Class: Tier-2 (`ColdBounded`), enforcing caller-supplied `byte_budget`.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetworkNode { pub id: u32, pub x: f64, pub y: f64, pub z_m: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetworkEdge { pub id: u32, pub from: usize, pub to: usize, pub base_cost: f64, pub capacity: f64, pub one_way: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnRestriction { pub via_node: usize, pub from_node: usize, pub to_node: usize, pub penalty_cost: f64 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpNetworkError {
    InvalidNodeIndex(usize),
    InvalidEdgeEndpoint { edge_id: u32, endpoint: usize },
    NegativeCostOrCapacity,
    NoPathFound,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for GpNetworkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidNodeIndex(i) => write!(f, "Invalid node index: {i}"),
            Self::InvalidEdgeEndpoint { edge_id, endpoint } => write!(f, "Invalid endpoint {endpoint} on edge {edge_id}"),
            Self::NegativeCostOrCapacity => write!(f, "Negative cost or capacity in network"),
            Self::NoPathFound => write!(f, "No path found between requested nodes"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Budget exceeded: req {requested} B, budget {budget} B"),
        }
    }
}
impl std::error::Error for GpNetworkError {}

#[derive(Clone, Copy, PartialEq)]
struct OrderedCost(f64);
impl Eq for OrderedCost {}
impl Ord for OrderedCost {
    fn cmp(&self, o: &Self) -> core::cmp::Ordering { self.0.partial_cmp(&o.0).unwrap_or(core::cmp::Ordering::Equal) }
}
impl PartialOrd for OrderedCost {
    fn partial_cmp(&self, o: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(o)) }
}

struct DisjointSet { parent: Vec<usize>, rank: Vec<u8> }
impl DisjointSet {
    fn new(n: usize) -> Self { Self { parent: (0..n).collect(), rank: vec![0; n] } }
    fn find(&mut self, mut x: usize) -> usize {
        while x != self.parent[x] { self.parent[x] = self.parent[self.parent[x]]; x = self.parent[x]; }
        x
    }
    fn union(&mut self, x: usize, y: usize) -> bool {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry { return false; }
        if self.rank[rx] < self.rank[ry] { self.parent[rx] = ry; }
        else if self.rank[rx] > self.rank[ry] { self.parent[ry] = rx; }
        else { self.parent[ry] = rx; self.rank[rx] += 1; }
        true
    }
}

#[derive(Clone)]
struct FlowEdge { to: usize, rev: usize, capacity: f64, flow: f64 }

fn validate_network(n: usize, edges: &[NetworkEdge]) -> Result<(), GpNetworkError> {
    for e in edges {
        if e.from >= n { return Err(GpNetworkError::InvalidEdgeEndpoint { edge_id: e.id, endpoint: e.from }); }
        if e.to >= n { return Err(GpNetworkError::InvalidEdgeEndpoint { edge_id: e.id, endpoint: e.to }); }
        if e.base_cost < 0.0 || e.capacity < 0.0 { return Err(GpNetworkError::NegativeCostOrCapacity); }
    }
    Ok(())
}

fn build_adjacency(n: usize, edges: &[NetworkEdge]) -> Vec<Vec<(usize, f64)>> {
    let mut adj = vec![Vec::new(); n];
    for e in edges {
        adj[e.from].push((e.to, e.base_cost));
        if !e.one_way { adj[e.to].push((e.from, e.base_cost)); }
    }
    adj
}

/// Shortest path with turn restrictions and elevation grade penalty (Tier-2 ColdBounded).
pub fn gp_shortest_path_astar_turn_grade(
    nodes: &[NetworkNode], edges: &[NetworkEdge], turns: &[TurnRestriction],
    start: usize, goal: usize, uphill_penalty_per_m: f64, byte_budget: usize,
) -> Result<(f64, Vec<usize>), GpNetworkError> {
    if start >= nodes.len() { return Err(GpNetworkError::InvalidNodeIndex(start)); }
    if goal >= nodes.len() { return Err(GpNetworkError::InvalidNodeIndex(goal)); }
    validate_network(nodes.len(), edges)?;
    let req = nodes.len() * 48 + edges.len() * 64 + turns.len() * 32 + 512;
    if req > byte_budget { return Err(GpNetworkError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    if start == goal { return Ok((0.0, vec![start])); }

    let adj = build_adjacency(nodes.len(), edges);
    let (mut dist, mut parent, mut pq) = (HashMap::new(), HashMap::new(), BinaryHeap::new());
    dist.insert((usize::MAX, start), 0.0);
    pq.push((Reverse(OrderedCost(0.0)), usize::MAX, start));

    let mut target_state = None;
    while let Some((Reverse(OrderedCost(g)), prev, curr)) = pq.pop() {
        if g > *dist.get(&(prev, curr)).unwrap_or(&f64::INFINITY) { continue; }
        if curr == goal { target_state = Some((prev, curr, g)); break; }

        for &(next_node, base_cost) in &adj[curr] {
            let mut turn_penalty = 0.0;
            if prev != usize::MAX {
                if let Some(t) = turns.iter().find(|t| t.via_node == curr && t.from_node == prev && t.to_node == next_node) {
                    turn_penalty = t.penalty_cost;
                }
            }
            if turn_penalty.is_infinite() || turn_penalty >= 1e30 { continue; }

            let dz = nodes[next_node].z_m - nodes[curr].z_m;
            let grade_penalty = if dz > 0.0 { dz * uphill_penalty_per_m } else { 0.0 };
            let next_g = g + base_cost + grade_penalty + turn_penalty;

            let state_key = (curr, next_node);
            if next_g < *dist.get(&state_key).unwrap_or(&f64::INFINITY) {
                dist.insert(state_key, next_g);
                parent.insert(state_key, (prev, curr));
                pq.push((Reverse(OrderedCost(next_g)), curr, next_node));
            }
        }
    }

    let (mut p_prev, mut p_curr, total_cost) = target_state.ok_or(GpNetworkError::NoPathFound)?;
    let mut path = Vec::new();
    while p_curr != start || p_prev != usize::MAX {
        path.push(p_curr);
        let p = parent[&(p_prev, p_curr)];
        p_prev = p.0; p_curr = p.1;
    }
    path.push(start);
    path.reverse();
    Ok((total_cost, path))
}

/// Multi-origin Isochrone Service Area calculation (Tier-2 ColdBounded).
pub fn gp_service_area_isochrone_nodes(
    nodes: &[NetworkNode], edges: &[NetworkEdge], origins: &[usize],
    max_cost_budget: f64, byte_budget: usize,
) -> Result<Vec<(usize, f64)>, GpNetworkError> {
    for &orig in origins {
        if orig >= nodes.len() { return Err(GpNetworkError::InvalidNodeIndex(orig)); }
    }
    validate_network(nodes.len(), edges)?;
    let req = nodes.len() * 32 + edges.len() * 32 + origins.len() * 16 + 512;
    if req > byte_budget { return Err(GpNetworkError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }

    let adj = build_adjacency(nodes.len(), edges);
    let mut dist = vec![f64::INFINITY; nodes.len()];
    let mut pq = BinaryHeap::new();
    for &orig in origins { dist[orig] = 0.0; pq.push((Reverse(OrderedCost(0.0)), orig)); }

    while let Some((Reverse(OrderedCost(d)), u)) = pq.pop() {
        if d > dist[u] || d > max_cost_budget { continue; }
        for &(v, cost) in &adj[u] {
            let next_d = d + cost;
            if next_d <= max_cost_budget && next_d < dist[v] {
                dist[v] = next_d;
                pq.push((Reverse(OrderedCost(next_d)), v));
            }
        }
    }

    let mut reachable = Vec::new();
    for (idx, &d) in dist.iter().enumerate() {
        if d <= max_cost_budget { reachable.push((idx, d)); }
    }
    Ok(reachable)
}

/// Travelling Salesperson Problem solver using 2-opt edge-swap refinement (Tier-2 ColdBounded).
pub fn gp_tsp_tour_2opt(
    nodes: &[NetworkNode], stops: &[usize], byte_budget: usize,
) -> Result<(f64, Vec<usize>), GpNetworkError> {
    for &s in stops {
        if s >= nodes.len() { return Err(GpNetworkError::InvalidNodeIndex(s)); }
    }
    let req = stops.len() * 32 + 256;
    if req > byte_budget { return Err(GpNetworkError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    if stops.is_empty() { return Ok((0.0, Vec::new())); }
    if stops.len() == 1 { return Ok((0.0, vec![stops[0], stops[0]])); }

    let dist = |a: usize, b: usize| -> f64 {
        let (na, nb) = (nodes[a], nodes[b]);
        ((na.x - nb.x).powi(2) + (na.y - nb.y).powi(2) + (na.z_m - nb.z_m).powi(2)).sqrt()
    };

    let mut visited = vec![false; stops.len()];
    let mut tour = Vec::with_capacity(stops.len() + 1);
    tour.push(stops[0]);
    visited[0] = true;
    let mut curr_stop = 0;

    for _ in 1..stops.len() {
        let (mut nearest, mut min_d) = (usize::MAX, f64::INFINITY);
        for (j, &vis) in visited.iter().enumerate() {
            if !vis {
                let d = dist(stops[curr_stop], stops[j]);
                if d < min_d { min_d = d; nearest = j; }
            }
        }
        visited[nearest] = true;
        tour.push(stops[nearest]);
        curr_stop = nearest;
    }
    tour.push(stops[0]);

    let (mut improved, mut iters) = (true, 0);
    while improved && iters < 500 {
        improved = false;
        iters += 1;
        for i in 1..stops.len() - 1 {
            for j in (i + 1)..stops.len() {
                let d_old = dist(tour[i - 1], tour[i]) + dist(tour[j], tour[j + 1]);
                let d_new = dist(tour[i - 1], tour[j]) + dist(tour[i], tour[j + 1]);
                if d_new < d_old - 1e-9 { tour[i..=j].reverse(); improved = true; }
            }
        }
    }

    let total_len: f64 = tour.windows(2).map(|w| dist(w[0], w[1])).sum();
    Ok((total_len, tour))
}

/// Kruskal Minimum Spanning Tree with Union-Find path compression (Tier-2 ColdBounded).
pub fn gp_minimum_spanning_tree_kruskal(
    num_nodes: usize, edges: &[NetworkEdge], byte_budget: usize,
) -> Result<(f64, Vec<u32>), GpNetworkError> {
    validate_network(num_nodes, edges)?;
    let req = num_nodes * 16 + edges.len() * 24 + 256;
    if req > byte_budget { return Err(GpNetworkError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }

    let mut sorted_edges: Vec<&NetworkEdge> = edges.iter().collect();
    sorted_edges.sort_by(|a, b| {
        a.base_cost.partial_cmp(&b.base_cost).unwrap_or(core::cmp::Ordering::Equal).then_with(|| a.id.cmp(&b.id))
    });

    let mut dsu = DisjointSet::new(num_nodes);
    let (mut total_weight, mut mst_edge_ids) = (0.0, Vec::new());
    for edge in sorted_edges {
        if dsu.union(edge.from, edge.to) {
            total_weight += edge.base_cost;
            mst_edge_ids.push(edge.id);
            if mst_edge_ids.len() == num_nodes.saturating_sub(1) { break; }
        }
    }
    Ok((total_weight, mst_edge_ids))
}

/// Edmonds-Karp BFS augmenting-path maximum flow & min-cut vertex partition (Tier-2 ColdBounded).
pub fn gp_max_flow_edmonds_karp(
    num_nodes: usize, edges: &[NetworkEdge], source: usize, sink: usize, byte_budget: usize,
) -> Result<(f64, Vec<usize>), GpNetworkError> {
    if source >= num_nodes { return Err(GpNetworkError::InvalidNodeIndex(source)); }
    if sink >= num_nodes { return Err(GpNetworkError::InvalidNodeIndex(sink)); }
    validate_network(num_nodes, edges)?;
    let req = num_nodes * 64 + edges.len() * 48 + 512;
    if req > byte_budget { return Err(GpNetworkError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    if source == sink { return Ok((0.0, vec![source])); }

    let mut adj: Vec<Vec<FlowEdge>> = vec![Vec::new(); num_nodes];
    for e in edges {
        let (u, v) = (e.from, e.to);
        let (f_cap, b_cap) = (e.capacity, if e.one_way { 0.0 } else { e.capacity });
        let (f_idx, b_idx) = (adj[u].len(), adj[v].len() + if u == v { 1 } else { 0 });
        adj[u].push(FlowEdge { to: v, rev: b_idx, capacity: f_cap, flow: 0.0 });
        adj[v].push(FlowEdge { to: u, rev: f_idx, capacity: b_cap, flow: 0.0 });
    }

    let mut max_flow = 0.0;
    loop {
        let mut parent: Vec<Option<(usize, usize)>> = vec![None; num_nodes];
        let mut queue = VecDeque::new();
        queue.push_back(source);

        while let Some(u) = queue.pop_front() {
            if u == sink { break; }
            for (e_idx, edge) in adj[u].iter().enumerate() {
                if edge.to != source && parent[edge.to].is_none() && (edge.capacity - edge.flow) > 1e-12 {
                    parent[edge.to] = Some((u, e_idx));
                    queue.push_back(edge.to);
                }
            }
        }
        if parent[sink].is_none() { break; }

        let (mut curr, mut bottleneck) = (sink, f64::INFINITY);
        while curr != source {
            let (p, e_idx) = parent[curr].unwrap();
            bottleneck = bottleneck.min(adj[p][e_idx].capacity - adj[p][e_idx].flow);
            curr = p;
        }

        curr = sink;
        while curr != source {
            let (p, e_idx) = parent[curr].unwrap();
            adj[p][e_idx].flow += bottleneck;
            let rev_idx = adj[p][e_idx].rev;
            adj[curr][rev_idx].flow -= bottleneck;
            curr = p;
        }
        max_flow += bottleneck;
    }

    let mut visited = vec![false; num_nodes];
    let mut queue = VecDeque::new();
    visited[source] = true;
    queue.push_back(source);
    while let Some(u) = queue.pop_front() {
        for edge in &adj[u] {
            if !visited[edge.to] && (edge.capacity - edge.flow) > 1e-12 {
                visited[edge.to] = true;
                queue.push_back(edge.to);
            }
        }
    }
    let cut_nodes: Vec<usize> = (0..num_nodes).filter(|&i| visited[i]).collect();
    Ok((max_flow, cut_nodes))
}

/// Brandes Closeness and Betweenness Centrality for all nodes (Tier-2 ColdBounded).
pub fn gp_centrality_closeness_betweenness(
    num_nodes: usize, edges: &[NetworkEdge], byte_budget: usize,
) -> Result<(Vec<f64>, Vec<f64>), GpNetworkError> {
    validate_network(num_nodes, edges)?;
    let req = num_nodes * 64 + edges.len() * 32 + 512;
    if req > byte_budget { return Err(GpNetworkError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }

    let adj = build_adjacency(num_nodes, edges);
    let is_undirected = edges.iter().all(|e| !e.one_way);
    let (mut closeness, mut betweenness) = (vec![0.0; num_nodes], vec![0.0; num_nodes]);

    for s in 0..num_nodes {
        let (mut stack, mut p) = (Vec::with_capacity(num_nodes), vec![Vec::new(); num_nodes]);
        let (mut sigma, mut d) = (vec![0.0; num_nodes], vec![f64::INFINITY; num_nodes]);
        sigma[s] = 1.0;
        d[s] = 0.0;
        let mut pq = BinaryHeap::new();
        pq.push((Reverse(OrderedCost(0.0)), s));

        while let Some((Reverse(OrderedCost(dist_u)), u)) = pq.pop() {
            if dist_u > d[u] { continue; }
            stack.push(u);
            for &(v, weight) in &adj[u] {
                let new_d = dist_u + weight;
                if new_d < d[v] - 1e-12 {
                    d[v] = new_d;
                    pq.push((Reverse(OrderedCost(new_d)), v));
                    sigma[v] = sigma[u];
                    p[v].clear();
                    p[v].push(u);
                } else if (new_d - d[v]).abs() <= 1e-12 {
                    sigma[v] += sigma[u];
                    p[v].push(u);
                }
            }
        }

        let (mut reachable, mut sum_d) = (0usize, 0.0);
        for (t, &dist_t) in d.iter().enumerate() {
            if t != s && dist_t.is_finite() && dist_t > 0.0 { reachable += 1; sum_d += dist_t; }
        }
        if reachable > 0 && sum_d > 0.0 { closeness[s] = (reachable as f64) / sum_d; }

        let mut delta = vec![0.0; num_nodes];
        while let Some(w) = stack.pop() {
            for &v in &p[w] {
                if sigma[w] > 0.0 { delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]); }
            }
            if w != s { betweenness[w] += delta[w]; }
        }
    }

    if is_undirected {
        for b in &mut betweenness { *b /= 2.0; }
    }
    Ok((closeness, betweenness))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u32, x: f64, y: f64, z_m: f64) -> NetworkNode { NetworkNode { id, x, y, z_m } }
    fn edge(id: u32, from: usize, to: usize, base_cost: f64, capacity: f64, one_way: bool) -> NetworkEdge {
        NetworkEdge { id, from, to, base_cost, capacity, one_way }
    }

    #[test]
    fn test_shortest_path_turn_grade() {
        let nodes = vec![node(0, 0.0, 0.0, 0.0), node(1, 10.0, 0.0, 0.0), node(2, 0.0, 10.0, 20.0), node(3, 10.0, 10.0, 0.0)];
        let edges = vec![
            edge(0, 0, 1, 10.0, 1.0, true), edge(1, 1, 3, 10.0, 1.0, true),
            edge(2, 0, 2, 15.0, 1.0, true), edge(3, 2, 3, 15.0, 1.0, true),
        ];
        let turns = vec![TurnRestriction { via_node: 1, from_node: 0, to_node: 3, penalty_cost: f64::INFINITY }];
        let (cost, path) = gp_shortest_path_astar_turn_grade(&nodes, &edges, &turns, 0, 3, 0.5, 100_000).unwrap();
        assert_eq!(path, vec![0, 2, 3]);
        assert!((cost - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_service_area_isochrone() {
        let nodes: Vec<_> = (0..5).map(|i| node(i, i as f64 * 10.0, 0.0, 0.0)).collect();
        let edges: Vec<_> = (0..4).map(|i| edge(i, i as usize, (i + 1) as usize, 10.0, 1.0, false)).collect();
        let reachable = gp_service_area_isochrone_nodes(&nodes, &edges, &[0], 25.0, 100_000).unwrap();
        assert_eq!(reachable.into_iter().map(|(i, _)| i).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn test_tsp_tour_2opt() {
        let nodes = vec![node(0, 0.0, 0.0, 0.0), node(1, 10.0, 0.0, 0.0), node(2, 10.0, 10.0, 0.0), node(3, 0.0, 10.0, 0.0)];
        let stops = vec![0, 2, 1, 3];
        let (total_len, tour) = gp_tsp_tour_2opt(&nodes, &stops, 100_000).unwrap();
        assert_eq!(tour.len(), 5);
        assert_eq!(tour[0], tour[4]);
        assert!((total_len - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_kruskal_mst() {
        let edges = vec![
            edge(0, 0, 1, 1.0, 1.0, false), edge(1, 1, 2, 2.0, 1.0, false), edge(2, 2, 3, 3.0, 1.0, false),
            edge(3, 0, 3, 10.0, 1.0, false), edge(4, 0, 2, 5.0, 1.0, false),
        ];
        let (weight, mst_ids) = gp_minimum_spanning_tree_kruskal(4, &edges, 100_000).unwrap();
        assert!((weight - 6.0).abs() < 1e-6);
        assert_eq!(mst_ids, vec![0, 1, 2]);
    }

    #[test]
    fn test_max_flow_min_cut_and_centrality_and_budget() {
        let edges = vec![
            edge(0, 0, 1, 1.0, 10.0, true), edge(1, 0, 2, 1.0, 10.0, true), edge(2, 1, 2, 1.0, 2.0, true),
            edge(3, 1, 3, 1.0, 10.0, true), edge(4, 2, 3, 1.0, 10.0, true),
        ];
        let (max_flow, cut) = gp_max_flow_edmonds_karp(4, &edges, 0, 3, 100_000).unwrap();
        assert!((max_flow - 20.0).abs() < 1e-6);
        assert!(cut.contains(&0) && !cut.contains(&3));

        let star_edges = vec![edge(0, 0, 1, 1.0, 1.0, false), edge(1, 0, 2, 1.0, 1.0, false), edge(2, 0, 3, 1.0, 1.0, false)];
        let (close, betw) = gp_centrality_closeness_betweenness(4, &star_edges, 100_000).unwrap();
        assert!(close[0] > close[1] && (close[0] - 1.0).abs() < 1e-6 && (close[1] - 0.6).abs() < 1e-6);
        assert!((betw[0] - 3.0).abs() < 1e-6 && betw[1] == 0.0);

        let budget_err = gp_shortest_path_astar_turn_grade(&[node(0, 0.0, 0.0, 0.0)], &[], &[], 0, 0, 0.0, 10);
        assert!(matches!(budget_err, Err(GpNetworkError::ByteBudgetExceeded { .. })));
    }
}
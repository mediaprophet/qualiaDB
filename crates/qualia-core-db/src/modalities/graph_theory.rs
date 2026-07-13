//! Advanced graph theory analysis for NQuin graphs.
//!
//! This module exposes two graph-analysis tiers:
//! - `analyze_graph_topology_bounded` is the preferred zero-heap path for 10D tensor
//!   orchestration and edge inference. It uses fixed-capacity arrays only.
//! - `analyze_graph_topology` is the quarantined compatibility path for bounded,
//!   batch-style topology jobs. It still uses `HashMap`, `HashSet`, `Vec`, and
//!   `VecDeque`, so callers must keep it off hot paths and within the input cap below.

use crate::NQuin;
use std::collections::{HashMap, HashSet};

/// Heap-backed graph analysis is quarantined behind a fixed input cap so daemon
/// callers do not accidentally fan out into unbounded topology jobs on edge nodes.
pub const MAX_HEAP_GRAPH_ANALYSIS_QUINS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphAnalysisError {
    InputTooLarge,
    NodeCapacityExceeded,
    OutputBufferFull,
}

/// Preferred zero-heap node cap for edge-safe topology analysis.
pub const MAX_BOUNDED_GRAPH_ANALYSIS_NODES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommunitySpan {
    pub start: u16,
    pub len: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopNodeScore {
    pub node_id: u64,
    pub centrality_score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotifRecord {
    pub pattern: MotifPattern,
    pub node_a: u64,
    pub node_b: u64,
    pub node_c: u64,
    pub frequency: f32,
}

/// Maximum number of nodes in a subgraph-isomorphism query pattern. Bounding the
/// pattern keeps the backtracking search depth (and therefore the stack) fixed.
pub const MAX_SUBGRAPH_PATTERN_NODES: usize = 8;

/// A directed query pattern for subgraph isomorphism. `adjacency[i][j] != 0`
/// requires a data-graph edge from the node mapped to pattern node `i` to the
/// node mapped to pattern node `j`. Only the first `node_count` rows/cols are read.
#[derive(Debug, Clone, Copy)]
pub struct SubgraphPattern {
    pub adjacency: [[u8; MAX_SUBGRAPH_PATTERN_NODES]; MAX_SUBGRAPH_PATTERN_NODES],
    pub node_count: usize,
}

impl SubgraphPattern {
    /// Build an empty (edgeless) pattern of `node_count` nodes.
    pub fn new(node_count: usize) -> Self {
        Self {
            adjacency: [[0; MAX_SUBGRAPH_PATTERN_NODES]; MAX_SUBGRAPH_PATTERN_NODES],
            node_count: node_count.min(MAX_SUBGRAPH_PATTERN_NODES),
        }
    }

    /// Add a required directed edge `from -> to` to the pattern.
    pub fn with_edge(mut self, from: usize, to: usize) -> Self {
        if from < self.node_count && to < self.node_count {
            self.adjacency[from][to] = 1;
        }
        self
    }
}

/// One subgraph-isomorphism match: the data-graph node ids assigned to pattern
/// nodes `0..len`. `missing_edges` is 0 for an exact (induced-monomorphism)
/// match and counts unsatisfied pattern edges for an approximate match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubgraphMatch {
    pub mapping: [u64; MAX_SUBGRAPH_PATTERN_NODES],
    pub len: u16,
    pub missing_edges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedGraphAnalysisSummary {
    pub density: f32,
    pub node_count: u16,
    pub edge_count: u16,
    pub community_count: u16,
    pub motif_count: u16,
    pub top_node_count: u16,
    pub graph_quin_count: u16,
}

#[derive(Debug, Clone)]
struct BoundedQualiaGraph {
    node_ids: [u64; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
    centrality_scores: [f32; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
    degrees: [u16; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
    adjacency: [[u8; MAX_BOUNDED_GRAPH_ANALYSIS_NODES]; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
    node_count: usize,
    edge_count: usize,
}

impl BoundedQualiaGraph {
    fn from_quins(quins: &[NQuin]) -> Result<Self, GraphAnalysisError> {
        let mut graph = Self {
            node_ids: [0; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
            centrality_scores: [0.0; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
            degrees: [0; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
            adjacency: [[0; MAX_BOUNDED_GRAPH_ANALYSIS_NODES]; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
            node_count: 0,
            edge_count: 0,
        };

        for quin in quins {
            let source = graph.get_or_insert_node(quin.subject)?;
            let target = graph.get_or_insert_node(quin.object)?;
            if graph.adjacency[source][target] == 0 {
                graph.adjacency[source][target] = 1;
                graph.degrees[source] = graph.degrees[source].saturating_add(1);
                graph.edge_count += 1;
            }
        }

        Ok(graph)
    }

    fn get_or_insert_node(&mut self, node_id: u64) -> Result<usize, GraphAnalysisError> {
        for index in 0..self.node_count {
            if self.node_ids[index] == node_id {
                return Ok(index);
            }
        }

        if self.node_count >= MAX_BOUNDED_GRAPH_ANALYSIS_NODES {
            return Err(GraphAnalysisError::NodeCapacityExceeded);
        }

        let index = self.node_count;
        self.node_ids[index] = node_id;
        self.node_count += 1;
        Ok(index)
    }

    fn calculate_betweenness_centrality(&mut self) {
        let n = self.node_count;
        if n < 2 {
            return;
        }

        let mut scores = [0f32; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut sigma = [0f32; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut dist = [-1i16; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut delta = [0f32; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut queue = [0usize; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut stack = [0usize; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];

        for source in 0..n {
            sigma[..n].fill(0.0);
            dist[..n].fill(-1);
            delta[..n].fill(0.0);

            let mut queue_head = 0usize;
            let mut queue_tail = 0usize;
            let mut stack_len = 0usize;

            sigma[source] = 1.0;
            dist[source] = 0;
            queue[queue_tail] = source;
            queue_tail += 1;

            while queue_head < queue_tail {
                let v = queue[queue_head];
                queue_head += 1;
                stack[stack_len] = v;
                stack_len += 1;

                for w in 0..n {
                    if self.adjacency[v][w] == 0 {
                        continue;
                    }
                    if dist[w] < 0 {
                        dist[w] = dist[v] + 1;
                        queue[queue_tail] = w;
                        queue_tail += 1;
                    }
                    if dist[w] == dist[v] + 1 {
                        sigma[w] += sigma[v];
                    }
                }
            }

            while stack_len > 0 {
                stack_len -= 1;
                let w = stack[stack_len];
                for v in 0..n {
                    if self.adjacency[v][w] == 0 || dist[v] != dist[w] - 1 || sigma[w] == 0.0 {
                        continue;
                    }
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
                if w != source {
                    scores[w] += delta[w];
                }
            }
        }

        let norm = if n > 2 {
            ((n - 1) * (n - 2)) as f32
        } else {
            1.0
        };
        for (index, score) in scores.iter().take(n).enumerate() {
            self.centrality_scores[index] = if norm > 0.0 { *score / norm } else { *score };
        }
    }

    fn density(&self) -> f32 {
        if self.node_count < 2 {
            return 0.0;
        }
        let possible_edges = (self.node_count * (self.node_count - 1)) as f32;
        self.edge_count as f32 / possible_edges
    }

    fn write_communities(
        &self,
        community_nodes_out: &mut [u64],
        community_spans_out: &mut [CommunitySpan],
    ) -> Result<usize, GraphAnalysisError> {
        let mut visited = [false; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut stack = [0usize; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut nodes_written = 0usize;
        let mut communities_written = 0usize;

        for start in 0..self.node_count {
            if visited[start] {
                continue;
            }
            if communities_written >= community_spans_out.len() {
                return Err(GraphAnalysisError::OutputBufferFull);
            }
            let span_start = nodes_written;
            let mut stack_len = 0usize;
            stack[stack_len] = start;
            stack_len += 1;
            visited[start] = true;

            while stack_len > 0 {
                stack_len -= 1;
                let node = stack[stack_len];
                if nodes_written >= community_nodes_out.len() {
                    return Err(GraphAnalysisError::OutputBufferFull);
                }
                community_nodes_out[nodes_written] = self.node_ids[node];
                nodes_written += 1;

                for neighbor in 0..self.node_count {
                    let linked =
                        self.adjacency[node][neighbor] != 0 || self.adjacency[neighbor][node] != 0;
                    if linked && !visited[neighbor] {
                        visited[neighbor] = true;
                        stack[stack_len] = neighbor;
                        stack_len += 1;
                    }
                }
            }

            community_spans_out[communities_written] = CommunitySpan {
                start: span_start as u16,
                len: (nodes_written - span_start) as u16,
            };
            communities_written += 1;
        }

        Ok(communities_written)
    }

    fn write_motifs(&self, motifs_out: &mut [MotifRecord]) -> Result<usize, GraphAnalysisError> {
        let mut written = 0usize;

        for a in 0..self.node_count {
            for b in 0..self.node_count {
                if self.adjacency[a][b] == 0 {
                    continue;
                }
                for c in 0..self.node_count {
                    if c == a || self.adjacency[b][c] == 0 || self.adjacency[c][a] == 0 {
                        continue;
                    }

                    let mut canonical = [self.node_ids[a], self.node_ids[b], self.node_ids[c]];
                    canonical.sort_unstable();
                    if motifs_out[..written].iter().any(|m| {
                        let mut existing = [m.node_a, m.node_b, m.node_c];
                        existing.sort_unstable();
                        existing == canonical
                    }) {
                        continue;
                    }

                    if written >= motifs_out.len() {
                        return Err(GraphAnalysisError::OutputBufferFull);
                    }
                    motifs_out[written] = MotifRecord {
                        pattern: MotifPattern::Triangle,
                        node_a: self.node_ids[a],
                        node_b: self.node_ids[b],
                        node_c: self.node_ids[c],
                        frequency: 1.0,
                    };
                    written += 1;
                }
            }
        }

        Ok(written)
    }

    fn write_top_nodes(&self, out: &mut [TopNodeScore]) -> usize {
        let count = out.len().min(self.node_count);
        if count == 0 {
            return 0;
        }

        let mut scratch = [TopNodeScore {
            node_id: 0,
            centrality_score: f32::MIN,
        }; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        for i in 0..self.node_count {
            scratch[i] = TopNodeScore {
                node_id: self.node_ids[i],
                centrality_score: self.centrality_scores[i],
            };
        }
        scratch[..self.node_count].sort_by(|a, b| {
            b.centrality_score
                .partial_cmp(&a.centrality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out[..count].copy_from_slice(&scratch[..count]);
        count
    }

    fn write_graph_quins(
        &self,
        context: u64,
        out: &mut [NQuin],
    ) -> Result<usize, GraphAnalysisError> {
        if out.len() < self.node_count {
            return Err(GraphAnalysisError::OutputBufferFull);
        }

        for i in 0..self.node_count {
            let mut quin = NQuin {
                subject: self.node_ids[i],
                predicate: crate::q_hash("has_centrality_score"),
                object: (self.centrality_scores[i] as f64 * 1000.0) as u64,
                context,
                metadata: self.degrees[i] as u64,
                parity: 0,
            };
            quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
            out[i] = quin;
        }

        Ok(self.node_count)
    }

    /// Zero-heap PageRank via power iteration over the bounded adjacency matrix.
    ///
    /// Google PageRank: `PR(v) = (1-d)/N + d·Σ_{u→v} PR(u)/outdeg(u)`, with the
    /// mass held by dangling (out-degree-0) nodes redistributed uniformly so the
    /// vector stays a probability distribution. Iterates until the L1 delta drops
    /// below `tolerance` or `max_iters` is reached. Writes `min(out.len(), n)`
    /// scores aligned to `self.node_ids` order and returns that count.
    fn calculate_pagerank(
        &self,
        damping: f32,
        max_iters: u32,
        tolerance: f32,
        out: &mut [f32],
    ) -> usize {
        let n = self.node_count;
        if n == 0 {
            return 0;
        }
        let count = out.len().min(n);

        let mut rank = [0f32; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut next = [0f32; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut outdeg = [0u16; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];

        let init = 1.0 / n as f32;
        rank[..n].fill(init);
        for v in 0..n {
            let mut d = 0u16;
            for w in 0..n {
                if self.adjacency[v][w] != 0 {
                    d += 1;
                }
            }
            outdeg[v] = d;
        }

        let base = (1.0 - damping) / n as f32;
        for _ in 0..max_iters {
            let mut dangling = 0f32;
            for v in 0..n {
                if outdeg[v] == 0 {
                    dangling += rank[v];
                }
            }
            let dangling_share = damping * dangling / n as f32;
            next[..n].fill(base + dangling_share);

            for v in 0..n {
                if outdeg[v] == 0 {
                    continue;
                }
                let share = damping * rank[v] / outdeg[v] as f32;
                for w in 0..n {
                    if self.adjacency[v][w] != 0 {
                        next[w] += share;
                    }
                }
            }

            let mut delta = 0f32;
            for v in 0..n {
                delta += (next[v] - rank[v]).abs();
                rank[v] = next[v];
            }
            if delta < tolerance {
                break;
            }
        }

        out[..count].copy_from_slice(&rank[..count]);
        count
    }

    /// Exact / approximate subgraph isomorphism by bounded backtracking search.
    ///
    /// Finds injective mappings of the query `pattern` onto the data graph such
    /// that every required pattern edge maps to a data edge (a directed
    /// monomorphism). With `max_missing_edges > 0` the search also reports
    /// approximate matches that miss up to that many required edges (recorded in
    /// `SubgraphMatch::missing_edges`). Recursion depth is bounded by the pattern
    /// size, so the call uses only fixed-size stack frames — no heap.
    fn find_subgraph_isomorphisms(
        &self,
        pattern: &SubgraphPattern,
        max_missing_edges: u8,
        out: &mut [SubgraphMatch],
    ) -> usize {
        if pattern.node_count == 0
            || pattern.node_count > MAX_SUBGRAPH_PATTERN_NODES
            || pattern.node_count > self.node_count
            || out.is_empty()
        {
            return 0;
        }
        let mut assign = [0usize; MAX_SUBGRAPH_PATTERN_NODES];
        let mut used = [false; MAX_BOUNDED_GRAPH_ANALYSIS_NODES];
        let mut written = 0usize;
        self.match_subgraph(
            pattern,
            0,
            0,
            max_missing_edges,
            &mut assign,
            &mut used,
            out,
            &mut written,
        );
        written
    }

    #[allow(clippy::too_many_arguments)]
    fn match_subgraph(
        &self,
        pattern: &SubgraphPattern,
        depth: usize,
        missing: u8,
        max_missing_edges: u8,
        assign: &mut [usize; MAX_SUBGRAPH_PATTERN_NODES],
        used: &mut [bool; MAX_BOUNDED_GRAPH_ANALYSIS_NODES],
        out: &mut [SubgraphMatch],
        written: &mut usize,
    ) {
        if *written >= out.len() {
            return;
        }
        if depth == pattern.node_count {
            let mut m = SubgraphMatch {
                mapping: [0; MAX_SUBGRAPH_PATTERN_NODES],
                len: pattern.node_count as u16,
                missing_edges: missing,
            };
            for (p, slot) in m.mapping.iter_mut().take(pattern.node_count).enumerate() {
                *slot = self.node_ids[assign[p]];
            }
            out[*written] = m;
            *written += 1;
            return;
        }

        for candidate in 0..self.node_count {
            if used[candidate] {
                continue;
            }
            // Edge-consistency against already-assigned pattern nodes.
            let mut local_missing = 0u8;
            for prev in 0..depth {
                let pd = assign[prev];
                if pattern.adjacency[prev][depth] != 0 && self.adjacency[pd][candidate] == 0 {
                    local_missing += 1;
                }
                if pattern.adjacency[depth][prev] != 0 && self.adjacency[candidate][pd] == 0 {
                    local_missing += 1;
                }
            }
            if missing + local_missing > max_missing_edges {
                continue;
            }
            assign[depth] = candidate;
            used[candidate] = true;
            self.match_subgraph(
                pattern,
                depth + 1,
                missing + local_missing,
                max_missing_edges,
                assign,
                used,
                out,
                written,
            );
            used[candidate] = false;
            if *written >= out.len() {
                return;
            }
        }
    }
}

/// Zero-heap topology analysis aligned with the 10D tensor hot-path constraints.
pub fn analyze_graph_topology_bounded(
    quins: &[NQuin],
    context: u64,
    graph_quins_out: &mut [NQuin],
    community_nodes_out: &mut [u64],
    community_spans_out: &mut [CommunitySpan],
    motifs_out: &mut [MotifRecord],
    top_nodes_out: &mut [TopNodeScore],
) -> Result<BoundedGraphAnalysisSummary, GraphAnalysisError> {
    if quins.len() > MAX_HEAP_GRAPH_ANALYSIS_QUINS {
        return Err(GraphAnalysisError::InputTooLarge);
    }

    let mut graph = BoundedQualiaGraph::from_quins(quins)?;
    graph.calculate_betweenness_centrality();

    let graph_quin_count = graph.write_graph_quins(context, graph_quins_out)?;
    let community_count = graph.write_communities(community_nodes_out, community_spans_out)?;
    let motif_count = graph.write_motifs(motifs_out)?;
    let top_node_count = graph.write_top_nodes(top_nodes_out);

    Ok(BoundedGraphAnalysisSummary {
        density: graph.density(),
        node_count: graph.node_count as u16,
        edge_count: graph.edge_count as u16,
        community_count: community_count as u16,
        motif_count: motif_count as u16,
        top_node_count: top_node_count as u16,
        graph_quin_count: graph_quin_count as u16,
    })
}

/// Zero-heap PageRank over an NQuin relation set (bounded path).
///
/// Builds the bounded adjacency from `quins`, runs power iteration with the given
/// `damping` (typically 0.85), and writes node scores into `scores_out` aligned
/// with first-seen subject/object order. Returns the number of scores written.
/// `max_iters`/`tolerance` bound the iteration (e.g. 100 / 1e-6).
pub fn pagerank_bounded(
    quins: &[NQuin],
    damping: f32,
    max_iters: u32,
    tolerance: f32,
    node_ids_out: &mut [u64],
    scores_out: &mut [f32],
) -> Result<usize, GraphAnalysisError> {
    if quins.len() > MAX_HEAP_GRAPH_ANALYSIS_QUINS {
        return Err(GraphAnalysisError::InputTooLarge);
    }
    let graph = BoundedQualiaGraph::from_quins(quins)?;
    let count = graph.calculate_pagerank(damping, max_iters, tolerance, scores_out);
    if node_ids_out.len() < count {
        return Err(GraphAnalysisError::OutputBufferFull);
    }
    node_ids_out[..count].copy_from_slice(&graph.node_ids[..count]);
    Ok(count)
}

/// Zero-heap exact/approximate subgraph isomorphism over an NQuin relation set.
///
/// `max_missing_edges == 0` requires an exact directed monomorphism; a positive
/// value also returns approximate matches missing up to that many pattern edges.
/// Matches are written into `matches_out` (search stops when it fills). Returns
/// the number of matches found.
pub fn find_subgraph_isomorphisms_bounded(
    quins: &[NQuin],
    pattern: &SubgraphPattern,
    max_missing_edges: u8,
    matches_out: &mut [SubgraphMatch],
) -> Result<usize, GraphAnalysisError> {
    if quins.len() > MAX_HEAP_GRAPH_ANALYSIS_QUINS {
        return Err(GraphAnalysisError::InputTooLarge);
    }
    let graph = BoundedQualiaGraph::from_quins(quins)?;
    Ok(graph.find_subgraph_isomorphisms(pattern, max_missing_edges, matches_out))
}

/// Graph structure built from NQuin relations
#[derive(Debug, Clone)]
pub struct QualiaGraph {
    pub nodes: HashMap<u64, GraphNode>,
    pub edges: HashMap<(u64, u64), GraphEdge>,
    pub adjacency_list: HashMap<u64, Vec<u64>>,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: u64,
    pub degree: usize,
    pub centrality_score: f64,
    pub community_id: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source: u64,
    pub target: u64,
    pub weight: f64,
}

impl QualiaGraph {
    /// Create a new graph from NQuin relations
    pub fn from_quins(quins: &[NQuin]) -> Self {
        let mut nodes = HashMap::new();
        let mut edges = HashMap::new();
        let mut adjacency_list = HashMap::new();

        // Build graph structure
        for quin in quins {
            // Add source node
            nodes.entry(quin.subject).or_insert_with(|| GraphNode {
                id: quin.subject,
                degree: 0,
                centrality_score: 0.0,
                community_id: None,
            });

            // Add target node
            nodes.entry(quin.object).or_insert_with(|| GraphNode {
                id: quin.object,
                degree: 0,
                centrality_score: 0.0,
                community_id: None,
            });

            // Add edge
            let edge = GraphEdge {
                source: quin.subject,
                target: quin.object,
                weight: 1.0, // Default weight
            };
            edges.insert((quin.subject, quin.object), edge);

            // Update adjacency list
            adjacency_list
                .entry(quin.subject)
                .or_insert_with(Vec::new)
                .push(quin.object);

            // Update degrees
            if let Some(node) = nodes.get_mut(&quin.subject) {
                node.degree += 1;
            }
        }

        Self {
            nodes,
            edges,
            adjacency_list,
        }
    }

    /// Calculate betweenness centrality for all nodes
    pub fn calculate_betweenness_centrality(&mut self) {
        // Brandes' algorithm for betweenness centrality (directed graph)
        // sigma[v] = number of shortest paths from source to v
        // dist[v]  = BFS distance from source to v (-1 = unvisited)
        // delta[v] = dependency of source on v
        let node_ids: Vec<u64> = self.nodes.keys().cloned().collect();
        let mut scores: HashMap<u64, f64> = node_ids.iter().map(|&id| (id, 0.0)).collect();

        for &source in &node_ids {
            let mut sigma: HashMap<u64, f64> = node_ids.iter().map(|&id| (id, 0.0)).collect();
            let mut dist: HashMap<u64, i64> = node_ids.iter().map(|&id| (id, -1)).collect();
            // predecessors[v] = list of nodes w on a shortest path to v
            let mut pred: HashMap<u64, Vec<u64>> =
                node_ids.iter().map(|&id| (id, Vec::new())).collect();
            let mut stack: Vec<u64> = Vec::new();
            // FIFO queue for BFS
            let mut queue: std::collections::VecDeque<u64> = std::collections::VecDeque::new();

            sigma.insert(source, 1.0);
            dist.insert(source, 0);
            queue.push_back(source);

            // BFS phase
            while let Some(v) = queue.pop_front() {
                stack.push(v);
                let v_dist = dist[&v];
                if let Some(neighbors) = self.adjacency_list.get(&v) {
                    for &w in neighbors {
                        // First time visiting w?
                        if dist[&w] < 0 {
                            queue.push_back(w);
                            dist.insert(w, v_dist + 1);
                        }
                        // Is v on a shortest path to w?
                        if dist[&w] == v_dist + 1 {
                            *sigma.get_mut(&w).unwrap() += sigma[&v];
                            pred.get_mut(&w).unwrap().push(v);
                        }
                    }
                }
            }

            // Accumulation phase (back-propagation)
            let mut delta: HashMap<u64, f64> = node_ids.iter().map(|&id| (id, 0.0)).collect();
            while let Some(w) = stack.pop() {
                for &v in &pred[&w] {
                    let coeff = (sigma[&v] / sigma[&w]) * (1.0 + delta[&w]);
                    *delta.get_mut(&v).unwrap() += coeff;
                }
                if w != source {
                    *scores.get_mut(&w).unwrap() += delta[&w];
                }
            }
        }

        // Normalize scores (directed graph: divide by (n-1)(n-2))
        let n = self.nodes.len() as f64;
        if n > 2.0 {
            let norm = (n - 1.0) * (n - 2.0);
            for score in scores.values_mut() {
                *score /= norm;
            }
        }

        // Update node centrality scores
        for (node_id, score) in scores {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.centrality_score = score;
            }
        }
    }

    /// Detect communities using simple modularity optimization
    pub fn detect_communities(&mut self) -> Vec<Vec<u64>> {
        let mut communities: Vec<Vec<u64>> =
            self.nodes.keys().cloned().map(|id| vec![id]).collect();
        let mut improved = true;

        while improved {
            improved = false;

            for i in 0..communities.len() {
                if i >= communities.len() {
                    break;
                }

                let current_community = communities[i].clone();
                let best_move = self.find_best_community_move(&current_community, &communities, i);

                if let Some((target_community, modularity_gain)) = best_move {
                    if modularity_gain > 0.0 {
                        // Move nodes to target community
                        communities[target_community].extend(&current_community);
                        communities.remove(i);
                        improved = true;
                        break;
                    }
                }
            }
        }

        // Update community IDs in nodes
        for (community_id, community) in communities.iter().enumerate() {
            for &node_id in community {
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    node.community_id = Some(community_id);
                }
            }
        }

        communities
    }

    /// Find best community move for modularity optimization
    fn find_best_community_move(
        &self,
        community: &[u64],
        all_communities: &[Vec<u64>],
        current_index: usize,
    ) -> Option<(usize, f64)> {
        let mut best_move = None;
        let mut best_gain = 0.0;

        for (i, other_community) in all_communities.iter().enumerate() {
            if i == current_index {
                continue;
            }

            let gain = self.calculate_modularity_gain(community, other_community);
            if gain > best_gain {
                best_gain = gain;
                best_move = Some((i, gain));
            }
        }

        best_move
    }

    /// Calculate modularity gain for merging two communities (Louvain delta-Q).
    ///
    /// ΔQ = e_ij/m  −  (a_i × a_j) / (2m²)
    /// where e_ij = edges crossing between comm1 and comm2,
    ///       a_x  = sum of degrees of nodes in community x,
    ///       m    = total number of edges in the graph.
    fn calculate_modularity_gain(&self, comm1: &[u64], comm2: &[u64]) -> f64 {
        let m = self.edges.len() as f64;
        if m == 0.0 {
            return 0.0;
        }

        let set1: HashSet<u64> = comm1.iter().cloned().collect();
        let set2: HashSet<u64> = comm2.iter().cloned().collect();

        // Count edges crossing from comm1 to comm2 or comm2 to comm1
        let mut e_ij = 0.0;
        for &(source, target) in self.edges.keys() {
            if (set1.contains(&source) && set2.contains(&target))
                || (set2.contains(&source) && set1.contains(&target))
            {
                e_ij += 1.0;
            }
        }

        // Degree sums
        let a1: f64 = comm1
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .map(|n| n.degree as f64)
            .sum();
        let a2: f64 = comm2
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .map(|n| n.degree as f64)
            .sum();

        (e_ij / m) - (a1 * a2) / (2.0 * m * m)
    }

    /// Find common motifs (3-node patterns)
    pub fn find_motifs(&self) -> Vec<Motif> {
        // Deduplicate triangles by canonical sorted triple so that each
        // undirected triangle (regardless of orientation in the directed graph)
        // is reported exactly once.
        let mut seen: HashSet<[u64; 3]> = HashSet::new();
        let mut motifs = Vec::new();

        for &node_a in self.nodes.keys() {
            if let Some(neighbors_a) = self.adjacency_list.get(&node_a) {
                for &node_b in neighbors_a {
                    if let Some(neighbors_b) = self.adjacency_list.get(&node_b) {
                        for &node_c in neighbors_b {
                            if node_c != node_a {
                                // Check if this forms a triangle motif (directed cycle a→b→c→a)
                                if let Some(neighbors_c) = self.adjacency_list.get(&node_c) {
                                    if neighbors_c.contains(&node_a) {
                                        // Build canonical key: sorted triple
                                        let mut key = [node_a, node_b, node_c];
                                        key.sort_unstable();
                                        if seen.insert(key) {
                                            motifs.push(Motif {
                                                pattern: MotifPattern::Triangle,
                                                nodes: vec![node_a, node_b, node_c],
                                                frequency: 1.0,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        motifs
    }

    /// Get top nodes by centrality score
    pub fn get_top_central_nodes(&self, top_n: usize) -> Vec<(u64, f64)> {
        let mut nodes: Vec<(u64, f64)> = self
            .nodes
            .iter()
            .map(|(id, node)| (*id, node.centrality_score))
            .collect();

        nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        nodes.truncate(top_n);
        nodes
    }

    /// Calculate graph density
    pub fn density(&self) -> f64 {
        let n = self.nodes.len();
        if n < 2 {
            return 0.0;
        }

        let possible_edges = n * (n - 1);
        self.edges.len() as f64 / possible_edges as f64
    }

    /// Convert graph state to NQuin for storage
    pub fn to_quins(&self, context: u64) -> Vec<NQuin> {
        let mut quins = Vec::new();

        // Store node centrality scores
        for (node_id, node) in &self.nodes {
            let mut quin = NQuin {
                subject: *node_id,
                predicate: crate::q_hash("has_centrality_score"),
                object: (node.centrality_score * 1000.0) as u64, // Store as scaled integer
                context,
                metadata: 0,
                parity: 0,
            };

            // Store degree in metadata
            quin.metadata = node.degree as u64;
            quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
            quins.push(quin);
        }

        quins
    }
}

/// Motif pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifPattern {
    Triangle,
    Chain,
    Star,
    Fork,
}

/// Graph motif representation
#[derive(Debug, Clone)]
pub struct Motif {
    pub pattern: MotifPattern,
    pub nodes: Vec<u64>,
    pub frequency: f64,
}

/// Analyze graph topology in a bounded, heap-backed batch.
pub fn analyze_graph_topology(
    quins: &[NQuin],
    context: u64,
) -> Result<GraphAnalysisResult, GraphAnalysisError> {
    if quins.len() > MAX_HEAP_GRAPH_ANALYSIS_QUINS {
        return Err(GraphAnalysisError::InputTooLarge);
    }

    let mut graph = QualiaGraph::from_quins(quins);

    // Calculate centrality
    graph.calculate_betweenness_centrality();

    // Detect communities
    let communities = graph.detect_communities();

    // Find motifs
    let motifs = graph.find_motifs();

    // Get top central nodes
    let top_nodes = graph.get_top_central_nodes(10);

    // Calculate density
    let density = graph.density();

    // Convert to quins for storage
    let graph_quins = graph.to_quins(context);

    Ok(GraphAnalysisResult {
        graph_quins,
        communities,
        motifs,
        top_nodes,
        density,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
    })
}

/// Result of graph analysis
#[derive(Debug, Clone)]
pub struct GraphAnalysisResult {
    pub graph_quins: Vec<NQuin>,
    pub communities: Vec<Vec<u64>>,
    pub motifs: Vec<Motif>,
    pub top_nodes: Vec<(u64, f64)>,
    pub density: f64,
    pub node_count: usize,
    pub edge_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: crate::q_hash("connects_to"),
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: crate::q_hash("connects_to"),
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 1,
                predicate: crate::q_hash("connects_to"),
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let graph = QualiaGraph::from_quins(&quins);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(graph.adjacency_list.get(&1).unwrap().len(), 2);
    }

    #[test]
    fn test_centrality_calculation() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let mut graph = QualiaGraph::from_quins(&quins);
        graph.calculate_betweenness_centrality();

        // Node 2 should have highest betweenness because it is the only bridge from 1 to 3.
        let node2_centrality = graph.nodes.get(&2).unwrap().centrality_score;
        let node1_centrality = graph.nodes.get(&1).unwrap().centrality_score;

        assert!(node2_centrality > node1_centrality);
    }

    #[test]
    fn test_community_detection() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 1,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 4,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 4,
                predicate: 1,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let mut graph = QualiaGraph::from_quins(&quins);
        let communities = graph.detect_communities();

        // Should detect two separate communities
        assert_eq!(communities.len(), 2);
    }

    #[test]
    fn test_motif_detection() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 1,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let graph = QualiaGraph::from_quins(&quins);
        let motifs = graph.find_motifs();

        // Should detect one triangle motif
        assert_eq!(motifs.len(), 1);
        assert_eq!(motifs[0].pattern, MotifPattern::Triangle);
    }

    #[test]
    fn test_graph_analysis() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 1,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];

        let result = analyze_graph_topology(&quins, 100).unwrap();

        assert_eq!(result.node_count, 3);
        assert_eq!(result.edge_count, 3);
        assert!(result.density > 0.0);
        assert!(!result.communities.is_empty());
    }

    #[test]
    fn test_graph_analysis_rejects_oversized_batches() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            };
            MAX_HEAP_GRAPH_ANALYSIS_QUINS + 1
        ];

        let result = analyze_graph_topology(&quins, 100);

        assert!(matches!(result, Err(GraphAnalysisError::InputTooLarge)));
    }

    #[test]
    fn test_bounded_graph_analysis_zero_heap_path() {
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 100,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 1,
                context: 100,
                metadata: 0,
                parity: 0,
            },
        ];
        let mut graph_quins = [NQuin::default(); 16];
        let mut community_nodes = [0u64; 16];
        let mut community_spans = [CommunitySpan { start: 0, len: 0 }; 8];
        let mut motifs = [MotifRecord {
            pattern: MotifPattern::Triangle,
            node_a: 0,
            node_b: 0,
            node_c: 0,
            frequency: 0.0,
        }; 8];
        let mut top_nodes = [TopNodeScore {
            node_id: 0,
            centrality_score: 0.0,
        }; 8];

        let summary = analyze_graph_topology_bounded(
            &quins,
            100,
            &mut graph_quins,
            &mut community_nodes,
            &mut community_spans,
            &mut motifs,
            &mut top_nodes,
        )
        .unwrap();

        assert_eq!(summary.node_count, 3);
        assert_eq!(summary.edge_count, 3);
        assert_eq!(summary.community_count, 1);
        assert_eq!(summary.motif_count, 1);
        assert!(summary.top_node_count >= 1);
        assert!(summary.density > 0.0);
    }

    #[test]
    fn test_pagerank_sums_to_one_and_ranks_hub() {
        // Star: 1→3, 2→3, 4→3 (node 3 is the sink hub) plus 3→1 to avoid a pure dangling.
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 4,
                predicate: 1,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 1,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];
        let mut ids = [0u64; 16];
        let mut scores = [0f32; 16];
        let count = pagerank_bounded(&quins, 0.85, 100, 1e-6, &mut ids, &mut scores).unwrap();
        assert_eq!(count, 4);

        // Probability distribution: scores sum to ~1.
        let total: f32 = scores[..count].iter().sum();
        assert!(
            (total - 1.0).abs() < 1e-3,
            "pagerank should sum to 1, got {total}"
        );

        // Node 3 (the hub everyone points at) must rank highest.
        let hub_pos = ids[..count].iter().position(|&id| id == 3).unwrap();
        let hub_score = scores[hub_pos];
        for i in 0..count {
            if i != hub_pos {
                assert!(
                    hub_score > scores[i],
                    "hub node 3 should outrank node {}",
                    ids[i]
                );
            }
        }
    }

    #[test]
    fn test_subgraph_isomorphism_exact_triangle() {
        // Data graph contains a directed triangle 1→2→3→1 plus an extra edge.
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 1,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 3,
                predicate: 1,
                object: 4,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];
        // Pattern: directed 3-cycle a→b→c→a.
        let pattern = SubgraphPattern::new(3)
            .with_edge(0, 1)
            .with_edge(1, 2)
            .with_edge(2, 0);
        let mut matches = [SubgraphMatch {
            mapping: [0; MAX_SUBGRAPH_PATTERN_NODES],
            len: 0,
            missing_edges: 0,
        }; 16];
        let n = find_subgraph_isomorphisms_bounded(&quins, &pattern, 0, &mut matches).unwrap();

        // The directed 3-cycle has exactly 3 rotations of the same triangle.
        assert_eq!(n, 3, "expected 3 rotations of the directed triangle");
        for m in &matches[..n] {
            assert_eq!(m.missing_edges, 0);
            assert_eq!(m.len, 3);
            // every matched node id is one of {1,2,3}, never the extra node 4
            for &id in &m.mapping[..3] {
                assert!(
                    id == 1 || id == 2 || id == 3,
                    "match included non-triangle node {id}"
                );
            }
        }
    }

    #[test]
    fn test_subgraph_isomorphism_approximate_allows_missing_edge() {
        // Data graph has a path 1→2→3 but NOT the closing edge 3→1.
        let quins = vec![
            NQuin {
                subject: 1,
                predicate: 1,
                object: 2,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 2,
                predicate: 1,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];
        let triangle = SubgraphPattern::new(3)
            .with_edge(0, 1)
            .with_edge(1, 2)
            .with_edge(2, 0);
        let mut matches = [SubgraphMatch {
            mapping: [0; MAX_SUBGRAPH_PATTERN_NODES],
            len: 0,
            missing_edges: 0,
        }; 16];

        // Exact: no triangle exists.
        let exact = find_subgraph_isomorphisms_bounded(&quins, &triangle, 0, &mut matches).unwrap();
        assert_eq!(exact, 0);

        // Approximate (allow 1 missing edge): the open path 1→2→3 matches with the
        // 3→1 edge absent.
        let approx =
            find_subgraph_isomorphisms_bounded(&quins, &triangle, 1, &mut matches).unwrap();
        assert!(approx >= 1, "approximate match should find the open path");
        assert!(matches[..approx].iter().any(|m| {
            m.missing_edges == 1 && m.mapping[0] == 1 && m.mapping[1] == 2 && m.mapping[2] == 3
        }));
    }
}

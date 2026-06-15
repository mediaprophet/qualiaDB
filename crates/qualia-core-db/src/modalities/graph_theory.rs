//! Advanced graph theory analysis for NQuin graphs.
//!
//! This module is intentionally heap-backed and meant for bounded, batch-style
//! topology analysis rather than hot-path inference. It uses `HashMap`, `HashSet`,
//! `Vec`, and `VecDeque` internally, so callers should keep analysis inputs within
//! the explicit guardrails below and avoid invoking it from zero-heap execution loops.

use crate::NQuin;
use std::collections::{HashMap, HashSet};

/// Heap-backed graph analysis is quarantined behind a fixed input cap so daemon
/// callers do not accidentally fan out into unbounded topology jobs on edge nodes.
pub const MAX_HEAP_GRAPH_ANALYSIS_QUINS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphAnalysisError {
    InputTooLarge,
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
            adjacency_list.entry(quin.subject).or_insert_with(Vec::new).push(quin.object);
            
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
            let mut pred: HashMap<u64, Vec<u64>> = node_ids.iter().map(|&id| (id, Vec::new())).collect();
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
        let mut communities: Vec<Vec<u64>> = self.nodes.keys().cloned().map(|id| vec![id]).collect();
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
    fn find_best_community_move(&self, community: &[u64], all_communities: &[Vec<u64>], current_index: usize) -> Option<(usize, f64)> {
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
        let a1: f64 = comm1.iter()
            .filter_map(|id| self.nodes.get(id))
            .map(|n| n.degree as f64)
            .sum();
        let a2: f64 = comm2.iter()
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
        let mut nodes: Vec<(u64, f64)> = self.nodes.iter()
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
#[derive(Debug, Clone, PartialEq)]
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
            NQuin { subject: 1, predicate: 1, object: 2, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 2, predicate: 1, object: 3, context: 100, metadata: 0, parity: 0 },
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
            NQuin { subject: 1, predicate: 1, object: 2, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 2, predicate: 1, object: 1, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 3, predicate: 1, object: 4, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 4, predicate: 1, object: 3, context: 100, metadata: 0, parity: 0 },
        ];
        
        let mut graph = QualiaGraph::from_quins(&quins);
        let communities = graph.detect_communities();
        
        // Should detect two separate communities
        assert_eq!(communities.len(), 2);
    }
    
    #[test]
    fn test_motif_detection() {
        let quins = vec![
            NQuin { subject: 1, predicate: 1, object: 2, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 2, predicate: 1, object: 3, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 3, predicate: 1, object: 1, context: 100, metadata: 0, parity: 0 },
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
            NQuin { subject: 1, predicate: 1, object: 2, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 2, predicate: 1, object: 3, context: 100, metadata: 0, parity: 0 },
            NQuin { subject: 3, predicate: 1, object: 1, context: 100, metadata: 0, parity: 0 },
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
}

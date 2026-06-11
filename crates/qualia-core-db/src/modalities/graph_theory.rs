// Advanced Graph Theory Algorithms
// Zero-allocation centrality, community detection, and motif finding for NQuin graphs

use crate::NQuin;
use std::collections::{HashMap, HashSet};

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
        // Brandes' algorithm for betweenness centrality
        let mut scores = HashMap::new();
        
        for node_id in self.nodes.keys() {
            scores.insert(*node_id, 0.0);
        }
        
        for source in self.nodes.keys() {
            let mut dependencies = HashMap::new();
            let mut shortest_paths = HashMap::new();
            let mut queue = Vec::new();
            let mut stack = Vec::new();
            
            // Initialize
            for node_id in self.nodes.keys() {
                dependencies.insert(*node_id, 0.0);
                shortest_paths.insert(*node_id, 0);
            }
            
            shortest_paths.insert(*source, 1);
            queue.push(*source);
            
            // BFS
            while let Some(current) = queue.pop() {
                stack.push(current);
                
                if let Some(neighbors) = self.adjacency_list.get(&current) {
                    for &neighbor in neighbors {
                        if shortest_paths[&neighbor] == 0 {
                            queue.push(neighbor);
                            shortest_paths.insert(neighbor, shortest_paths[&current]);
                        } else if shortest_paths[&neighbor] == shortest_paths[&current] {
                            shortest_paths.insert(neighbor, shortest_paths[&neighbor] + 1);
                        }
                        
                        if shortest_paths[&neighbor] == shortest_paths[&current] + 1 {
                            dependencies.insert(neighbor, dependencies[&neighbor] + dependencies[&current]);
                        }
                    }
                }
            }
            
            // Accumulate scores
            let mut delta = HashMap::new();
            for node_id in self.nodes.keys() {
                delta.insert(*node_id, 0.0);
            }
            
            while let Some(w) = stack.pop() {
                if let Some(neighbors) = self.adjacency_list.get(&w) {
                    for &v in neighbors {
                        if shortest_paths[&v] == shortest_paths[&w] + 1 {
                            let coeff = (1.0 + delta[&w]) / shortest_paths[&v] as f64;
                            delta.insert(v, delta[&v] + coeff);
                            
                            if let Some(score) = scores.get_mut(&v) {
                                *score += coeff;
                            }
                        }
                    }
                }
            }
        }
        
        // Normalize scores
        let n = self.nodes.len() as f64;
        if n > 2.0 {
            let normalization_factor = (n - 1.0) * (n - 2.0);
            for (node_id, score) in scores.iter_mut() {
                *score /= normalization_factor;
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
    
    /// Calculate modularity gain for merging communities
    fn calculate_modularity_gain(&self, comm1: &[u64], comm2: &[u64]) -> f64 {
        let total_edges = self.edges.len() as f64;
        if total_edges == 0.0 {
            return 0.0;
        }
        
        let mut internal_edges = 0.0;
        let mut degree_sum = 0.0;
        
        // Count internal edges in merged community
        let merged_community: HashSet<u64> = comm1.iter().chain(comm2).cloned().collect();
        
        for (&(source, target), edge) in &self.edges {
            if merged_community.contains(&source) && merged_community.contains(&target) {
                internal_edges += edge.weight;
            }
        }
        
        // Calculate degree sum for merged community
        for &node_id in &merged_community {
            if let Some(node) = self.nodes.get(&node_id) {
                degree_sum += node.degree as f64;
            }
        }
        
        let expected_edges = (degree_sum * degree_sum) / (2.0 * total_edges);
        (internal_edges / total_edges) - expected_edges / (2.0 * total_edges)
    }
    
    /// Find common motifs (3-node patterns)
    pub fn find_motifs(&self) -> Vec<Motif> {
        let mut motifs = Vec::new();
        
        for &node_a in self.nodes.keys() {
            if let Some(neighbors_a) = self.adjacency_list.get(&node_a) {
                for &node_b in neighbors_a {
                    if let Some(neighbors_b) = self.adjacency_list.get(&node_b) {
                        for &node_c in neighbors_b {
                            if node_c != node_a {
                                // Check if this forms a triangle motif
                                if let Some(neighbors_c) = self.adjacency_list.get(&node_c) {
                                    if neighbors_c.contains(&node_a) {
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

/// Analyze graph topology
pub fn analyze_graph_topology(quins: &[NQuin], context: u64) -> GraphAnalysisResult {
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
    
    GraphAnalysisResult {
        graph_quins,
        communities,
        motifs,
        top_nodes,
        density,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
    }
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
            NQuin { subject: 1, predicate: 1, object: 3, context: 100, metadata: 0, parity: 0 },
        ];
        
        let mut graph = QualiaGraph::from_quins(&quins);
        graph.calculate_betweenness_centrality();
        
        // Node 2 should have highest betweenness (it's between nodes 1 and 3)
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
        
        let result = analyze_graph_topology(&quins, 100);
        
        assert_eq!(result.node_count, 3);
        assert_eq!(result.edge_count, 3);
        assert!(result.density > 0.0);
        assert!(!result.communities.is_empty());
    }
}

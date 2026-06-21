use crate::NQuin;

pub fn evaluate_threshold(weight: f32, threshold: f32) -> bool {
    weight >= threshold
}

pub const MAX_BAYESIAN_NODES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BayesianNode {
    pub id: u64, // Variable hash
    pub parent_ids: [u64; 4], // Max 4 parents to keep it fixed size
    pub num_parents: usize,
    pub probabilities: [f32; 16], // 2^4 = 16 conditional probabilities
    pub evidence: Option<bool>, // Observed state if any
}

impl Default for BayesianNode {
    fn default() -> Self {
        Self {
            id: 0,
            parent_ids: [0; 4],
            num_parents: 0,
            probabilities: [0.0; 16],
            evidence: None,
        }
    }
}

pub struct BayesianNetwork {
    pub nodes: [BayesianNode; MAX_BAYESIAN_NODES],
    pub num_nodes: usize,
}

impl BayesianNetwork {
    pub fn new() -> Self {
        Self {
            nodes: [BayesianNode::default(); MAX_BAYESIAN_NODES],
            num_nodes: 0,
        }
    }

    pub fn add_node(&mut self, node: BayesianNode) -> Result<(), &'static str> {
        if self.num_nodes >= MAX_BAYESIAN_NODES {
            return Err("Max nodes exceeded");
        }
        self.nodes[self.num_nodes] = node;
        self.num_nodes += 1;
        Ok(())
    }

    /// Extract probability encoded in the Quin's metadata field (canonical
    /// truth-degree via the FrameLayout ABI — shared with fuzzy/stochastic).
    pub fn extract_weight(quin: &NQuin) -> f32 {
        crate::frame_layout::truth_degree(quin.metadata)
    }

    /// Exact inference via variable enumeration.
    /// Fully zero-allocation, iteratively evaluates joint probability table on the stack.
    /// Max unobserved variables allowed is 16 to prevent excessive CPU loop blocking (O(2^N)).
    pub fn update_beliefs(&self, target_id: u64) -> Option<f32> {
        let mut target_idx = None;
        for i in 0..self.num_nodes {
            if self.nodes[i].id == target_id {
                target_idx = Some(i);
                break;
            }
        }
        let target_idx = target_idx?;

        let mut num_unobserved = 0;
        let mut unobserved_indices = [0; MAX_BAYESIAN_NODES];
        for i in 0..self.num_nodes {
            if self.nodes[i].evidence.is_none() && i != target_idx {
                unobserved_indices[num_unobserved] = i;
                num_unobserved += 1;
            }
        }

        if num_unobserved > 16 {
            return None; // Too complex for zero-heap full enumeration
        }

        let num_assignments = 1 << num_unobserved;
        
        let mut prob_target_true = 0.0;
        let mut prob_target_false = 0.0;

        for assignment in 0..num_assignments {
            for target_state in [true, false] {
                let mut joint_prob = 1.0;
                
                for i in 0..self.num_nodes {
                    let node = &self.nodes[i];
                    
                    let state = if i == target_idx {
                        target_state
                    } else if let Some(e) = node.evidence {
                        e
                    } else {
                        let mut bit_idx = 0;
                        for j in 0..num_unobserved {
                            if unobserved_indices[j] == i {
                                bit_idx = j;
                                break;
                            }
                        }
                        ((assignment >> bit_idx) & 1) == 1
                    };

                    let mut parent_idx = 0;
                    for p in 0..node.num_parents {
                        let pid = node.parent_ids[p];
                        let mut p_state = false;
                        for j in 0..self.num_nodes {
                            if self.nodes[j].id == pid {
                                p_state = if j == target_idx {
                                    target_state
                                } else if let Some(e) = self.nodes[j].evidence {
                                    e
                                } else {
                                    let mut bit_idx = 0;
                                    for k in 0..num_unobserved {
                                        if unobserved_indices[k] == j {
                                            bit_idx = k;
                                            break;
                                        }
                                    }
                                    ((assignment >> bit_idx) & 1) == 1
                                };
                                break;
                            }
                        }
                        if p_state {
                            parent_idx |= 1 << p;
                        }
                    }
                    
                    let p_node = if state {
                        node.probabilities[parent_idx]
                    } else {
                        1.0 - node.probabilities[parent_idx]
                    };
                    
                    joint_prob *= p_node;
                }
                
                if target_state {
                    prob_target_true += joint_prob;
                } else {
                    prob_target_false += joint_prob;
                }
            }
        }

        let total = prob_target_true + prob_target_false;
        if total > 0.0 {
            Some(prob_target_true / total)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayesian_network() {
        let mut network = BayesianNetwork::new();
        
        // Node 0: Rain (no parents, P(Rain)=0.2)
        let mut node0 = BayesianNode::default();
        node0.id = 100;
        node0.probabilities[0] = 0.2;
        network.add_node(node0).unwrap();

        // Node 1: Sprinkler (parent: Rain)
        // P(Sprinkler|Rain) = 0.01
        // P(Sprinkler|NoRain) = 0.40
        let mut node1 = BayesianNode::default();
        node1.id = 200;
        node1.parent_ids[0] = 100;
        node1.num_parents = 1;
        node1.probabilities[1] = 0.01; // Rain=True -> bit 0 is 1
        node1.probabilities[0] = 0.40; // Rain=False -> bit 0 is 0
        network.add_node(node1).unwrap();

        // Node 2: Grass Wet (parents: Rain, Sprinkler)
        // Rain is bit 0, Sprinkler is bit 1
        // F, F (idx 0) = 0.0
        // T, F (idx 1) = 0.8
        // F, T (idx 2) = 0.9
        // T, T (idx 3) = 0.99
        let mut node2 = BayesianNode::default();
        node2.id = 300;
        node2.parent_ids[0] = 100;
        node2.parent_ids[1] = 200;
        node2.num_parents = 2;
        node2.probabilities[0] = 0.0;
        node2.probabilities[1] = 0.8;
        node2.probabilities[2] = 0.9;
        node2.probabilities[3] = 0.99;
        
        // Let's observe the grass is wet
        node2.evidence = Some(true);
        network.add_node(node2).unwrap();

        // Query: What is probability it rained, given grass is wet?
        let p_rain = network.update_beliefs(100).unwrap();
        
        // Approximate expected result: P(Rain|Wet) ≈ 0.3577
        assert!((p_rain - 0.3577).abs() < 0.001);
    }
}

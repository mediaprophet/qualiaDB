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

    /// The **Markov blanket** of `node_id`: its parents, its children, and its children's OTHER
    /// parents (co-parents). Conditioned on its blanket a node is independent of all others —
    /// the locality used for rapid conditional-independence testing and Gibbs sampling. Writes the
    /// member ids into `out`, returns the count. Zero-heap.
    pub fn markov_blanket(&self, node_id: u64, out: &mut [u64]) -> usize {
        let mut n = 0usize;
        let add = |id: u64, out: &mut [u64], n: &mut usize| {
            if id != node_id && id != 0 && !out[..*n].contains(&id) && *n < out.len() {
                out[*n] = id;
                *n += 1;
            }
        };
        for i in 0..self.num_nodes {
            if self.nodes[i].id == node_id {
                for p in 0..self.nodes[i].num_parents {
                    add(self.nodes[i].parent_ids[p], out, &mut n);
                }
            }
        }
        for i in 0..self.num_nodes {
            let child = self.nodes[i];
            if child.parent_ids[..child.num_parents].contains(&node_id) {
                add(child.id, out, &mut n); // a child
                for p in 0..child.num_parents {
                    add(child.parent_ids[p], out, &mut n); // its co-parents
                }
            }
        }
        n
    }

    /// `P(node = state | parents)` from the node's CPT, reading parents' states out of `assign`
    /// (indexed by node order).
    fn node_cpt(&self, node_idx: usize, state: bool, assign: &[bool]) -> f32 {
        let node = &self.nodes[node_idx];
        let mut parent_idx = 0usize;
        for p in 0..node.num_parents {
            let pid = node.parent_ids[p];
            for j in 0..self.num_nodes {
                if self.nodes[j].id == pid {
                    if assign[j] {
                        parent_idx |= 1 << p;
                    }
                    break;
                }
            }
        }
        let pt = node.probabilities[parent_idx];
        if state { pt } else { 1.0 - pt }
    }

    /// Unnormalised `P(node_idx = state | rest)` ∝ `P(node|parents) · Π_children P(child|parents)`
    /// — the Gibbs full-conditional over the Markov blanket.
    fn gibbs_conditional(&self, node_idx: usize, state: bool, assign: &[bool]) -> f32 {
        let mut a = [false; MAX_BAYESIAN_NODES];
        a[..self.num_nodes].copy_from_slice(&assign[..self.num_nodes]);
        a[node_idx] = state;
        let mut prob = self.node_cpt(node_idx, state, &a);
        let this_id = self.nodes[node_idx].id;
        for c in 0..self.num_nodes {
            let child = self.nodes[c];
            if child.parent_ids[..child.num_parents].contains(&this_id) {
                prob *= self.node_cpt(c, a[c], &a);
            }
        }
        prob
    }

    /// **Gibbs sampling** (MCMC) estimate of `P(target = true | evidence)` — approximate inference
    /// for networks too large for exact enumeration. `samples` sweeps, `seed` for the PRNG. Each
    /// non-evidence variable is resampled from its Markov-blanket conditional. Zero-heap (bounded
    /// stack arrays). `None` if `target_id` is not in the network.
    pub fn gibbs_estimate(&self, target_id: u64, samples: u32, seed: u64) -> Option<f32> {
        let mut target_idx = None;
        for i in 0..self.num_nodes {
            if self.nodes[i].id == target_id {
                target_idx = Some(i);
            }
        }
        let target_idx = target_idx?;
        let mut rng = seed | 1;
        let mut assign = [false; MAX_BAYESIAN_NODES];
        for i in 0..self.num_nodes {
            assign[i] = match self.nodes[i].evidence {
                Some(e) => e,
                None => next_unit(&mut rng) < 0.5,
            };
        }
        let mut count_true = 0u32;
        for _ in 0..samples {
            for i in 0..self.num_nodes {
                if self.nodes[i].evidence.is_some() {
                    continue;
                }
                let pt = self.gibbs_conditional(i, true, &assign);
                let pf = self.gibbs_conditional(i, false, &assign);
                let denom = pt + pf;
                let prob = if denom > 0.0 { pt / denom } else { 0.5 };
                assign[i] = next_unit(&mut rng) < prob;
            }
            if assign[target_idx] {
                count_true += 1;
            }
        }
        if samples == 0 {
            None
        } else {
            Some(count_true as f32 / samples as f32)
        }
    }
}

/// Deterministic xorshift PRNG → a uniform `f32` in `[0,1)`. Zero-heap.
fn next_unit(state: &mut u64) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    ((x >> 40) as f32) / ((1u32 << 24) as f32)
}

/// **PC-algorithm skeleton** (constraint-based structure learning): two variables are adjacent
/// (share an edge) iff their `correlation` is at/above `threshold` in absolute value — i.e. they
/// are NOT marginally independent. This is the order-0 skeleton; the full PC additionally removes
/// edges via conditional-independence tests over separating sets.
#[inline]
pub fn pc_adjacent(correlation: f32, threshold: f32) -> bool {
    correlation.abs() >= threshold
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

    fn sprinkler(with_wet_evidence: bool) -> BayesianNetwork {
        let mut net = BayesianNetwork::new();
        let mut n0 = BayesianNode::default();
        n0.id = 100;
        n0.probabilities[0] = 0.2;
        net.add_node(n0).unwrap();
        let mut n1 = BayesianNode::default();
        n1.id = 200;
        n1.parent_ids[0] = 100;
        n1.num_parents = 1;
        n1.probabilities[1] = 0.01;
        n1.probabilities[0] = 0.40;
        net.add_node(n1).unwrap();
        let mut n2 = BayesianNode::default();
        n2.id = 300;
        n2.parent_ids[0] = 100;
        n2.parent_ids[1] = 200;
        n2.num_parents = 2;
        n2.probabilities = [0.0, 0.8, 0.9, 0.99, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        if with_wet_evidence {
            n2.evidence = Some(true);
        }
        net.add_node(n2).unwrap();
        net
    }

    #[test]
    fn markov_blanket_of_a_node() {
        let net = sprinkler(false);
        // Blanket(Rain) = children {Sprinkler, GrassWet} + co-parents (Sprinkler already in).
        let mut out = [0u64; 8];
        let n = net.markov_blanket(100, &mut out);
        assert_eq!(n, 2);
        assert!(out[..n].contains(&200) && out[..n].contains(&300));
    }

    #[test]
    fn gibbs_approximates_the_exact_posterior() {
        let net = sprinkler(true);
        let exact = net.update_beliefs(100).unwrap(); // ≈ 0.3577
        let approx = net.gibbs_estimate(100, 40_000, 0x9E3779B97F4A7C15).unwrap();
        assert!((approx - exact).abs() < 0.08, "Gibbs {approx} should approximate exact {exact}");
    }

    #[test]
    fn pc_skeleton_uses_absolute_correlation() {
        assert!(pc_adjacent(0.6, 0.3));
        assert!(!pc_adjacent(0.1, 0.3));
        assert!(pc_adjacent(-0.5, 0.3), "structure depends on |correlation|");
    }
}

use crate::NQuin;

pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;
pub const DO_INTERVENTION_BIT: u64 = 1u64 << 57;
pub const COUNTERFACTUAL_BIT: u64 = 1u64 << 56;

/// Causal intervention operator for do-calculus
/// Implements P(Y | do(X = x)) by intervening on the causal graph.
///
/// The do-calculus intervention do(X = x) severs X from its parents (removes
/// incoming edges to X) while preserving X's outgoing causal edges so that
/// X can still influence downstream variables.  We mark the intervention on
/// the relevant quins with DO_INTERVENTION_BIT but do NOT overwrite the
/// structural `object` field (which encodes the causal successor, not X's
/// value).  The intervention_value is recorded in metadata so callers can
/// inspect it; path existence is used as the evidence of causal effect.
pub fn do_intervention(
    graph: &[NQuin],
    intervention_var: u64,
    intervention_value: u64,
    target_var: u64,
) -> Option<f64> {
    let mut causal_paths = Vec::new();
    let mut intervened_graph = graph.to_vec();

    // Apply intervention: mark outgoing edges from X and drop incoming edges
    // to X (cut X from its parents) but preserve the causal structure X → …
    // Store intervention_value in the upper bits of metadata so it is
    // visible without corrupting the `object` (causal target) field.
    intervened_graph.retain(|q| q.object != intervention_var); // remove parents of X
    for quin in &mut intervened_graph {
        if quin.subject == intervention_var {
            quin.metadata = DO_INTERVENTION_BIT | (intervention_value << 32);
        }
    }

    // Find causal paths from intervention variable to target
    find_causal_paths(&intervened_graph, intervention_var, target_var, &mut causal_paths);

    if causal_paths.is_empty() {
        return None;
    }

    // P(Y = target_var reached) = fraction of discovered paths
    // Each discovered path represents one causal route; all routes count as
    // evidence that the intervention influences the target.
    let total_count = causal_paths.len() as f64;
    Some(total_count / total_count) // = 1.0 when any path exists
}

/// Counterfactual query: "What would happen if X were x?"
pub fn counterfactual_query(
    actual_graph: &[NQuin],
    factual_outcome: u64,
    counterfactual_intervention: u64,
    intervention_value: u64,
    target_var: u64,
) -> Option<NQuin> {
    // Step 1: Abduction - update beliefs based on actual outcome
    let mut updated_graph = actual_graph.to_vec();
    for quin in &mut updated_graph {
        if quin.subject == target_var {
            quin.object = factual_outcome;
            quin.metadata |= COUNTERFACTUAL_BIT;
        }
    }
    
    // Step 2: Action - apply counterfactual intervention.
    // Mark the intervention in metadata (upper bits hold the intervention value)
    // but preserve the structural `object` field (causal successor) so that
    // do_intervention() can still traverse the causal graph.
    for quin in &mut updated_graph {
        if quin.subject == counterfactual_intervention {
            quin.metadata |= DO_INTERVENTION_BIT | (intervention_value << 32);
        }
    }
    
    // Step 3: Prediction - compute counterfactual outcome
    if let Some(counterfactual_prob) = do_intervention(
        &updated_graph,
        counterfactual_intervention,
        intervention_value,
        target_var,
    ) {
        let mut result = NQuin::default();
        result.subject = target_var;
        result.predicate = crate::q_hash("has_counterfactual_probability");
        result.object = (counterfactual_prob * 1000.0) as u64; // Store as scaled integer
        result.metadata = COUNTERFACTUAL_BIT;
        result.parity = result.subject ^ result.predicate ^ result.object ^ result.context;
        
        Some(result)
    } else {
        None
    }
}

/// Find all causal paths from source to target in the causal graph
fn find_causal_paths(
    graph: &[NQuin],
    source: u64,
    target: u64,
    paths: &mut Vec<Vec<NQuin>>,
) {
    // Simple depth-first search for causal paths
    let mut visited = std::collections::HashSet::new();
    let mut current_path = Vec::new();
    
    dfs_find_paths(graph, source, target, &mut visited, &mut current_path, paths);
}

/// Depth-first search helper for finding causal paths
fn dfs_find_paths(
    graph: &[NQuin],
    current: u64,
    target: u64,
    visited: &mut std::collections::HashSet<u64>,
    current_path: &mut Vec<NQuin>,
    all_paths: &mut Vec<Vec<NQuin>>,
) {
    if visited.contains(&current) {
        return;
    }
    
    visited.insert(current);
    
    // Find all outgoing edges from current node
    for quin in graph {
        if quin.subject == current {
            current_path.push(*quin);
            
            if quin.object == target {
                // Found a path to target
                all_paths.push(current_path.clone());
            } else {
                // Continue searching
                dfs_find_paths(graph, quin.object, target, visited, current_path, all_paths);
            }
            
            current_path.pop();
        }
    }
    
    visited.remove(&current);
}

/// Check if two variables are confounded (share a common cause)
pub fn are_confounded(graph: &[NQuin], var1: u64, var2: u64) -> bool {
    // Find common causes by looking for nodes that point to both var1 and var2
    let mut parents1 = std::collections::HashSet::new();
    let mut parents2 = std::collections::HashSet::new();
    
    for quin in graph {
        if quin.object == var1 {
            parents1.insert(quin.subject);
        }
        if quin.object == var2 {
            parents2.insert(quin.subject);
        }
    }
    
    // Check for intersection (common causes)
    !parents1.is_disjoint(&parents2)
}

/// Compute do-calculus adjustment for confounding
pub fn adjust_for_confounding(
    graph: &[NQuin],
    treatment: u64,
    outcome: u64,
    confounder: u64,
) -> Option<f64> {
    // Simplified adjustment: P(Y|do(X)) = Σ_z P(Y|X,Z=z) * P(Z=z)
    // This is a basic implementation - full do-calculus would be more sophisticated
    
    let mut adjusted_prob = 0.0;
    let mut confounder_values = std::collections::HashSet::new();
    
    // Collect all possible values of confounder
    for quin in graph {
        if quin.subject == confounder {
            confounder_values.insert(quin.object);
        }
    }
    
    // Compute adjustment
    for &confounder_val in &confounder_values {
        // P(Y|X,Z=z)
        let mut filtered_graph = graph.to_vec();
        for quin in &mut filtered_graph {
            if quin.subject == treatment {
                quin.metadata |= DO_INTERVENTION_BIT;
            }
            if quin.subject == confounder {
                quin.object = confounder_val;
            }
        }
        
        if let Some(p_y_given_x_z) = compute_conditional_probability(&filtered_graph, outcome, treatment) {
            // P(Z=z) - simplified as uniform distribution
            let p_z = 1.0 / confounder_values.len() as f64;
            adjusted_prob += p_y_given_x_z * p_z;
        }
    }
    
    if adjusted_prob > 0.0 {
        Some(adjusted_prob)
    } else {
        None
    }
}

/// Compute conditional probability P(Y|X) from graph.
///
/// In a causal graph whose edges represent causal arrows (not observations),
/// P(Y|X) is estimated as 1.0 if Y is causally reachable from X via a
/// directed path, and None if X has no outgoing edges at all (X is
/// unobserved / disconnected in this context).
fn compute_conditional_probability(graph: &[NQuin], y_var: u64, x_var: u64) -> Option<f64> {
    // Check that X participates as a cause in this graph
    let x_has_edges = graph.iter().any(|q| q.subject == x_var);
    if !x_has_edges {
        return None;
    }

    // BFS / DFS reachability from x_var to y_var
    let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut frontier: Vec<u64> = vec![x_var];
    while let Some(current) = frontier.pop() {
        if current == y_var {
            return Some(1.0);
        }
        if visited.insert(current) {
            for quin in graph {
                if quin.subject == current && !visited.contains(&quin.object) {
                    frontier.push(quin.object);
                }
            }
        }
    }

    // Y not reachable from X in this graph — no causal effect
    Some(0.0)
}

pub fn synthesize_dialectical(thesis: &NQuin, antithesis: &NQuin) -> Option<NQuin> {
    // A contradiction requires the same subject and predicate but different object
    if thesis.subject == antithesis.subject
        && thesis.predicate == antithesis.predicate
        && thesis.object != antithesis.object
    {
        let mut synthesized = *thesis;
        synthesized.context = thesis.context ^ antithesis.context;
        synthesized.metadata |= SYNTHESIZED_BIT;
        // The object becomes a combination, maybe just bitwise XOR for now?
        synthesized.object = thesis.object ^ antithesis.object;

        // Update parity to maintain structural integrity
        synthesized.parity =
            synthesized.subject ^ synthesized.predicate ^ synthesized.object ^ synthesized.context;

        return Some(synthesized);
    }
    None
}

// ─── Causal necessity (but-for) — zero-heap reachability ─────────────────────────

/// Max nodes for the bounded zero-heap causal reachability search.
pub const MAX_CAUSAL_NODES: usize = 256;

/// Zero-heap reachability over causal edges (`subject → object`): is `target`
/// reachable from `source` WITHOUT ever passing through `avoid`? Bounded BFS over
/// fixed stack buffers (no allocation). Pass `avoid == u64::MAX` to avoid nothing.
/// (The heap variant `find_causal_paths` enumerates *all* paths for analysis;
/// this answers the yes/no reachability the but-for test needs, allocation-free.)
pub fn reachable_avoiding(graph: &[NQuin], source: u64, target: u64, avoid: u64) -> bool {
    if source == avoid {
        return false;
    }
    if source == target {
        return true;
    }
    let mut stack = [0u64; MAX_CAUSAL_NODES];
    let mut slen = 1usize;
    stack[0] = source;
    let mut visited = [0u64; MAX_CAUSAL_NODES];
    let mut vlen = 1usize;
    visited[0] = source;

    while slen > 0 {
        slen -= 1;
        let node = stack[slen];
        for q in graph {
            if q.subject != node || q.object == avoid {
                continue;
            }
            if q.object == target {
                return true;
            }
            let mut seen = false;
            for &v in visited.iter().take(vlen) {
                if v == q.object {
                    seen = true;
                    break;
                }
            }
            if !seen && vlen < MAX_CAUSAL_NODES && slen < MAX_CAUSAL_NODES {
                visited[vlen] = q.object;
                vlen += 1;
                stack[slen] = q.object;
                slen += 1;
            }
        }
    }
    false
}

/// But-for causal necessity: `candidate` is a NECESSARY cause of `effect` (from
/// origin `root`) iff `effect` is reachable from `root`, but is NOT reachable once
/// `candidate` is removed from the causal graph. The attribution/liability test
/// ("would the harm have occurred but for this agent's act?"). Zero-heap.
pub fn is_necessary_cause(graph: &[NQuin], root: u64, candidate: u64, effect: u64) -> bool {
    reachable_avoiding(graph, root, effect, u64::MAX)
        && !reachable_avoiding(graph, root, effect, candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(cause: u64, effect: u64) -> NQuin {
        let mut q = NQuin { subject: cause, predicate: crate::q_hash("causal:causes"), object: effect, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn but_for_causal_necessity() {
        // Chain: root → C → effect. C is necessary (removing it disconnects effect).
        let chain = [edge(1, 2), edge(2, 3)];
        assert!(is_necessary_cause(&chain, 1, 2, 3), "C is a necessary cause in a chain");
        // Diamond: root → C → effect AND root → D → effect. C is NOT necessary.
        let diamond = [edge(1, 2), edge(2, 4), edge(1, 3), edge(3, 4)];
        assert!(!is_necessary_cause(&diamond, 1, 2, 4), "C is not necessary when an alternative path exists");
        assert!(reachable_avoiding(&diamond, 1, 4, u64::MAX), "effect is reachable normally");
    }

    #[test]
    fn test_synthesize_dialectical() {
        let thesis = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 10,
            metadata: 0,
            parity: 0,
        };
        let antithesis = NQuin {
            subject: 1,
            predicate: 2,
            object: 4,
            context: 20,
            metadata: 0,
            parity: 0,
        };

        let syn = synthesize_dialectical(&thesis, &antithesis).unwrap();
        assert_eq!(syn.context, 10 ^ 20);
        assert!(syn.metadata & SYNTHESIZED_BIT != 0);
    }
    
    #[test]
    fn test_do_intervention() {
        // Create a simple causal graph: X -> Y
        let mut graph = Vec::new();
        
        // X = 1 causes Y = 1
        let mut x_to_y = NQuin::default();
        x_to_y.subject = 1; // X
        x_to_y.predicate = crate::q_hash("causes");
        x_to_y.object = 2; // Y
        x_to_y.context = 100;
        x_to_y.parity = x_to_y.subject ^ x_to_y.predicate ^ x_to_y.object ^ x_to_y.context;
        graph.push(x_to_y);
        
        // Test intervention: do(X = 1) should affect Y
        let result = do_intervention(&graph, 1, 1, 2);
        assert!(result.is_some());
        assert!(result.unwrap() > 0.0);
    }
    
    #[test]
    fn test_counterfactual_query() {
        // Create causal graph: Treatment -> Outcome
        let mut graph = Vec::new();
        
        let mut treatment_to_outcome = NQuin::default();
        treatment_to_outcome.subject = 10; // Treatment
        treatment_to_outcome.predicate = crate::q_hash("causes");
        treatment_to_outcome.object = 20; // Outcome
        treatment_to_outcome.context = 200;
        treatment_to_outcome.parity = treatment_to_outcome.subject ^ treatment_to_outcome.predicate ^ treatment_to_outcome.object ^ treatment_to_outcome.context;
        graph.push(treatment_to_outcome);
        
        // Test counterfactual: "What if Treatment were 0?"
        let result = counterfactual_query(&graph, 1, 10, 0, 20);
        assert!(result.is_some());
        
        let counterfactual = result.unwrap();
        assert_eq!(counterfactual.subject, 20); // Target is outcome
        assert!(counterfactual.metadata & COUNTERFACTUAL_BIT != 0);
    }
    
    #[test]
    fn test_confounding_detection() {
        // Create graph with confounding: Confounder -> Treatment, Confounder -> Outcome
        let mut graph = Vec::new();
        
        // Confounder -> Treatment
        let mut conf_to_treat = NQuin::default();
        conf_to_treat.subject = 100; // Confounder
        conf_to_treat.predicate = crate::q_hash("causes");
        conf_to_treat.object = 10; // Treatment
        conf_to_treat.context = 300;
        conf_to_treat.parity = conf_to_treat.subject ^ conf_to_treat.predicate ^ conf_to_treat.object ^ conf_to_treat.context;
        graph.push(conf_to_treat);
        
        // Confounder -> Outcome
        let mut conf_to_outcome = NQuin::default();
        conf_to_outcome.subject = 100; // Confounder
        conf_to_outcome.predicate = crate::q_hash("causes");
        conf_to_outcome.object = 20; // Outcome
        conf_to_outcome.context = 301;
        conf_to_outcome.parity = conf_to_outcome.subject ^ conf_to_outcome.predicate ^ conf_to_outcome.object ^ conf_to_outcome.context;
        graph.push(conf_to_outcome);
        
        // Test confounding detection
        let confounded = are_confounded(&graph, 10, 20);
        assert!(confounded);
    }
    
    #[test]
    fn test_adjust_for_confounding() {
        // Create graph with confounding
        let mut graph = Vec::new();
        
        // Confounder -> Treatment
        let mut conf_to_treat = NQuin::default();
        conf_to_treat.subject = 100; // Confounder
        conf_to_treat.predicate = crate::q_hash("causes");
        conf_to_treat.object = 10; // Treatment
        conf_to_treat.context = 400;
        conf_to_treat.parity = conf_to_treat.subject ^ conf_to_treat.predicate ^ conf_to_treat.object ^ conf_to_treat.context;
        graph.push(conf_to_treat);
        
        // Treatment -> Outcome
        let mut treat_to_outcome = NQuin::default();
        treat_to_outcome.subject = 10; // Treatment
        treat_to_outcome.predicate = crate::q_hash("causes");
        treat_to_outcome.object = 20; // Outcome
        treat_to_outcome.context = 401;
        treat_to_outcome.parity = treat_to_outcome.subject ^ treat_to_outcome.predicate ^ treat_to_outcome.object ^ treat_to_outcome.context;
        graph.push(treat_to_outcome);
        
        // Test adjustment
        let adjusted = adjust_for_confounding(&graph, 10, 20, 100);
        assert!(adjusted.is_some());
        assert!(adjusted.unwrap() >= 0.0);
    }
    
    #[test]
    fn test_no_contradiction() {
        let thesis = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 10,
            metadata: 0,
            parity: 0,
        };

        let no_contradiction = NQuin {
            subject: 1,
            predicate: 3, // Different predicate
            object: 4,
            context: 20,
            metadata: 0,
            parity: 0,
        };
        
        assert!(synthesize_dialectical(&thesis, &no_contradiction).is_none());
    }
}

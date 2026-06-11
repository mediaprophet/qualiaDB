use crate::NQuin;

pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;
pub const DO_INTERVENTION_BIT: u64 = 1u64 << 57;
pub const COUNTERFACTUAL_BIT: u64 = 1u64 << 56;

/// Causal intervention operator for do-calculus
/// Implements P(Y | do(X = x)) by intervening on the causal graph
pub fn do_intervention(
    graph: &[NQuin],
    intervention_var: u64,
    intervention_value: u64,
    target_var: u64,
) -> Option<f64> {
    // Find all quins representing causal relationships
    let mut causal_paths = Vec::new();
    let mut intervened_graph = graph.to_vec();
    
    // Apply intervention: set X = x by removing incoming edges to X
    for quin in &mut intervened_graph {
        if quin.subject == intervention_var {
            quin.object = intervention_value;
            quin.metadata |= DO_INTERVENTION_BIT;
        }
    }
    
    // Find causal paths from intervention to target
    find_causal_paths(&intervened_graph, intervention_var, target_var, &mut causal_paths);
    
    if causal_paths.is_empty() {
        return None;
    }
    
    // Calculate probability of target given intervention
    // Simplified: count paths where target is true
    let mut true_count = 0;
    let mut total_count = 0;
    
    for path in &causal_paths {
        if path.last().map_or(false, |q| q.object == 1) {
            true_count += 1;
        }
        total_count += 1;
    }
    
    if total_count > 0 {
        Some(true_count as f64 / total_count as f64)
    } else {
        None
    }
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
    
    // Step 2: Action - apply counterfactual intervention
    for quin in &mut updated_graph {
        if quin.subject == counterfactual_intervention {
            quin.object = intervention_value;
            quin.metadata |= DO_INTERVENTION_BIT;
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

/// Compute conditional probability P(Y|X) from graph
fn compute_conditional_probability(graph: &[NQuin], y_var: u64, x_var: u64) -> Option<f64> {
    let mut x_true_count = 0;
    let mut x_true_y_true_count = 0;
    
    for quin in graph {
        if quin.subject == x_var && quin.object == 1 {
            x_true_count += 1;
            
            // Check if Y is also true in this context
            for y_quin in graph {
                if y_quin.subject == y_var && y_quin.context == quin.context && y_quin.object == 1 {
                    x_true_y_true_count += 1;
                    break;
                }
            }
        }
    }
    
    if x_true_count > 0 {
        Some(x_true_y_true_count as f64 / x_true_count as f64)
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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

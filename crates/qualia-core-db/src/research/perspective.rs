//! Perspective analysis — register, bias, compare, conflict detection, reconciliation.

/// A perspective held by an agent or observer.
#[derive(Debug, Clone)]
pub struct Perspective {
    pub id: String,
    pub agent_id: String,
    pub viewpoint: String,
    pub biases: Vec<Bias>,
    pub confidence: f64,
}

/// A cognitive bias affecting a perspective.
#[derive(Debug, Clone)]
pub struct Bias {
    pub bias_type: String,
    pub description: String,
    pub severity: f64,
}

/// A conflict between two perspectives.
#[derive(Debug, Clone)]
pub struct PerspectiveConflict {
    pub perspective_a: String,
    pub perspective_b: String,
    pub conflict_type: String,
    pub description: String,
    pub severity: f64,
}

/// Register a new perspective.
pub fn register_perspective(id: &str, agent_id: &str, viewpoint: &str) -> Perspective {
    Perspective {
        id: id.to_string(),
        agent_id: agent_id.to_string(),
        viewpoint: viewpoint.to_string(),
        biases: Vec::new(),
        confidence: 0.5,
    }
}

/// Add a bias to a perspective.
pub fn add_bias(perspective: &mut Perspective, bias: Bias) {
    perspective.biases.push(bias);
}

/// Compare two perspectives — returns similarity score and identified conflicts.
pub fn compare_perspectives(a: &Perspective, b: &Perspective) -> PerspectiveComparison {
    let a_lower = a.viewpoint.to_lowercase();
    let b_lower = b.viewpoint.to_lowercase();
    let a_words: Vec<&str> = a_lower.split_whitespace().collect();
    let b_words: Vec<&str> = b_lower.split_whitespace().collect();
    let a_set: std::collections::BTreeSet<&str> = a_words.iter().copied().collect();
    let b_set: std::collections::BTreeSet<&str> = b_words.iter().copied().collect();
    let common = a_set.intersection(&b_set).count();
    let total = a_set.union(&b_set).count();
    let similarity = if total > 0 {
        common as f64 / total as f64
    } else {
        0.0
    };

    let conflicts = detect_perspective_conflict(a, b);
    PerspectiveComparison {
        similarity,
        conflicts,
    }
}

#[derive(Debug, Clone)]
pub struct PerspectiveComparison {
    pub similarity: f64,
    pub conflicts: Vec<PerspectiveConflict>,
}

/// Detect conflicts between two perspectives.
pub fn detect_perspective_conflict(a: &Perspective, b: &Perspective) -> Vec<PerspectiveConflict> {
    let mut conflicts = Vec::new();
    // Check for opposing sentiment markers.
    let opposition_markers = [
        ("agree", "disagree"),
        ("true", "false"),
        ("correct", "incorrect"),
        ("right", "wrong"),
        ("good", "bad"),
        ("yes", "no"),
    ];
    let a_lower = a.viewpoint.to_lowercase();
    let b_lower = b.viewpoint.to_lowercase();
    for (pos, neg) in opposition_markers {
        if (a_lower.contains(pos) && b_lower.contains(neg))
            || (a_lower.contains(neg) && b_lower.contains(pos))
        {
            conflicts.push(PerspectiveConflict {
                perspective_a: a.id.clone(),
                perspective_b: b.id.clone(),
                conflict_type: "opposing_claim".into(),
                description: format!("'{pos}' vs '{neg}' opposition detected"),
                severity: 0.8,
            });
        }
    }
    conflicts
}

/// Reconcile two perspectives — find common ground.
pub fn reconcile_perspectives(a: &Perspective, b: &Perspective) -> ReconciliationResult {
    let comparison = compare_perspectives(a, b);
    let a_lower = a.viewpoint.to_lowercase();
    let b_lower = b.viewpoint.to_lowercase();
    let common_words: Vec<&str> = {
        let a_set: std::collections::BTreeSet<&str> = a_lower.split_whitespace().collect();
        let b_set: std::collections::BTreeSet<&str> = b_lower.split_whitespace().collect();
        a_set.intersection(&b_set).copied().collect()
    };
    ReconciliationResult {
        common_ground: common_words.join(" "),
        similarity: comparison.similarity,
        conflict_count: comparison.conflicts.len(),
        reconcilable: comparison.similarity > 0.3 && comparison.conflicts.len() < 3,
    }
}

#[derive(Debug, Clone)]
pub struct ReconciliationResult {
    pub common_ground: String,
    pub similarity: f64,
    pub conflict_count: usize,
    pub reconcilable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_perspective_basic() {
        let p = register_perspective("p1", "agent_a", "Climate change is real");
        assert_eq!(p.id, "p1");
        assert_eq!(p.agent_id, "agent_a");
    }

    #[test]
    fn add_bias_basic() {
        let mut p = register_perspective("p1", "agent_a", "Test");
        add_bias(
            &mut p,
            Bias {
                bias_type: "confirmation".into(),
                description: "Seeks confirming evidence".into(),
                severity: 0.7,
            },
        );
        assert_eq!(p.biases.len(), 1);
    }

    #[test]
    fn compare_similar_perspectives() {
        let a = register_perspective("p1", "a", "climate change is real and dangerous");
        let b = register_perspective("p2", "b", "climate change is real and concerning");
        let comp = compare_perspectives(&a, &b);
        assert!(comp.similarity > 0.5);
    }

    #[test]
    fn compare_different_perspectives() {
        let a = register_perspective("p1", "a", "the sky is blue");
        let b = register_perspective("p2", "b", "economic policy reform");
        let comp = compare_perspectives(&a, &b);
        assert!(comp.similarity < 0.3);
    }

    #[test]
    fn detect_conflict_opposing() {
        let a = register_perspective("p1", "a", "I agree with the proposal");
        let b = register_perspective("p2", "b", "I disagree with the proposal");
        let conflicts = detect_perspective_conflict(&a, &b);
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn reconcile_reconcilable() {
        let a = register_perspective("p1", "a", "climate change is real and dangerous");
        let b = register_perspective("p2", "b", "climate change is real and concerning");
        let result = reconcile_perspectives(&a, &b);
        assert!(result.reconcilable);
        assert!(!result.common_ground.is_empty());
    }

    #[test]
    fn reconcile_not_reconcilable() {
        let a = register_perspective("p1", "a", "the sky is blue today");
        let b = register_perspective("p2", "b", "economic policy reform needed");
        let result = reconcile_perspectives(&a, &b);
        assert!(!result.reconcilable);
    }
}

//! Task output scoring and validation against ground truth.

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TaskScore {
    pub accuracy: f32,
    pub citation_fidelity: f32,
    pub constraint_satisfaction: f32,
}

impl TaskScore {
    pub fn composite_score(&self) -> f32 {
        0.5 * self.accuracy + 0.3 * self.citation_fidelity + 0.2 * self.constraint_satisfaction
    }
}

/// Score an output string against the expected outcome and evidence citations.
pub fn evaluate_task_output(
    actual: &str,
    expected: &str,
    citations_present: bool,
    constraints_satisfied: bool,
) -> TaskScore {
    let accuracy = if actual.trim() == expected.trim() {
        1.0
    } else if actual.contains(expected) {
        0.7
    } else {
        0.0
    };

    let citation_fidelity = if citations_present { 1.0 } else { 0.0 };
    let constraint_satisfaction = if constraints_satisfied { 1.0 } else { 0.0 };

    TaskScore {
        accuracy,
        citation_fidelity,
        constraint_satisfaction,
    }
}

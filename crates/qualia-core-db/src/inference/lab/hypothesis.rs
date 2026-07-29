//! Hypothesis backlog and typed belief graph with confidence cascade.
//!
//! The belief graph is a directed acyclic graph (DAG) of epistemic nodes:
//! - **Hypothesis**: a testable claim about an optimization strategy.
//! - **Experiment**: a trial that produces evidence for/against a hypothesis.
//! - **Observation**: a measured datum from an experiment.
//! - **Claim**: a derived conclusion from one or more observations.
//!
//! When an experiment produces a verdict on a hypothesis, the confidence
//! cascades to dependent hypotheses via the DAG edges.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Unique identifier for a belief graph node.
pub type NodeId = String;

/// A hypothesis: a testable claim about an optimization strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: NodeId,
    /// Human-readable statement, e.g. "coop_gemv improves decode tok/s by >20%".
    pub statement: String,
    /// The configuration space this hypothesis operates on.
    pub space_name: String,
    /// Expected direction: true = improvement, false = regression.
    pub expects_improvement: bool,
    /// Current confidence in [-1, 1]: -1 = refuted, 0 = unknown, +1 = confirmed.
    pub confidence: f64,
    /// IDs of hypotheses this depends on (predecessors in the DAG).
    pub depends_on: Vec<NodeId>,
    /// IDs of experiments that have tested this hypothesis.
    pub experiments: Vec<NodeId>,
    /// Whether this hypothesis is active (eligible for testing).
    pub active: bool,
    /// Creation timestamp (unix ms).
    pub created_ms: u64,
}

impl Hypothesis {
    pub fn new(
        id: impl Into<String>,
        statement: impl Into<String>,
        space_name: impl Into<String>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            id: id.into(),
            statement: statement.into(),
            space_name: space_name.into(),
            expects_improvement: true,
            confidence: 0.0,
            depends_on: Vec::new(),
            experiments: Vec::new(),
            active: true,
            created_ms: now,
        }
    }

    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.depends_on.push(dep.into());
        self
    }

    pub fn expects_regression(mut self) -> Self {
        self.expects_improvement = false;
        self
    }
}

/// The verdict of an experiment on a hypothesis.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExperimentVerdict {
    /// The hypothesis is supported by the evidence.
    Confirmed,
    /// The hypothesis is refuted by the evidence.
    Refuted,
    /// The evidence is inconclusive.
    Inconclusive,
    /// The experiment failed (no data).
    Failed,
}

impl ExperimentVerdict {
    /// Convert to a confidence delta.
    pub fn confidence_delta(&self, weight: f64) -> f64 {
        match self {
            Self::Confirmed => weight,
            Self::Refuted => -weight,
            Self::Inconclusive => 0.0,
            Self::Failed => 0.0,
        }
    }
}

/// An experiment node in the belief graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentNode {
    pub id: NodeId,
    /// The hypothesis this experiment tests.
    pub hypothesis_id: NodeId,
    /// The configuration tested.
    pub config_hash: u64,
    /// The verdict.
    pub verdict: ExperimentVerdict,
    /// Weight of this experiment's evidence [0, 1].
    pub weight: f64,
    /// Timestamp (unix ms).
    pub timestamp_ms: u64,
}

/// An observation node: a single measured datum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: NodeId,
    pub experiment_id: NodeId,
    pub metric: String,
    pub value: f64,
    pub unit: String,
}

/// A claim node: a derived conclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: NodeId,
    pub statement: String,
    /// Observations that support this claim.
    pub supported_by: Vec<NodeId>,
    pub confidence: f64,
}

/// The typed belief graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefGraph {
    pub hypotheses: HashMap<NodeId, Hypothesis>,
    pub experiments: HashMap<NodeId, ExperimentNode>,
    pub observations: HashMap<NodeId, Observation>,
    pub claims: HashMap<NodeId, Claim>,
}

impl BeliefGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hypothesis to the graph.
    pub fn add_hypothesis(&mut self, h: Hypothesis) {
        self.hypotheses.insert(h.id.clone(), h);
    }

    /// Record an experiment result against a hypothesis and update confidence.
    pub fn record_experiment(
        &mut self,
        experiment_id: impl Into<String>,
        hypothesis_id: &str,
        config_hash: u64,
        verdict: ExperimentVerdict,
        weight: f64,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let exp = ExperimentNode {
            id: experiment_id.into(),
            hypothesis_id: hypothesis_id.to_string(),
            config_hash,
            verdict,
            weight,
            timestamp_ms: now,
        };

        // Update hypothesis confidence.
        if let Some(h) = self.hypotheses.get_mut(hypothesis_id) {
            let delta = verdict.confidence_delta(weight);
            // Exponential moving average for confidence.
            let alpha = 0.3;
            h.confidence = h.confidence * (1.0 - alpha) + delta * alpha;
            h.confidence = h.confidence.clamp(-1.0, 1.0);
            h.experiments.push(exp.id.clone());

            // If strongly refuted, deactivate.
            if h.confidence < -0.5 {
                h.active = false;
            }
        }

        self.experiments.insert(exp.id.clone(), exp);

        // Cascade confidence to dependent hypotheses.
        self.cascade_confidence(hypothesis_id);
    }

    /// Cascade confidence from a hypothesis to its dependents.
    fn cascade_confidence(&mut self, source_id: &str) {
        // Find hypotheses that depend on the source.
        let dependents: Vec<NodeId> = self
            .hypotheses
            .values()
            .filter(|h| h.depends_on.iter().any(|d| d == source_id))
            .map(|h| h.id.clone())
            .collect();

        for dep_id in dependents {
            // The dependent's confidence is influenced by the source's confidence.
            let source_confidence = self
                .hypotheses
                .get(source_id)
                .map(|s| s.confidence)
                .unwrap_or(0.0);
            if let Some(dep) = self.hypotheses.get_mut(&dep_id) {
                // If the source is confirmed, boost the dependent's confidence.
                // If refuted, reduce it.
                let influence = source_confidence * 0.2;
                dep.confidence = (dep.confidence + influence).clamp(-1.0, 1.0);
            }
        }
    }

    /// Add an observation linked to an experiment.
    pub fn add_observation(
        &mut self,
        id: impl Into<String>,
        experiment_id: impl Into<String>,
        metric: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
    ) {
        let obs = Observation {
            id: id.into(),
            experiment_id: experiment_id.into(),
            metric: metric.into(),
            value,
            unit: unit.into(),
        };
        self.observations.insert(obs.id.clone(), obs);
    }

    /// Add a claim supported by observations.
    pub fn add_claim(
        &mut self,
        id: impl Into<String>,
        statement: impl Into<String>,
        supported_by: Vec<NodeId>,
    ) {
        let confidence = self.compute_claim_confidence(&supported_by);
        let claim = Claim {
            id: id.into(),
            statement: statement.into(),
            supported_by,
            confidence,
        };
        self.claims.insert(claim.id.clone(), claim);
    }

    /// Compute confidence for a claim based on supporting observations.
    fn compute_claim_confidence(&self, obs_ids: &[NodeId]) -> f64 {
        if obs_ids.is_empty() {
            return 0.0;
        }
        // Average the experiment confidences that produced these observations.
        let mut total = 0.0;
        let mut count = 0;
        for obs_id in obs_ids {
            if let Some(obs) = self.observations.get(obs_id) {
                if let Some(exp) = self.experiments.get(&obs.experiment_id) {
                    if let Some(h) = self.hypotheses.get(&exp.hypothesis_id) {
                        total += h.confidence;
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            0.0
        } else {
            (total / count as f64).clamp(-1.0, 1.0)
        }
    }

    /// Get all active hypotheses sorted by confidence (descending).
    pub fn active_hypotheses(&self) -> Vec<&Hypothesis> {
        let mut active: Vec<&Hypothesis> = self.hypotheses.values().filter(|h| h.active).collect();
        active.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        active
    }

    /// Get the next hypothesis to test (highest confidence uncertainty).
    pub fn next_to_test(&self) -> Option<&Hypothesis> {
        self.active_hypotheses().into_iter().min_by(|a, b| {
            a.confidence
                .abs()
                .partial_cmp(&b.confidence.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Serialize the entire belief graph to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = self.to_json();
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Load from a JSON file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }
}

/// Determine the verdict of an experiment against a hypothesis.
/// This compares the experiment result against a baseline.
pub fn evaluate_verdict(
    hypothesis: &Hypothesis,
    treatment_tok_s: f64,
    baseline_tok_s: f64,
    improvement_threshold: f64,
) -> ExperimentVerdict {
    if baseline_tok_s <= 0.0 || treatment_tok_s <= 0.0 {
        return ExperimentVerdict::Failed;
    }
    let relative = (treatment_tok_s - baseline_tok_s) / baseline_tok_s;
    let improved = relative > improvement_threshold;
    let regressed = relative < -improvement_threshold;

    match (hypothesis.expects_improvement, improved, regressed) {
        (true, true, false) => ExperimentVerdict::Confirmed,
        (true, false, true) => ExperimentVerdict::Refuted,
        (false, false, true) => ExperimentVerdict::Confirmed,
        (false, true, false) => ExperimentVerdict::Refuted,
        _ => ExperimentVerdict::Inconclusive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypothesis_confidence_updates() {
        let mut graph = BeliefGraph::new();
        let h = Hypothesis::new("H-001", "coop_gemv improves decode by >20%", "toggle_space");
        graph.add_hypothesis(h);

        graph.record_experiment("E-001", "H-001", 42, ExperimentVerdict::Confirmed, 0.8);
        let h = graph.hypotheses.get("H-001").unwrap();
        assert!(h.confidence > 0.0);
        assert_eq!(h.experiments.len(), 1);
    }

    #[test]
    fn confidence_cascade() {
        let mut graph = BeliefGraph::new();
        let h1 = Hypothesis::new("H-001", "coop_gemv improves decode", "space");
        graph.add_hypothesis(h1);
        let h2 =
            Hypothesis::new("H-002", "fused_ffn improves decode", "space").with_dependency("H-001");
        graph.add_hypothesis(h2);

        // Confirm H-001 strongly.
        graph.record_experiment("E-001", "H-001", 42, ExperimentVerdict::Confirmed, 1.0);
        graph.record_experiment("E-002", "H-001", 43, ExperimentVerdict::Confirmed, 1.0);

        // H-002 should have positive confidence from cascade.
        let h2 = graph.hypotheses.get("H-002").unwrap();
        assert!(h2.confidence > 0.0);
    }

    #[test]
    fn refuted_hypothesis_deactivates() {
        let mut graph = BeliefGraph::new();
        graph.add_hypothesis(Hypothesis::new("H-001", "X improves Y", "space"));

        // Strongly refute.
        graph.record_experiment("E-001", "H-001", 1, ExperimentVerdict::Refuted, 1.0);
        graph.record_experiment("E-002", "H-001", 2, ExperimentVerdict::Refuted, 1.0);
        graph.record_experiment("E-003", "H-001", 3, ExperimentVerdict::Refuted, 1.0);

        let h = graph.hypotheses.get("H-001").unwrap();
        assert!(!h.active);
    }

    #[test]
    fn evaluate_verdict_improvement() {
        let h = Hypothesis::new("H-001", "coop improves tok/s", "space");
        let v = evaluate_verdict(&h, 60.0, 40.0, 0.20);
        assert_eq!(v, ExperimentVerdict::Confirmed);

        let v = evaluate_verdict(&h, 41.0, 40.0, 0.20);
        assert_eq!(v, ExperimentVerdict::Inconclusive);

        let v = evaluate_verdict(&h, 30.0, 40.0, 0.20);
        assert_eq!(v, ExperimentVerdict::Refuted);
    }

    #[test]
    fn evaluate_verdict_regression() {
        let h =
            Hypothesis::new("H-001", "naive GEMV regresses tok/s", "space").expects_regression();
        let v = evaluate_verdict(&h, 30.0, 40.0, 0.20);
        assert_eq!(v, ExperimentVerdict::Confirmed);

        let v = evaluate_verdict(&h, 60.0, 40.0, 0.20);
        assert_eq!(v, ExperimentVerdict::Refuted);
    }

    #[test]
    fn next_to_test_picks_uncertain() {
        let mut graph = BeliefGraph::new();
        graph.add_hypothesis(Hypothesis::new("H-001", "certain claim", "space"));
        graph.add_hypothesis(Hypothesis::new("H-002", "uncertain claim", "space"));

        // Make H-001 very confident.
        graph.record_experiment("E-001", "H-001", 1, ExperimentVerdict::Confirmed, 1.0);
        graph.record_experiment("E-002", "H-001", 2, ExperimentVerdict::Confirmed, 1.0);

        // H-002 is still at 0.0 — most uncertain.
        let next = graph.next_to_test();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, "H-002");
    }

    #[test]
    fn belief_graph_save_load() {
        let mut graph = BeliefGraph::new();
        graph.add_hypothesis(Hypothesis::new("H-001", "test", "space"));
        graph.record_experiment("E-001", "H-001", 42, ExperimentVerdict::Confirmed, 0.5);

        let tmp =
            std::env::temp_dir().join(format!("qualia_belief_test_{}.json", std::process::id()));
        graph.save(&tmp).unwrap();
        let loaded = BeliefGraph::load(&tmp).unwrap();
        assert!(loaded.hypotheses.contains_key("H-001"));
        assert!(loaded.experiments.contains_key("E-001"));
        let _ = std::fs::remove_file(&tmp);
    }
}

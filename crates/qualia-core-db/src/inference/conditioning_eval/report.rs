//! Paired comparison reporting for baseline vs compiled conditioning.

use super::scoring::TaskScore;

#[derive(Debug, Clone)]
pub struct PairedComparison {
    pub task_id: String,
    pub baseline_score: TaskScore,
    pub compiled_score: TaskScore,
}

impl PairedComparison {
    pub fn delta(&self) -> f32 {
        self.compiled_score.composite_score() - self.baseline_score.composite_score()
    }
}

#[derive(Debug, Clone)]
pub struct EvaluationReport {
    pub run_id: String,
    pub comparisons: Vec<PairedComparison>,
}

impl EvaluationReport {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            comparisons: Vec::new(),
        }
    }

    pub fn average_improvement(&self) -> f32 {
        if self.comparisons.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.comparisons.iter().map(|c| c.delta()).sum();
        sum / self.comparisons.len() as f32
    }
}

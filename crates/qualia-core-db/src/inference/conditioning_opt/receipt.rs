//! Receipts for model-specific precision optimization campaigns.

use crate::inference::conditioning_eval::TaskScore;

/// Immutable receipt emitted upon completion of a model precision optimization run.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModelOptimizationReceipt {
    pub model_id: String,
    pub run_id: String,
    pub target_domain: String,
    pub baseline_variant: String,
    pub winning_variant: String,
    pub baseline_score: TaskScore,
    pub optimized_score: TaskScore,
    pub held_out_test_score: TaskScore,
    pub delta_improvement_pct: f32,
    pub winning_spec_identity: u64,
    pub promoted_version: u64,
    pub previous_version: Option<u64>,
}

impl ModelOptimizationReceipt {
    /// Calculate percentage improvement over baseline composite score.
    pub fn calculate_improvement(baseline: f32, optimized: f32) -> f32 {
        if baseline <= 0.0 {
            if optimized > 0.0 {
                100.0
            } else {
                0.0
            }
        } else {
            ((optimized - baseline) / baseline) * 100.0
        }
    }
}

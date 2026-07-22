//! End-to-end optimization campaign orchestration.
//!
//! Ties together the five lab layers into a single pipeline:
//! 1. `SearchEngine` proposes configurations (Sobol + Bayesian EI)
//! 2. `run_experiment_with_quality()` executes each trial
//! 3. `ParetoFrontier` computes the 6-dimensional non-dominated set
//! 4. `BeliefGraph` records verdicts and cascades confidence
//! 5. Results are persisted to JSONL for reproducibility
//!
//! The campaign runs in batches: after each batch, the frontier is recomputed
//! and the belief graph is updated. The search engine adapts based on all
//! observations so far.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::config_space::{Configuration, ConfigurationSpace, ParameterDef};
use super::experiment::{
    append_experiment_jsonl, load_experiment_log, ExperimentConfig, ExperimentResult,
};
use super::hypothesis::{evaluate_verdict, BeliefGraph, Hypothesis};
use super::pareto::{ApplicationProfileWeight, ParetoFrontier};
use super::search::SearchEngine;

/// Configuration for an optimization campaign.
#[derive(Debug, Clone)]
pub struct CampaignConfig {
    /// The configuration space to search over.
    pub space: ConfigurationSpace,
    /// Model path for benchmarking.
    pub model_path: String,
    /// Quantization label.
    pub quantization: String,
    /// Prompt for quality evaluation.
    pub prompt: String,
    /// Decode token budget (0 = production default).
    pub decode_tokens: u32,
    /// Warm repeats for benchmark.
    pub warm_repeats: u32,
    /// Total evaluation budget (number of trials).
    pub budget: usize,
    /// Batch size: after each batch, recompute frontier + update beliefs.
    pub batch_size: usize,
    /// Application profile for best-config selection.
    pub profile: ApplicationProfileWeight,
    /// Improvement threshold for verdict evaluation (e.g. 0.20 = 20%).
    pub improvement_threshold: f64,
    /// Optional path to persist JSONL experiment log.
    pub jsonl_path: Option<PathBuf>,
    /// Optional path to persist belief graph JSON.
    pub belief_path: Option<PathBuf>,
    /// Use quality verification (decode_with_metrics + verify_and_heal_turn).
    /// If false, uses run_bench only (faster but no quality score).
    pub with_quality: bool,
    /// Max wall-clock duration for the campaign.
    pub max_duration: Option<Duration>,
}

impl CampaignConfig {
    /// Create a campaign config with sensible defaults.
    pub fn new(space: ConfigurationSpace, model_path: impl Into<String>, budget: usize) -> Self {
        Self {
            space,
            model_path: model_path.into(),
            quantization: "q4_k".into(),
            prompt: "What is the capital of France?".into(),
            decode_tokens: 32,
            warm_repeats: 3,
            budget,
            batch_size: 5,
            profile: ApplicationProfileWeight::Interactive,
            improvement_threshold: 0.20,
            jsonl_path: None,
            belief_path: None,
            with_quality: true,
            max_duration: None,
        }
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    pub fn with_profile(mut self, profile: ApplicationProfileWeight) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_quality(mut self, quality: bool) -> Self {
        self.with_quality = quality;
        self
    }

    pub fn with_jsonl_log(mut self, path: impl Into<PathBuf>) -> Self {
        self.jsonl_path = Some(path.into());
        self
    }

    pub fn with_belief_log(mut self, path: impl Into<PathBuf>) -> Self {
        self.belief_path = Some(path.into());
        self
    }

    pub fn with_batch_size(mut self, batch: usize) -> Self {
        self.batch_size = batch.max(1);
        self
    }

    pub fn with_max_duration(mut self, dur: Duration) -> Self {
        self.max_duration = Some(dur);
        self
    }
}

/// The result of an optimization campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignReport {
    /// All experiment results from the campaign.
    pub results: Vec<ExperimentResult>,
    /// The Pareto frontier (indices into `results`).
    pub frontier: ParetoFrontier,
    /// The belief graph after the campaign.
    pub beliefs: BeliefGraph,
    /// Index of the best result (selected by profile).
    pub best_index: Option<usize>,
    /// Number of trials actually run (may be less than budget if time-limited).
    pub trials_run: usize,
    /// Wall-clock duration of the campaign in seconds.
    pub elapsed_s: f64,
    /// Baseline throughput (tok/s) for comparison.
    pub baseline_tok_s: Option<f64>,
    /// Best throughput achieved (tok/s).
    pub best_tok_s: Option<f64>,
    /// Summary message for logging.
    pub summary: String,
}

impl CampaignReport {
    /// The best experiment result (selected by application profile).
    pub fn best_result(&self) -> Option<&ExperimentResult> {
        self.best_index.and_then(|i| self.results.get(i))
    }

    /// Improvement ratio: best / baseline.
    pub fn improvement_ratio(&self) -> Option<f64> {
        match (self.best_tok_s, self.baseline_tok_s) {
            (Some(best), Some(baseline)) if baseline > 0.0 => Some(best / baseline),
            _ => None,
        }
    }

    /// Serialize the report to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
}

/// Run an optimization campaign end-to-end.
///
/// This is the main entry point for the AI Inference Optimization Lab.
/// It runs `budget` trials, computing the Pareto frontier and updating the
/// belief graph after each batch.
pub fn run_optimization_campaign(cfg: &CampaignConfig) -> CampaignReport {
    let t_start = Instant::now();
    let mut engine = SearchEngine::new(cfg.space.clone(), cfg.budget);
    let mut beliefs = BeliefGraph::new();
    let mut results: Vec<ExperimentResult> = Vec::new();

    // Load existing results from JSONL if present (resume capability).
    if let Some(ref jsonl) = cfg.jsonl_path {
        if let Ok(existing) = load_experiment_log(jsonl) {
            for r in &existing {
                if let Ok(config) = Configuration::from_cbor(&r.config_cbor) {
                    let normalized = cfg.space.normalize_config(&config);
                    let objective = compute_objective(r);
                    engine.surrogate_add(normalized, objective, r.config_hash);
                }
            }
            results = existing;
            log::info!("campaign|resumed|{} existing results", results.len());
        }
    }

    // Run a baseline measurement if we have no results yet.
    let baseline_tok_s = if results.is_empty() {
        let baseline_cfg = cfg.space.default_config();
        let exp_cfg = ExperimentConfig {
            space: cfg.space.clone(),
            config: baseline_cfg,
            model_path: cfg.model_path.clone(),
            quantization: cfg.quantization.clone(),
            prompt: cfg.prompt.clone(),
            decode_tokens: cfg.decode_tokens,
            warm_repeats: cfg.warm_repeats,
            seed: 0,
            hypothesis_id: None,
        };
        let baseline_result = if cfg.with_quality {
            super::experiment::run_experiment_with_quality(&exp_cfg)
        } else {
            super::experiment::run_experiment(&exp_cfg)
        };
        let bt = baseline_result.bench.as_ref().map(|b| b.decode_tok_s);
        results.push(baseline_result);
        bt
    } else {
        results[0].bench.as_ref().map(|b| b.decode_tok_s)
    };

    // Main search loop.
    let mut trials_run = 0;
    while trials_run < cfg.budget {
        // Check time budget.
        if let Some(max_dur) = cfg.max_duration {
            if t_start.elapsed() >= max_dur {
                log::info!("campaign|time_budget_exhausted|{}s", t_start.elapsed().as_secs_f64());
                break;
            }
        }

        // Ask the search engine for the next configuration.
        let config = match engine.ask() {
            Some(c) => c,
            None => break,
        };

        // Build the experiment config.
        let exp_cfg = ExperimentConfig {
            space: cfg.space.clone(),
            config: config.clone(),
            model_path: cfg.model_path.clone(),
            quantization: cfg.quantization.clone(),
            prompt: cfg.prompt.clone(),
            decode_tokens: cfg.decode_tokens,
            warm_repeats: cfg.warm_repeats,
            seed: trials_run as u64,
            hypothesis_id: None,
        };

        // Run the experiment.
        let result = if cfg.with_quality {
            super::experiment::run_experiment_with_quality(&exp_cfg)
        } else {
            super::experiment::run_experiment(&exp_cfg)
        };

        // Tell the search engine the outcome.
        engine.tell(&config, &result);

        // Persist to JSONL.
        if let Some(ref jsonl) = cfg.jsonl_path {
            if let Err(e) = append_experiment_jsonl(jsonl, &result) {
                log::warn!("campaign|jsonl_append_failed|{e}");
            }
        }

        results.push(result);
        trials_run += 1;

        // After each batch: recompute frontier + update beliefs.
        if trials_run % cfg.batch_size == 0 {
            update_beliefs(&mut beliefs, &results, baseline_tok_s, cfg.improvement_threshold);
            log::info!(
                "campaign|batch_complete|trials={}|frontier={}|beliefs={}",
                trials_run,
                ParetoFrontier::compute(&results).frontier_size(),
                beliefs.hypotheses.len(),
            );
        }
    }

    // Final frontier computation.
    let frontier = ParetoFrontier::compute(&results);

    // Final belief update.
    update_beliefs(&mut beliefs, &results, baseline_tok_s, cfg.improvement_threshold);

    // Select best result by application profile.
    let best_index = cfg.profile.select_best(&results, &frontier)
        .and_then(|r| results.iter().position(|x| x.config_hash == r.config_hash));

    let best_tok_s = best_index
        .and_then(|i| results[i].bench.as_ref())
        .map(|b| b.decode_tok_s);

    // Persist belief graph.
    if let Some(ref belief_path) = cfg.belief_path {
        if let Err(e) = beliefs.save(belief_path) {
            log::warn!("campaign|belief_save_failed|{e}");
        }
    }

    let elapsed_s = t_start.elapsed().as_secs_f64();

    // Build summary.
    let improvement = match (best_tok_s, baseline_tok_s) {
        (Some(best), Some(base)) if base > 0.0 => format!("{:.1}% improvement", (best / base - 1.0) * 100.0),
        _ => "no baseline".to_string(),
    };
    let summary = format!(
        "trials={}, frontier={}, beliefs={}, {}",
        trials_run,
        frontier.frontier_size(),
        beliefs.hypotheses.len(),
        improvement,
    );

    CampaignReport {
        results,
        frontier,
        beliefs,
        best_index,
        trials_run,
        elapsed_s,
        baseline_tok_s,
        best_tok_s,
        summary,
    }
}

/// Update the belief graph from experiment results.
/// For each result, evaluate the verdict against the baseline and record it.
fn update_beliefs(
    beliefs: &mut BeliefGraph,
    results: &[ExperimentResult],
    baseline_tok_s: Option<f64>,
    threshold: f64,
) {
    // Create a default hypothesis if none exist.
    if beliefs.hypotheses.is_empty() {
        let h = Hypothesis::new(
            "H-default",
            "Search finds configurations that improve decode throughput",
            "campaign",
        );
        beliefs.add_hypothesis(h);
    }

    let baseline = match baseline_tok_s {
        Some(b) if b > 0.0 => b,
        _ => return,
    };

    // Evaluate each result (skip the baseline, which is results[0]).
    for (i, r) in results.iter().enumerate().skip(1) {
        if r.error.is_some() {
            continue;
        }
        let tok_s = match r.bench.as_ref() {
            Some(b) => b.decode_tok_s,
            None => continue,
        };
        if tok_s <= 0.0 {
            continue;
        }

        // Check if we already recorded this experiment.
        let exp_id = format!("E-campaign-{i}");
        if beliefs.experiments.contains_key(&exp_id) {
            continue;
        }

        let h = beliefs.hypotheses.get("H-default").unwrap();
        let verdict = evaluate_verdict(h, tok_s, baseline, threshold);
        let weight = 0.5 + (tok_s / baseline).min(2.0) * 0.25; // weight scales with improvement
        beliefs.record_experiment(exp_id, "H-default", r.config_hash, verdict, weight);
    }
}

/// Compute a scalar objective from an experiment result.
/// Used by the search engine's surrogate model.
fn compute_objective(r: &ExperimentResult) -> f64 {
    if r.error.is_some() {
        return f64::NEG_INFINITY;
    }
    let tok_s = r.bench.as_ref().map(|b| b.decode_tok_s).unwrap_or(0.0);
    let quality = r.quality.composite();
    tok_s * quality
}

/// Build a default configuration space for inference toggle optimization.
/// This covers the main toggle parameters that `ExperimentConfig::apply_toggles` supports.
pub fn default_toggle_space() -> ConfigurationSpace {
    ConfigurationSpace::new("inference_toggles")
        .with("coop_gemv", ParameterDef::Bool)
        .with("ffn_fusion", ParameterDef::Bool)
        .with("kv_int8", ParameterDef::Bool)
        .with("kv_dict", ParameterDef::Bool)
        .with("resident_decode", ParameterDef::Bool)
        .with("resident_prefill", ParameterDef::Bool)
        .with("resident_weights", ParameterDef::Bool)
        .with("spec_decode", ParameterDef::Bool)
        .with("ternary_ffn", ParameterDef::Bool)
        .with("attention_preproject", ParameterDef::Bool)
        .with("attention_o_fuse", ParameterDef::Bool)
        .with("gpu_topk", ParameterDef::Bool)
}

/// Save a campaign report to a JSON file.
pub fn save_campaign_report(report: &CampaignReport, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = report.to_json();
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toggle_space_has_dims() {
        let space = default_toggle_space();
        assert!(space.dims() >= 10);
    }

    #[test]
    fn campaign_config_builder() {
        let space = default_toggle_space();
        let cfg = CampaignConfig::new(space, "model.gguf", 20)
            .with_prompt("Hello")
            .with_profile(ApplicationProfileWeight::LiveFast)
            .with_batch_size(4);

        assert_eq!(cfg.budget, 20);
        assert_eq!(cfg.batch_size, 4);
        assert_eq!(cfg.profile, ApplicationProfileWeight::LiveFast);
        assert_eq!(cfg.prompt, "Hello");
    }

    #[test]
    fn campaign_report_json_serializes() {
        let report = CampaignReport {
            results: Vec::new(),
            frontier: ParetoFrontier::default(),
            beliefs: BeliefGraph::new(),
            best_index: None,
            trials_run: 0,
            elapsed_s: 0.0,
            baseline_tok_s: Some(50.0),
            best_tok_s: Some(75.0),
            summary: "test".into(),
        };
        let json = report.to_json();
        assert!(json.contains("trials_run"));
        assert!(json.contains("baseline_tok_s"));
    }

    #[test]
    fn improvement_ratio_computes() {
        let report = CampaignReport {
            results: Vec::new(),
            frontier: ParetoFrontier::default(),
            beliefs: BeliefGraph::new(),
            best_index: None,
            trials_run: 0,
            elapsed_s: 0.0,
            baseline_tok_s: Some(50.0),
            best_tok_s: Some(75.0),
            summary: "test".into(),
        };
        assert_eq!(report.improvement_ratio(), Some(1.5));
    }

    #[test]
    fn save_and_load_report() {
        let report = CampaignReport {
            results: Vec::new(),
            frontier: ParetoFrontier::default(),
            beliefs: BeliefGraph::new(),
            best_index: None,
            trials_run: 5,
            elapsed_s: 12.5,
            baseline_tok_s: Some(40.0),
            best_tok_s: Some(60.0),
            summary: "test save".into(),
        };
        let tmp = std::env::temp_dir().join(format!("qualia_campaign_test_{}.json", std::process::id()));
        save_campaign_report(&report, &tmp).unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("trials_run"));
        assert!(content.contains("12.5"));
        let _ = std::fs::remove_file(&tmp);
    }
}

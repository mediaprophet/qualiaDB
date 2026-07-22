//! Inference superiority **lab** — instruments for evidence-based methodology.
//!
//! Plan: `docs/plans/inference-superiority-lab-and-toolset-plan.md`
//!
//! Native-only. No Ollama product dependency; optional external A/B via CLI script.

#![cfg(not(target_arch = "wasm32"))]

pub mod audit_path;
pub mod auto_improve;
pub mod device_roof;
pub mod experiment_log;
pub mod micro;
pub mod timeline;
pub mod ablate;

// AI Inference Optimization Lab — new architecture layers.
pub mod config_space;
pub mod experiment;
pub mod pareto;
pub mod search;
pub mod hypothesis;
pub mod campaign;

pub use ablate::{run_ablation_matrix, AblationRow};
pub use audit_path::{audit_hot_path, HotPathAudit};
pub use auto_improve::{
    default_search_space, format_lockin_summary, run_auto_improve, AutoImproveConfig, LabConfig,
    LockInPackage, TrialResult,
};
pub use device_roof::{calibrate_device_roof, DeviceRoof};
pub use experiment_log::{append_run_csv, ExperimentRun, CSV_HEADER};
pub use micro::{run_q4k_soa_microbench, MicrobenchResult};
pub use timeline::{run_decode_timeline, DecodeTimeline};

// Re-exports for the new optimization lab.
pub use config_space::{Configuration, ConfigurationSpace, ParameterDef, ParameterValue};
pub use experiment::{
    run_experiment, run_experiment_with_quality, append_experiment_jsonl,
    load_experiment_log, ExperimentConfig, ExperimentResult, BenchResultSerde,
    PhaseSnapshotSerde, QualityScore, ThermalSnapshot,
};
pub use pareto::{ParetoFrontier, ParetoPoint, ApplicationProfileWeight};
pub use search::{SearchEngine, SobolSequence, KnnSurrogate, TrackAndStopBandit, expected_improvement};
pub use hypothesis::{BeliefGraph, Hypothesis, ExperimentVerdict, evaluate_verdict};
pub use campaign::{
    run_optimization_campaign, save_campaign_report, default_toggle_space,
    CampaignConfig, CampaignReport,
};

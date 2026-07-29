//! Inference superiority **lab** — instruments for evidence-based methodology.
//!
//! Plan: `docs/plans/inference-superiority-lab-and-toolset-plan.md`
//!
//! Native-only. No Ollama product dependency; optional external A/B via CLI script.

#![cfg(not(target_arch = "wasm32"))]

pub mod ablate;
pub mod audit_path;
pub mod auto_improve;
pub mod device_roof;
pub mod experiment_log;
pub mod micro;
pub mod timeline;

// AI Inference Optimization Lab — new architecture layers.
pub mod campaign;
pub mod config_space;
pub mod experiment;
pub mod hypothesis;
pub mod pareto;
pub mod search;

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
pub use campaign::{
    default_toggle_space, run_optimization_campaign, save_campaign_report, CampaignConfig,
    CampaignReport,
};
pub use config_space::{Configuration, ConfigurationSpace, ParameterDef, ParameterValue};
pub use experiment::{
    append_experiment_jsonl, load_experiment_log, run_experiment, run_experiment_with_quality,
    BenchResultSerde, ExperimentConfig, ExperimentResult, PhaseSnapshotSerde, QualityScore,
    ThermalSnapshot,
};
pub use hypothesis::{evaluate_verdict, BeliefGraph, ExperimentVerdict, Hypothesis};
pub use pareto::{ApplicationProfileWeight, ParetoFrontier, ParetoPoint};
pub use search::{
    expected_improvement, KnnSurrogate, SearchEngine, SobolSequence, TrackAndStopBandit,
};

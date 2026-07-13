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

//! Deterministic WGSL generation, validation, certification, and tuning.
//!
//! WGSL Forge treats shader semantics and hardware scheduling as separate typed
//! inputs. Tuning is therefore allowed to change work distribution without
//! changing the mathematical operation being certified.

pub mod cache;
pub mod emit;
pub mod execute;
pub mod ir;
pub mod manifest;
pub mod oracle;
pub mod schedule;
pub mod tune;
pub mod validate;

pub use cache::ManifestCache;
pub use emit::{emit_shader, GeneratedShader, TargetBackend};
pub use ir::{
    BufferAccess, BufferElement, BufferSpec, BuiltinKernel, KernelSpec, Op,
    P64GpuWords64, ScalarType, SharedLen, SharedMemorySpec,
};
pub use manifest::{
    AdapterIdentity, CertificationManifest, TimingSource, TimingSummary, TuningManifest,
    ValidationLevel,
};
pub use oracle::{
    candidate_evaluation, certify_builtin, compare_f32, evaluate_builtin, evaluate_topk,
    topk_cpu, topk_inputs, AffineParams, ComparisonReport, GpuEvaluation, OracleCase,
    OracleTolerance, TopKParams,
};
pub use schedule::{AdapterConstraints, Schedule, ScheduleSpace};
pub use tune::{
    tune_with, CandidateEvaluation, CandidateFailure, CandidateResult, TuningConfig, TuningResult,
};
pub use validate::{validate_wgsl, validate_native, ValidationReport};

pub const FORGE_SCHEMA_VERSION: u32 = 1;
pub const WGPU_API_VERSION: &str = "29.0.3";
pub const NAGA_API_VERSION: &str = "29.0.3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeError {
    UnknownKernel(String),
    InvalidKernel(String),
    InvalidSchedule(String),
    Emission(String),
    WgslParse(String),
    WgslValidation(String),
    GpuUnavailable(String),
    GpuValidation(String),
    OracleMismatch(String),
    Serialization(String),
    Io(String),
}

impl core::fmt::Display for ForgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (kind, message) = match self {
            Self::UnknownKernel(message) => ("unknown kernel", message),
            Self::InvalidKernel(message) => ("invalid kernel", message),
            Self::InvalidSchedule(message) => ("invalid schedule", message),
            Self::Emission(message) => ("WGSL emission", message),
            Self::WgslParse(message) => ("WGSL parse", message),
            Self::WgslValidation(message) => ("WGSL validation", message),
            Self::GpuUnavailable(message) => ("GPU unavailable", message),
            Self::GpuValidation(message) => ("GPU validation", message),
            Self::OracleMismatch(message) => ("oracle mismatch", message),
            Self::Serialization(message) => ("serialization", message),
            Self::Io(message) => ("I/O", message),
        };
        write!(f, "{kind}: {message}")
    }
}

impl std::error::Error for ForgeError {}

impl From<std::io::Error> for ForgeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for ForgeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}

pub fn generate_builtin(
    builtin: BuiltinKernel,
    schedule: Schedule,
    target: TargetBackend,
) -> Result<GeneratedShader, ForgeError> {
    let kernel = builtin.spec();
    schedule.validate(&kernel, &AdapterConstraints::portable())?;
    emit_shader(&kernel, schedule, target)
}

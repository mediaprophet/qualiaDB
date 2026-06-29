//! Deterministic WGSL generation, validation, certification, and tuning.
//!
//! WGSL Forge treats shader semantics and hardware scheduling as separate typed
//! inputs. Tuning is therefore allowed to change work distribution without
//! changing the mathematical operation being certified.

pub mod backend;
pub mod cache;
pub mod dispatch;
pub mod emit;
pub mod execute;
pub mod ir;
pub mod manifest;
pub mod oracle;
pub mod roofline;
pub mod runtime;
pub mod schedule;
pub mod tune;
pub mod validate;

pub use backend::resolve_execution_backend;
pub use cache::ManifestCache;
pub use dispatch::{
    caps, fft_f32, gemm_cpu_f64, gemm_f32, gemm_f64, gemm_f64_df64, gemv_cpu_f64, gemv_f32,
    gemv_f64, ComputeCaps, GEMM_GPU_THRESHOLD,
};
pub use emit::{decode_spirv_words, emit_shader, matmul_tc_wgsl, GeneratedShader, TargetBackend};
pub use ir::{
    BufferAccess, BufferElement, BufferSpec, BuiltinKernel, KernelSpec, Op,
    P64GpuWords64, ScalarType, SharedLen, SharedMemorySpec,
};
pub use manifest::{
    AdapterIdentity, CertificationManifest, HardwareProfile, TimingSource, TimingSummary,
    TuningManifest, ValidationLevel,
};
pub use oracle::{
    candidate_evaluation, certify_builtin, compare_f32, dft_cpu, evaluate_builtin, evaluate_ffn,
    evaluate_fft, evaluate_matmul_tc, evaluate_p64, evaluate_topk, ffn_cpu, ffn_tensors,
    fft_inputs, matmul_cpu, p64_project_cpu, p64_records, topk_cpu,
    topk_inputs, AffineParams, ComparisonReport, FfnParams, FftParams, GpuEvaluation, OracleCase,
    OracleTolerance, TopKParams,
};
#[cfg(feature = "cuda")]
pub use oracle::{evaluate_affine_cuda, evaluate_ffn_cuda, evaluate_topk_cuda};
pub use roofline::{roofline_for, RooflineBound, RooflineEstimate};
pub use runtime::ForgeRuntime;
pub use schedule::{AdapterConstraints, Schedule, ScheduleSpace};
pub use tune::{
    tune_with, CandidateEvaluation, CandidateFailure, CandidateResult, TuningConfig, TuningResult,
};
pub use validate::{validate_wgsl, validate_native, ValidationReport};

pub const FORGE_SCHEMA_VERSION: u32 = 2;
pub const WGPU_API_VERSION: &str = "29.0.3";
pub const NAGA_API_VERSION: &str = "29.0.3";
/// `cudarc` crate API version the cross-backend (CUDA) oracle is built against.
/// Folded into the tuning/certification cache key (plan §8) so reuse is
/// invalidated when the CUDA toolchain surface changes.
pub const CUDARC_API_VERSION: &str = "0.19";

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
    /// The compute device was lost (driver reset, removal, or a fatal poll
    /// failure). Unified across backends so callers handle one variant rather
    /// than backend-specific panics (plan §7).
    DeviceLost(String),
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
            Self::DeviceLost(message) => ("device lost", message),
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

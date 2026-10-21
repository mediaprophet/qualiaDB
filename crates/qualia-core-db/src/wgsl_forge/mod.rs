//! Deterministic WGSL generation, validation, certification, and tuning.
//!
//! WGSL Forge treats shader semantics and hardware scheduling as separate typed
//! inputs. Tuning is therefore allowed to change work distribution without
//! changing the mathematical operation being certified.

pub mod attention;
pub mod audio;
pub mod backend;
// W10: the forge's calibration/adaptation pipeline (corpus→capture→learn→certify→package). Native-
// only (drives the real inference stack for capture + the PPL oracle). The third produce-and-certify
// entry point beside kernel certification and GGUF→p64 transcode.
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod calibration;
pub mod dispatch;
pub mod emit;
pub mod execute;
pub mod graph_ops;
pub mod ir;
pub mod manifest;
pub mod observer_kernels;
pub mod oracle;
pub mod physics;
pub mod roofline;
pub mod runtime;
pub mod schedule;
pub mod tune;
pub mod validate;

#[cfg(test)]
/// `?::test_gpu_available`.
pub(crate) fn test_gpu_available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| execute::WgpuComputeContext::new(64 * 1024).is_ok())
}

#[cfg(all(test, feature = "cuda"))]
/// `?::test_cuda_available`.
pub(crate) fn test_cuda_available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| execute::CudaComputeContext::new(64 * 1024).is_ok())
}

pub use attention::{
    causal_attention_oracle, synthesize_causal_attention, AttentionSchedule, AttentionShader,
    AttentionShaderParams, AttentionShape,
};
pub use backend::resolve_execution_backend;
pub use cache::ManifestCache;
pub use dispatch::{
    caps, coopmat_usable, fft_f32, gemm_cpu_f64, gemm_f32, gemm_f32_tc, gemm_f32_tc_coopmat,
    gemm_f32_tc_reduced, gemm_f64, gemm_f64_df64, gemv_cpu_f64, gemv_f32, gemv_f64,
    pairwise_sq_dist_cpu_f64, pairwise_sq_dist_f64, ComputeCaps, GEMM_GPU_THRESHOLD,
};
pub use emit::{decode_spirv_words, emit_shader, matmul_tc_wgsl, GeneratedShader, TargetBackend};
pub use ir::{
    BufferAccess, BufferElement, BufferSpec, BuiltinKernel, KernelSpec, Op, P64GpuWords64,
    ScalarType, SharedLen, SharedMemorySpec,
};
pub use manifest::{
    AdapterIdentity, CertificationManifest, HardwareProfile, TimingSource, TimingSummary,
    TuningManifest, ValidationLevel,
};
pub use oracle::{
    candidate_evaluation, certify_builtin, compare_f32, dft_cpu, evaluate_builtin, evaluate_ffn,
    evaluate_fft, evaluate_matmul_tc, evaluate_p64, evaluate_topk, ffn_cpu, ffn_tensors,
    fft_inputs, matmul_cpu, p64_project_cpu, p64_records, topk_cpu, topk_inputs, AffineParams,
    ComparisonReport, FfnParams, FftParams, GpuEvaluation, OracleCase, OracleTolerance, TopKParams,
};
#[cfg(feature = "cuda")]
pub use oracle::{evaluate_affine_cuda, evaluate_ffn_cuda, evaluate_topk_cuda};
pub use roofline::{roofline_for, RooflineBound, RooflineEstimate};
pub use runtime::ForgeRuntime;
pub use schedule::{AdapterConstraints, Schedule, ScheduleSpace};
pub use tune::{
    tune_with, CandidateEvaluation, CandidateFailure, CandidateResult, TuningConfig, TuningResult,
};
pub use validate::{validate_native, validate_wgsl, ValidationReport};
pub use observer_kernels::*;

#[cfg(feature = "gpu-runtime")]
pub mod etalon_bridge {
    use super::{BuiltinKernel, ForgeError, Schedule, TargetBackend};

    pub use etalon_wgsl_forge::{
        certify_kernel as certify_etalon_kernel, emit_shader as emit_etalon_shader,
        BuiltinKernel as EtalonBuiltinKernel, GeneratedShader as EtalonGeneratedShader,
        KernelSpec as EtalonKernelSpec, Schedule as EtalonSchedule,
        TargetBackend as EtalonTargetBackend, FORGE_CACHE_MAGIC as ETALON_FORGE_CACHE_MAGIC,
        FORGE_SCHEMA_VERSION as ETALON_FORGE_SCHEMA_VERSION,
    };

    // ── TargetBackend conversions ───────────────────────────────────────

    impl From<TargetBackend> for etalon_wgsl_forge::TargetBackend {
        fn from(target: TargetBackend) -> Self {
            match target {
                TargetBackend::Wgsl => Self::Wgsl,
                TargetBackend::Msl => Self::Msl,
                TargetBackend::Hlsl => Self::Hlsl,
                TargetBackend::Ptx => Self::Ptx,
                TargetBackend::CudaC => Self::CudaC,
                TargetBackend::Spirv => Self::Spirv,
            }
        }
    }

    impl From<etalon_wgsl_forge::TargetBackend> for TargetBackend {
        fn from(target: etalon_wgsl_forge::TargetBackend) -> Self {
            match target {
                etalon_wgsl_forge::TargetBackend::Wgsl => Self::Wgsl,
                etalon_wgsl_forge::TargetBackend::Msl => Self::Msl,
                etalon_wgsl_forge::TargetBackend::Hlsl => Self::Hlsl,
                etalon_wgsl_forge::TargetBackend::Ptx => Self::Ptx,
                etalon_wgsl_forge::TargetBackend::CudaC
                | etalon_wgsl_forge::TargetBackend::Cuda => Self::CudaC,
                etalon_wgsl_forge::TargetBackend::Spirv => Self::Spirv,
            }
        }
    }

    pub fn to_etalon_target(target: TargetBackend) -> etalon_wgsl_forge::TargetBackend {
        target.into()
    }

    pub fn from_etalon_target(target: etalon_wgsl_forge::TargetBackend) -> TargetBackend {
        target.into()
    }

    // ── BuiltinKernel conversions ───────────────────────────────────────

    impl From<BuiltinKernel> for etalon_wgsl_forge::BuiltinKernel {
        fn from(kernel: BuiltinKernel) -> Self {
            match kernel {
                BuiltinKernel::AffineF32 => Self::AffineF32,
                BuiltinKernel::FusedFfn => Self::FusedFfn,
                BuiltinKernel::P64Project => Self::P64Project,
                BuiltinKernel::TopK => Self::TopK,
                BuiltinKernel::RayProbe => Self::RayProbe,
                BuiltinKernel::TernaryGemv => Self::TernaryGemv,
                BuiltinKernel::Gemm => Self::Gemm,
                BuiltinKernel::Gemv => Self::Gemv,
                BuiltinKernel::Fft => Self::Fft,
            }
        }
    }

    impl From<etalon_wgsl_forge::BuiltinKernel> for BuiltinKernel {
        fn from(kernel: etalon_wgsl_forge::BuiltinKernel) -> Self {
            match kernel {
                etalon_wgsl_forge::BuiltinKernel::AffineF32 => Self::AffineF32,
                etalon_wgsl_forge::BuiltinKernel::FusedFfn => Self::FusedFfn,
                etalon_wgsl_forge::BuiltinKernel::P64Project => Self::P64Project,
                etalon_wgsl_forge::BuiltinKernel::TopK => Self::TopK,
                etalon_wgsl_forge::BuiltinKernel::RayProbe => Self::RayProbe,
                etalon_wgsl_forge::BuiltinKernel::TernaryGemv => Self::TernaryGemv,
                etalon_wgsl_forge::BuiltinKernel::Gemm => Self::Gemm,
                etalon_wgsl_forge::BuiltinKernel::Gemv => Self::Gemv,
                etalon_wgsl_forge::BuiltinKernel::Fft => Self::Fft,
            }
        }
    }

    pub fn to_etalon_builtin(kernel: BuiltinKernel) -> etalon_wgsl_forge::BuiltinKernel {
        kernel.into()
    }

    pub fn from_etalon_builtin(kernel: etalon_wgsl_forge::BuiltinKernel) -> BuiltinKernel {
        kernel.into()
    }

    // ── Schedule conversions ────────────────────────────────────────────

    impl From<Schedule> for etalon_wgsl_forge::Schedule {
        fn from(schedule: Schedule) -> Self {
            etalon_wgsl_forge::Schedule::from_1d(
                schedule.workgroup_size,
                schedule.items_per_invocation,
                schedule.vector_width,
            )
        }
    }

    impl From<etalon_wgsl_forge::Schedule> for Schedule {
        fn from(schedule: etalon_wgsl_forge::Schedule) -> Self {
            Schedule {
                workgroup_size: schedule.effective_workgroup_size(),
                items_per_invocation: schedule.items_per_invocation,
                vector_width: schedule.vector_width,
            }
        }
    }

    pub fn to_etalon_schedule(schedule: Schedule) -> etalon_wgsl_forge::Schedule {
        schedule.into()
    }

    pub fn from_etalon_schedule(schedule: etalon_wgsl_forge::Schedule) -> Schedule {
        schedule.into()
    }

    // ── GeneratedShader conversions ─────────────────────────────────────

    impl From<etalon_wgsl_forge::GeneratedShader> for super::GeneratedShader {
        fn from(s: etalon_wgsl_forge::GeneratedShader) -> Self {
            Self {
                kernel_id: s.kernel_id,
                semantic_hash: s.semantic_hash,
                source_hash: s.source_hash,
                schedule: s.schedule.into(),
                source: s.source,
            }
        }
    }

    impl From<super::GeneratedShader> for etalon_wgsl_forge::GeneratedShader {
        fn from(s: super::GeneratedShader) -> Self {
            Self {
                kernel_id: s.kernel_id,
                semantic_hash: s.semantic_hash,
                source_hash: s.source_hash,
                schedule: s.schedule.into(),
                source: s.source,
            }
        }
    }

    /// Emits and certifies a kernel via `etalon_wgsl_forge`.
    ///
    /// Maps `builtin`, `schedule`, and `target` to their Etalon counterparts, emits
    /// the shader source via `etalon_wgsl_forge::emit_shader`, certifies WGSL via
    /// `etalon_wgsl_forge::certify_kernel` when target is `TargetBackend::Wgsl`,
    /// and returns the resulting generated shader.
    pub fn generate_and_certify_via_etalon(
        builtin: BuiltinKernel,
        schedule: Schedule,
        target: TargetBackend,
    ) -> Result<etalon_wgsl_forge::GeneratedShader, ForgeError> {
        let etalon_builtin: etalon_wgsl_forge::BuiltinKernel = builtin.into();
        let etalon_schedule: etalon_wgsl_forge::Schedule = schedule.into();
        let etalon_target: etalon_wgsl_forge::TargetBackend = target.into();

        let spec = etalon_builtin.spec();
        let generated = etalon_wgsl_forge::emit_shader(&spec, etalon_schedule, etalon_target)
            .map_err(ForgeError::from)?;

        if matches!(target, TargetBackend::Wgsl) {
            etalon_wgsl_forge::certify_kernel(&generated.source)
                .map_err(ForgeError::from)?;
        }

        Ok(generated)
    }
}

#[cfg(feature = "gpu-runtime")]
pub use etalon_bridge::{
    certify_etalon_kernel, emit_etalon_shader, generate_and_certify_via_etalon,
    EtalonBuiltinKernel, EtalonGeneratedShader, EtalonKernelSpec, EtalonSchedule,
    EtalonTargetBackend, ETALON_FORGE_CACHE_MAGIC, ETALON_FORGE_SCHEMA_VERSION,
};

#[cfg(feature = "gpu-runtime")]
impl From<etalon_wgsl_forge::ForgeError> for ForgeError {
    fn from(value: etalon_wgsl_forge::ForgeError) -> Self {
        match value {
            etalon_wgsl_forge::ForgeError::UnknownKernel(msg) => Self::UnknownKernel(msg),
            etalon_wgsl_forge::ForgeError::InvalidKernel(msg) => Self::InvalidKernel(msg),
            etalon_wgsl_forge::ForgeError::InvalidSchedule(msg) => Self::InvalidSchedule(msg),
            etalon_wgsl_forge::ForgeError::Emission(msg) => Self::Emission(msg),
            etalon_wgsl_forge::ForgeError::WgslParse(msg) => Self::WgslParse(msg),
            etalon_wgsl_forge::ForgeError::WgslValidation(msg) => Self::WgslValidation(msg),
            etalon_wgsl_forge::ForgeError::Validation(msg) => Self::WgslValidation(msg),
            etalon_wgsl_forge::ForgeError::GpuUnavailable(msg) => Self::GpuUnavailable(msg),
            etalon_wgsl_forge::ForgeError::GpuValidation(msg) => Self::GpuValidation(msg),
            etalon_wgsl_forge::ForgeError::DeviceLost(msg) => Self::DeviceLost(msg),
            etalon_wgsl_forge::ForgeError::OracleMismatch(msg) => Self::OracleMismatch(msg),
            etalon_wgsl_forge::ForgeError::Serialization(msg) => Self::Serialization(msg),
            etalon_wgsl_forge::ForgeError::Io(msg) => Self::Io(msg),
        }
    }
}

#[cfg(feature = "gpu-runtime")]
impl From<ForgeError> for etalon_wgsl_forge::ForgeError {
    fn from(value: ForgeError) -> Self {
        match value {
            ForgeError::UnknownKernel(msg) => Self::UnknownKernel(msg),
            ForgeError::InvalidKernel(msg) => Self::InvalidKernel(msg),
            ForgeError::InvalidSchedule(msg) => Self::InvalidSchedule(msg),
            ForgeError::Emission(msg) => Self::Emission(msg),
            ForgeError::WgslParse(msg) => Self::WgslParse(msg),
            ForgeError::WgslValidation(msg) => Self::WgslValidation(msg),
            ForgeError::GpuUnavailable(msg) => Self::GpuUnavailable(msg),
            ForgeError::GpuValidation(msg) => Self::GpuValidation(msg),
            ForgeError::DeviceLost(msg) => Self::DeviceLost(msg),
            ForgeError::OracleMismatch(msg) => Self::OracleMismatch(msg),
            ForgeError::Serialization(msg) => Self::Serialization(msg),
            ForgeError::Io(msg) => Self::Io(msg),
        }
    }
}

pub const FORGE_SCHEMA_VERSION: u32 = 2;
pub const WGPU_API_VERSION: &str = "30.0.0";
pub const NAGA_API_VERSION: &str = "30.0.0";
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

/// Generates a WGSL builtin.
pub fn generate_builtin(
    builtin: BuiltinKernel,
    schedule: Schedule,
    target: TargetBackend,
) -> Result<GeneratedShader, ForgeError> {
    let kernel = builtin.spec();
    schedule.validate(&kernel, &AdapterConstraints::portable())?;
    emit_shader(&kernel, schedule, target)
}

#[cfg(all(test, feature = "gpu-runtime"))]
mod etalon_bridge_tests {
    use super::*;

    #[test]
    fn test_target_backend_conversion() {
        use etalon_bridge::*;
        for target in [
            TargetBackend::Wgsl,
            TargetBackend::Msl,
            TargetBackend::Hlsl,
            TargetBackend::Ptx,
            TargetBackend::CudaC,
            TargetBackend::Spirv,
        ] {
            let etalon_target: EtalonTargetBackend = target.into();
            let roundtrip: TargetBackend = etalon_target.into();
            assert_eq!(target, roundtrip);
            assert_eq!(to_etalon_target(target), etalon_target);
            assert_eq!(from_etalon_target(etalon_target), target);
        }
    }

    #[test]
    fn test_builtin_kernel_conversion() {
        use etalon_bridge::*;
        for kernel in BuiltinKernel::ALL {
            let etalon_kernel: EtalonBuiltinKernel = kernel.into();
            let roundtrip: BuiltinKernel = etalon_kernel.into();
            assert_eq!(kernel, roundtrip);
            assert_eq!(to_etalon_builtin(kernel), etalon_kernel);
            assert_eq!(from_etalon_builtin(etalon_kernel), kernel);
        }
    }

    #[test]
    fn test_schedule_conversion() {
        use etalon_bridge::*;
        let s = Schedule {
            workgroup_size: 128,
            items_per_invocation: 4,
            vector_width: 2,
        };
        let etalon_s: EtalonSchedule = s.into();
        let roundtrip: Schedule = etalon_s.into();
        assert_eq!(s, roundtrip);
        assert_eq!(to_etalon_schedule(s), etalon_s);
        assert_eq!(from_etalon_schedule(etalon_s), s);
    }

    #[test]
    fn test_generate_and_certify_affine_via_etalon() {
        let result = generate_and_certify_via_etalon(
            BuiltinKernel::AffineF32,
            Schedule::default(),
            TargetBackend::Wgsl,
        );
        assert!(result.is_ok(), "generate_and_certify_via_etalon failed: {:?}", result.err());
        let shader = result.unwrap();
        assert!(!shader.source.is_empty());
        assert_eq!(shader.kernel_id, "affine-f32");
    }
}

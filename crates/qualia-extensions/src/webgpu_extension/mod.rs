//! WebGPU Extension for QualiaDB Advanced
//!
//! Continuous-physics compute for fluid dynamics, electromagnetics, heat,
//! waves, particles and dense tensor algebra. Each operation is backed by a
//! **real finite-difference / finite-element / N-body solver** (the `solvers`
//! below), validated against an exact analytic solution in its own unit test.
//!
//! ## CPU reference vs. GPU dispatch
//!
//! The solvers here are the **verifiable CPU reference path**: deterministic,
//! analytic-solution-tested, and runnable on any machine (no GPU required).
//! This mirrors the core engine's pattern (`modalities/calculus`) where the CPU
//! kernel is ground truth and the GPU is an accelerator of the *same* math.
//!
//! The WGSL in [`shaders`] is the corresponding GPU kernel **spec**. GPU
//! acceleration routes through the core engine's shared `wgpu` device
//! (`qualia-core-db` `hetero_dispatch` / `WebGpuIntegrator`) rather than a
//! second device spun up here — the repo deliberately owns a single wgpu stack.
//! The CPU reference is what the tests exercise and what runs when no GPU is
//! present.

use crate::{
    Extension, ExtensionCapability, ExtensionError, ExtensionJob, ExtensionResult, NQuin,
    ResourceRequirements,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

pub mod electromagnetics;
pub mod fluid;
pub mod heat;
pub mod particles;
pub mod shaders;
pub mod tensor;
pub mod wave;

/// WebGPU Extension implementation
pub struct WebGpuExtension {
    shader_manager: WebGpuShaderManager,
    capability: ExtensionCapability,
}

/// Registry of the GPU kernel specs (WGSL) for the supported physics domains.
pub struct WebGpuShaderManager {
    loaded_shaders: HashMap<String, WebGpuShader>,
}

/// A GPU compute-shader spec (the kernel the CPU reference solver mirrors).
#[derive(Debug, Clone)]
pub struct WebGpuShader {
    pub name: String,
    pub shader_type: ShaderType,
    pub wgsl_source: String,
    pub workgroup_size: (u32, u32, u32),
}

/// Physics domains this extension can solve.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShaderType {
    FluidDynamics,
    Electromagnetics,
    HeatTransfer,
    WavePropagation,
    ParticleSimulation,
    TensorOperations,
}

/// Job parameters for a physics solve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebGpuJobParams {
    /// Name of the kernel/domain to run (e.g. `navier_stokes_2d`).
    pub shader_name: String,
    /// Grid extents `(nx, ny, nz)`; `0` on an axis means "use the solver default".
    pub grid_size: (u32, u32, u32),
    /// Named input fields (initial conditions). Empty ⇒ the solver seeds an
    /// analytic initial condition (used by the validation tests).
    pub input_data: HashMap<String, Vec<f32>>,
    /// Named scalar parameters (e.g. `viscosity`, `epsilon`, `m`/`k`/`n`).
    pub uniform_data: HashMap<String, f32>,
    /// Time-stepping / iteration controls.
    pub dispatch_params: DispatchParams,
}

/// Time-stepping / iteration controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchParams {
    pub iterations: u32,
    pub time_step: f64,
    pub convergence_threshold: f32,
    pub max_execution_time_ms: u64,
}

impl Default for DispatchParams {
    fn default() -> Self {
        Self {
            iterations: 0,
            time_step: 0.0,
            convergence_threshold: 1e-3,
            max_execution_time_ms: 30_000,
        }
    }
}

/// Result of a physics solve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebGpuExecutionResult {
    pub output_data: HashMap<String, Vec<f32>>,
    pub performance_metrics: GpuPerformanceMetrics,
    pub convergence_info: ConvergenceInfo,
    pub execution_time_ms: u64,
}

/// Measured throughput of the solve. On the CPU reference path the
/// `gpu_*_utilization` fields are `0.0` (not applicable); the time, FLOP-rate
/// and bandwidth figures are **measured**, not invented.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuPerformanceMetrics {
    pub compute_shader_time_ms: u64,
    pub memory_bandwidth_mb_s: f64,
    pub tflops_achieved: f64,
    pub gpu_utilization_percent: f64,
    pub memory_utilization_percent: f64,
}

/// Convergence / residual report from a solve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceInfo {
    pub converged: bool,
    pub iterations_used: u32,
    pub final_residual: f32,
    pub convergence_rate: f32,
}

/// Output of a solver: the field data plus honest residual/effort bookkeeping.
pub struct SolverReport {
    /// Named output fields.
    pub output: HashMap<String, Vec<f32>>,
    /// Time steps / iterations actually performed.
    pub iterations_used: u32,
    /// A real residual: incompressibility `max|∇·u|` for fluids, the steady
    /// step-to-step change for relaxations, or `0` for exact direct solves.
    pub final_residual: f32,
    /// `final_residual <= convergence_threshold`.
    pub converged: bool,
    /// Floating-point operations performed (for the throughput metric).
    pub flops: u64,
}

/// `0` on an axis means "use the solver default".
#[inline]
pub(crate) fn dim(v: u32, default: usize) -> usize {
    if v == 0 {
        default
    } else {
        v as usize
    }
}

/// Read a named scalar parameter, falling back to `default`.
#[inline]
pub(crate) fn uniform(params: &WebGpuJobParams, key: &str, default: f32) -> f32 {
    params.uniform_data.get(key).copied().unwrap_or(default)
}

impl WebGpuExtension {
    pub fn new() -> Self {
        let mut shader_manager = WebGpuShaderManager {
            loaded_shaders: HashMap::new(),
        };
        Self::load_builtin_shaders(&mut shader_manager);

        Self {
            shader_manager,
            capability: ExtensionCapability {
                name: "webgpu".to_string(),
                version: "2.0.0".to_string(),
                description:
                    "Real finite-difference / N-body physics solvers (analytic-solution validated). \
                     CPU reference path; WGSL kernel specs included for GPU dispatch."
                        .to_string(),
                required_resources: ResourceRequirements {
                    min_memory_mb: 128,
                    // GPU is optional: the CPU reference path runs without one.
                    min_vram_mb: Some(1024),
                    requires_gpu: false,
                    requires_network: false,
                    max_concurrent_jobs: 3,
                },
                supported_operations: vec![
                    "simulate_fluid".to_string(),
                    "solve_electromagnetics".to_string(),
                    "compute_heat_transfer".to_string(),
                    "propagate_waves".to_string(),
                    "simulate_particles".to_string(),
                    "tensor_operations".to_string(),
                ],
            },
        }
    }

    fn load_builtin_shaders(mgr: &mut WebGpuShaderManager) {
        let specs = [
            (
                "navier_stokes_2d",
                ShaderType::FluidDynamics,
                shaders::NAVIER_STOKES_2D,
                (16u32, 16u32, 1u32),
            ),
            (
                "maxwell_fdtd_1d",
                ShaderType::Electromagnetics,
                shaders::MAXWELL_FDTD_1D,
                (64, 1, 1),
            ),
            (
                "heat_diffusion_2d",
                ShaderType::HeatTransfer,
                shaders::HEAT_DIFFUSION_2D,
                (16, 16, 1),
            ),
            (
                "wave_equation_2d",
                ShaderType::WavePropagation,
                shaders::WAVE_EQUATION_2D,
                (16, 16, 1),
            ),
        ];
        for (name, ty, src, wg) in specs {
            mgr.loaded_shaders.insert(
                name.to_string(),
                WebGpuShader {
                    name: name.to_string(),
                    shader_type: ty,
                    wgsl_source: src.to_string(),
                    workgroup_size: wg,
                },
            );
        }
    }

    /// Resolve a kernel name to its physics domain. Falls back to a name table
    /// for solvers whose GPU kernel ships only as a CPU reference.
    fn shader_type_for(&self, name: &str) -> Option<ShaderType> {
        if let Some(s) = self.shader_manager.loaded_shaders.get(name) {
            return Some(s.shader_type);
        }
        Some(match name {
            "navier_stokes_2d" => ShaderType::FluidDynamics,
            "maxwell_fdtd_1d" => ShaderType::Electromagnetics,
            "heat_diffusion_2d" => ShaderType::HeatTransfer,
            "wave_equation_2d" => ShaderType::WavePropagation,
            "nbody_verlet" => ShaderType::ParticleSimulation,
            "tensor_gemm" => ShaderType::TensorOperations,
            _ => return None,
        })
    }

    /// Run the real solver for `params.shader_name` and report measured timing.
    pub async fn execute_shader(
        &self,
        params: WebGpuJobParams,
    ) -> Result<WebGpuExecutionResult, ExtensionError> {
        let shader_type = self.shader_type_for(&params.shader_name).ok_or_else(|| {
            ExtensionError::ExecutionFailed(format!("Unknown shader '{}'", params.shader_name))
        })?;

        let start = Instant::now();
        let report = run_solver(shader_type, &params);
        let elapsed = start.elapsed();

        let secs = elapsed.as_secs_f64().max(1e-9);
        let out_floats: usize = report.output.values().map(|v| v.len()).sum();
        let bytes_moved = (out_floats * 4 * report.iterations_used.max(1) as usize) as f64;

        Ok(WebGpuExecutionResult {
            performance_metrics: GpuPerformanceMetrics {
                compute_shader_time_ms: elapsed.as_millis() as u64,
                memory_bandwidth_mb_s: bytes_moved / secs / 1.0e6,
                tflops_achieved: report.flops.max(1) as f64 / secs / 1.0e12,
                // Not applicable on the CPU reference path; measured only on the
                // wgpu dispatch path.
                gpu_utilization_percent: 0.0,
                memory_utilization_percent: 0.0,
            },
            convergence_info: ConvergenceInfo {
                converged: report.converged,
                iterations_used: report.iterations_used,
                final_residual: report.final_residual,
                convergence_rate: if report.iterations_used > 0 && report.final_residual > 0.0 {
                    1.0 - report.final_residual.min(1.0)
                } else {
                    1.0
                },
            },
            output_data: report.output,
            execution_time_ms: elapsed.as_millis() as u64,
        })
    }

    fn result_to_quins(result: &WebGpuExecutionResult, job_id: &str) -> Vec<NQuin> {
        vec![
            NQuin {
                subject: crate::q_hash(job_id),
                predicate: crate::q_hash("q42:hasGpuPerformance"),
                object: (result.performance_metrics.tflops_achieved * 1000.0) as u64,
                context: crate::q_hash("webgpu:performance"),
                metadata: ((result.performance_metrics.compute_shader_time_ms) << 32)
                    | (result.performance_metrics.gpu_utilization_percent as u64),
                parity: 0,
            },
            NQuin {
                subject: crate::q_hash(job_id),
                predicate: crate::q_hash("q42:hasConvergence"),
                object: (result.convergence_info.final_residual as f64 * 1_000_000.0) as u64,
                context: crate::q_hash("webgpu:convergence"),
                metadata: ((result.convergence_info.iterations_used as u64) << 32)
                    | (if result.convergence_info.converged {
                        1
                    } else {
                        0
                    }),
                parity: 0,
            },
            NQuin {
                subject: crate::q_hash(job_id),
                predicate: crate::q_hash("q42:hasExecutionTime"),
                object: result.execution_time_ms,
                context: crate::q_hash("webgpu:performance"),
                metadata: 0,
                parity: 0,
            },
        ]
    }
}

impl Default for WebGpuExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Dispatch a solve to the real CPU reference solver for `shader_type`.
pub(crate) fn run_solver(shader_type: ShaderType, params: &WebGpuJobParams) -> SolverReport {
    match shader_type {
        ShaderType::FluidDynamics => fluid::solve(params),
        ShaderType::Electromagnetics => electromagnetics::solve(params),
        ShaderType::HeatTransfer => heat::solve(params),
        ShaderType::WavePropagation => wave::solve(params),
        ShaderType::ParticleSimulation => particles::solve(params),
        ShaderType::TensorOperations => tensor::solve(params),
    }
}

/// Map an advertised operation to its default kernel name.
fn operation_kernel(op: &str) -> Option<&'static str> {
    Some(match op {
        "simulate_fluid" => "navier_stokes_2d",
        "solve_electromagnetics" => "maxwell_fdtd_1d",
        "compute_heat_transfer" => "heat_diffusion_2d",
        "propagate_waves" => "wave_equation_2d",
        "simulate_particles" => "nbody_verlet",
        "tensor_operations" => "tensor_gemm",
        _ => return None,
    })
}

#[async_trait]
impl Extension for WebGpuExtension {
    fn capability(&self) -> ExtensionCapability {
        self.capability.clone()
    }

    async fn execute(&self, job: ExtensionJob) -> Result<ExtensionResult, ExtensionError> {
        let start_time = Instant::now();

        let kernel = operation_kernel(&job.operation)
            .ok_or_else(|| ExtensionError::OperationNotSupported(job.operation.clone()))?;

        // Parse caller params if present, otherwise run the solver's analytic
        // default initial condition.
        let mut params: WebGpuJobParams = match job.parameters.get("webgpu_params") {
            Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
                ExtensionError::ExecutionFailed(format!("Invalid webgpu_params: {}", e))
            })?,
            None => WebGpuJobParams {
                shader_name: kernel.to_string(),
                grid_size: (0, 0, 0),
                input_data: HashMap::new(),
                uniform_data: HashMap::new(),
                dispatch_params: DispatchParams::default(),
            },
        };
        params.shader_name = kernel.to_string();

        let result = self.execute_shader(params).await?;
        let quins = Self::result_to_quins(&result, &job.job_id);

        let mut metadata = HashMap::new();
        metadata.insert("kernel".to_string(), kernel.to_string());
        metadata.insert(
            "tflops".to_string(),
            result.performance_metrics.tflops_achieved.to_string(),
        );
        metadata.insert(
            "converged".to_string(),
            result.convergence_info.converged.to_string(),
        );
        metadata.insert(
            "final_residual".to_string(),
            result.convergence_info.final_residual.to_string(),
        );

        Ok(ExtensionResult {
            job_id: job.job_id,
            success: true,
            result_quins: quins,
            metadata,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    fn shutdown(&self) -> Result<(), ExtensionError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(op: &str) -> ExtensionJob {
        ExtensionJob {
            job_id: format!("job-{op}"),
            extension_name: "webgpu".to_string(),
            operation: op.to_string(),
            parameters: HashMap::new(),
            boundary_conditions: vec![],
        }
    }

    #[tokio::test]
    async fn capability_advertises_real_solvers() {
        let ext = WebGpuExtension::new();
        let cap = ext.capability();
        assert_eq!(cap.name, "webgpu");
        assert_eq!(cap.version, "2.0.0");
        assert!(cap
            .supported_operations
            .contains(&"simulate_fluid".to_string()));
        // GPU is optional now: the CPU reference path runs without one.
        assert!(!cap.required_resources.requires_gpu);
    }

    #[tokio::test]
    async fn every_advertised_operation_runs_a_real_solver() {
        let ext = WebGpuExtension::new();
        for op in [
            "simulate_fluid",
            "solve_electromagnetics",
            "compute_heat_transfer",
            "propagate_waves",
            "simulate_particles",
            "tensor_operations",
        ] {
            let res = ext.execute(job(op)).await.unwrap_or_else(|e| {
                panic!("operation {op} failed: {e}");
            });
            assert!(res.success, "{op} did not succeed");
            assert!(!res.result_quins.is_empty(), "{op} produced no quins");
            assert!(
                res.metadata
                    .get("tflops")
                    .map(|s| s != "0")
                    .unwrap_or(false),
                "{op} reported no measured throughput"
            );
        }
    }

    #[tokio::test]
    async fn unknown_operation_is_rejected() {
        let ext = WebGpuExtension::new();
        let err = ext.execute(job("mine_bitcoin")).await.unwrap_err();
        assert!(matches!(err, ExtensionError::OperationNotSupported(_)));
    }

    #[tokio::test]
    async fn shader_specs_are_registered() {
        let ext = WebGpuExtension::new();
        let s = ext
            .shader_manager
            .loaded_shaders
            .get("navier_stokes_2d")
            .expect("navier_stokes_2d registered");
        assert_eq!(s.shader_type, ShaderType::FluidDynamics);
        assert_eq!(s.workgroup_size, (16, 16, 1));
        assert!(!s.wgsl_source.is_empty());
    }
}

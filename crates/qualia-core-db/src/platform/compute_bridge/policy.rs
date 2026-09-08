//! `ComputePolicy` — the one shared dispatch surface the whole engine calls
//! (HARDWARE_BACKEND_AUTOSELECT_PLAN.md §4).
//!
//! A STEM call site asks `policy.select(class, problem_bytes)` and runs the returned
//! [`Plan`]. No module embeds a device decision; they all defer to the measured
//! per-class matrix. `select` is **O(1) and zero-heap** (it reads an already-built
//! matrix and returns a `Copy` plan) — the heavy probe ran once at boot. CPU is
//! always a valid plan and never hard-fails (plan §7).
//!
//! With `gpu-runtime` this *wraps* the existing vendor-neutral `hetero_dispatch`
//! helpers (precision, tiling, zero-copy) rather than duplicating them, and adds
//! the part they cannot do alone: the measured per-class backend/circuit choice
//! from the matrix. Without that feature there is no GPU probe and no matrix —
//! `select` always returns a CPU plan.

#[cfg(feature = "gpu-runtime")]
use crate::device_benchmark::CircuitKind;
#[cfg(feature = "gpu-runtime")]
use crate::platform::hetero_dispatch::{
    select_precision, HeterogeneousDispatcher, HostCapabilities, PowerThermalBudget, Precision,
    ZeroCopyStrategy,
};

use super::backend::BackendId;
#[cfg(feature = "gpu-runtime")]
use super::backend::KernelPanel;
use super::kernel_class::KernelClass;
#[cfg(feature = "gpu-runtime")]
use super::matrix::{probe_class_matrix, ClassMatrix};

/// The resolved execution plan for one kernel dispatch. `Copy`, zero-heap — it
/// names the backend/circuit and the precision/tiling/transfer policy, all of which
/// are small scalars. The human-readable circuit label stays in the matrix.
///
/// GPU-only fields (`circuit_kind`, `precision`, `zero_copy`) exist only with
/// `gpu-runtime`. Without it the plan is a CPU backend with a single in-pool tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Which acceleration method to run on (`"cpu"`, `"wgpu"`, …).
    pub backend: BackendId,
    /// The circuit class chosen (discrete GPU / iGPU / CPU / NPU).
    #[cfg(feature = "gpu-runtime")]
    pub circuit_kind: CircuitKind,
    /// Numeric precision to use (from the host's VRAM/power/thermal envelope).
    #[cfg(feature = "gpu-runtime")]
    pub precision: Precision,
    /// Number of sequential tiles a GPU job is split into so each fits in VRAM
    /// (graceful degradation, never an OOM hard-fail). 1 on CPU or when it fits.
    pub tiles: u32,
    /// How data reaches the device (mmap-direct on unified memory, else staging).
    #[cfg(feature = "gpu-runtime")]
    pub zero_copy: ZeroCopyStrategy,
}

impl Plan {
    /// Is this plan running on the CPU fallback?
    pub fn is_cpu(self) -> bool {
        #[cfg(feature = "gpu-runtime")]
        {
            self.backend == BackendId::CPU || self.circuit_kind == CircuitKind::Cpu
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            self.backend == BackendId::CPU
        }
    }

    fn cpu() -> Self {
        Self {
            backend: BackendId::CPU,
            #[cfg(feature = "gpu-runtime")]
            circuit_kind: CircuitKind::Cpu,
            #[cfg(feature = "gpu-runtime")]
            precision: Precision::F32,
            tiles: 1,
            #[cfg(feature = "gpu-runtime")]
            zero_copy: ZeroCopyStrategy::MmapDirect,
        }
    }
}

/// The shared compute policy: a measured per-class matrix plus the host's
/// precision/VRAM envelope. Built once at startup; `select` is the hot, O(1) call.
/// Without `gpu-runtime` there is no matrix — every select is the CPU plan.
pub struct ComputePolicy {
    #[cfg(feature = "gpu-runtime")]
    matrix: ClassMatrix,
    #[cfg(feature = "gpu-runtime")]
    budget: PowerThermalBudget,
    #[cfg(feature = "gpu-runtime")]
    host: HostCapabilities,
}

impl ComputePolicy {
    /// CPU-only policy used when `gpu-runtime` is off (no probe, no matrix).
    #[cfg(not(feature = "gpu-runtime"))]
    pub fn cpu_only() -> Self {
        Self {}
    }

    /// Build from an already-measured per-class matrix (e.g. one loaded from the
    /// passport, or a synthetic one in tests — no GPU required).
    #[cfg(feature = "gpu-runtime")]
    pub fn from_class_matrix(
        matrix: ClassMatrix,
        budget: PowerThermalBudget,
        host: HostCapabilities,
    ) -> Self {
        Self {
            matrix,
            budget,
            host,
        }
    }

    /// Probe the registry once (heavy) and build the policy. Call at startup; cache
    /// the matrix in the passport to avoid re-probing every boot.
    #[cfg(feature = "gpu-runtime")]
    pub fn probe(
        registry: &super::backend::BackendRegistry,
        panel: &KernelPanel,
        budget: PowerThermalBudget,
        host: HostCapabilities,
    ) -> Self {
        Self::from_class_matrix(probe_class_matrix(registry, panel), budget, host)
    }

    /// The measured per-class matrix (for inspection / passport caching).
    #[cfg(feature = "gpu-runtime")]
    pub fn matrix(&self) -> &ClassMatrix {
        &self.matrix
    }

    /// Select the execution plan for a `class` kernel touching `problem_bytes` of
    /// data. O(1), zero-heap, never fails: returns a CPU plan when no accelerator
    /// wins, when no GPU was probed, when `gpu-runtime` is off, or when the measured
    /// GPU win is within noise of CPU for a class that is not typically GPU-amenable
    /// (the §13 tie-break — a measured GPU win on an amenable class is always
    /// honoured).
    pub fn select(&self, class: KernelClass, problem_bytes: u64) -> Plan {
        #[cfg(not(feature = "gpu-runtime"))]
        {
            let _ = (class, problem_bytes);
            return Plan::cpu();
        }
        #[cfg(feature = "gpu-runtime")]
        {
            // Precision from the host's VRAM/power/thermal envelope (reuse hetero_dispatch).
            // Treat the problem as f32 elements for the param-count proxy.
            let param_count = (problem_bytes / 4).max(1);
            let precision = select_precision(param_count, &self.budget);

            let best = self.matrix.best_for(class);
            let cpu_ms = self
                .matrix
                .rows(class)
                .iter()
                .find(|r| r.kind == CircuitKind::Cpu)
                .map(|r| r.ms_per_gemv);

            let choose_gpu = match best {
                None => false,                                  // class not probed at all → CPU
                Some(b) if b.kind == CircuitKind::Cpu => false, // CPU already won
                Some(b) => {
                    // A GPU/other circuit measured fastest. Honour it unless the class is
                    // not GPU-amenable AND the win over CPU is within ~5% (measurement
                    // noise) — then prefer the simpler CPU path.
                    if class.is_typically_gpu_amenable() {
                        true
                    } else if let Some(cms) = cpu_ms {
                        b.ms_per_gemv < cms * 0.95
                    } else {
                        true
                    }
                }
            };

            if let (true, Some(b)) = (choose_gpu, best) {
                let dispatcher = HeterogeneousDispatcher::new(self.host);
                let tiles = dispatcher.gpu_tiles(problem_bytes);
                let zero_copy = match b.kind {
                    CircuitKind::DiscreteGpu => ZeroCopyStrategy::StagingUpload,
                    _ => ZeroCopyStrategy::MmapDirect, // integrated/unified or NPU
                };
                Plan {
                    backend: BackendId(backend_id_for(&b.backend)),
                    circuit_kind: b.kind,
                    precision,
                    tiles,
                    zero_copy,
                }
            } else {
                Plan::cpu()
            }
        }
    }
}

/// Map a `CircuitBench.backend` string (`"Vulkan"`, `"Dx12"`, `"native"`, …) to a
/// stable `BackendId`. wgpu circuits all carry the `"wgpu"` id (the adapter API is
/// in `circuit_kind`/the matrix label); native is CPU.
#[cfg(feature = "gpu-runtime")]
fn backend_id_for(backend: &str) -> &'static str {
    match backend {
        "native" => "cpu",
        _ => "wgpu",
    }
}

#[cfg(all(test, feature = "gpu-runtime"))]
mod tests {
    use super::*;
    use crate::device_benchmark::CircuitBench;

    const GIB: u64 = 1 << 30;

    fn roomy_host() -> (PowerThermalBudget, HostCapabilities) {
        (
            PowerThermalBudget {
                vram_budget_bytes: 16 * GIB,
                power_budget_mw: 60_000,
                thermal_headroom_c: 30.0,
            },
            HostCapabilities {
                gpu_available: true,
                vram_available: 8 * GIB,
                npu_available: false,
                cpu_threads: 16,
            },
        )
    }

    fn row(kind: CircuitKind, backend: &str, ms: f64) -> CircuitBench {
        CircuitBench {
            label: format!("{backend} circuit"),
            kind,
            backend: backend.to_string(),
            ms_per_gemv: ms,
            gflops: 0.0,
            upload_gbps: if kind == CircuitKind::Cpu {
                f64::INFINITY
            } else {
                5.0
            },
            rel_score: 1.0,
            decode_proxy_tok_s: None,
        }
    }

    /// A synthetic matrix: GPU clearly wins DenseLinear; CPU only for Divergent.
    fn synth_matrix() -> ClassMatrix {
        let mut per_class = Vec::new();
        for class in KernelClass::ALL {
            let rows = match class {
                KernelClass::DenseLinear => vec![
                    row(CircuitKind::DiscreteGpu, "Vulkan", 0.4),
                    row(CircuitKind::Cpu, "native", 20.0),
                ],
                KernelClass::Divergent => vec![row(CircuitKind::Cpu, "native", 3.0)],
                _ => vec![row(CircuitKind::Cpu, "native", 5.0)],
            };
            per_class.push((class, rows));
        }
        ClassMatrix::from_per_class(per_class)
    }

    #[test]
    fn selects_measured_gpu_winner_for_dense_linear() {
        let (budget, host) = roomy_host();
        let policy = ComputePolicy::from_class_matrix(synth_matrix(), budget, host);
        let plan = policy.select(KernelClass::DenseLinear, 64 * 1024 * 1024);
        assert_eq!(plan.backend, BackendId::WGPU);
        assert_eq!(plan.circuit_kind, CircuitKind::DiscreteGpu);
        assert_eq!(plan.zero_copy, ZeroCopyStrategy::StagingUpload); // discrete → staging
        assert!(!plan.is_cpu());
    }

    #[test]
    fn falls_back_to_cpu_when_only_cpu_probed() {
        let (budget, host) = roomy_host();
        let policy = ComputePolicy::from_class_matrix(synth_matrix(), budget, host);
        let plan = policy.select(KernelClass::Divergent, 1 << 20);
        assert!(plan.is_cpu());
        assert_eq!(plan.backend, BackendId::CPU);
        assert_eq!(plan.tiles, 1);
    }

    #[test]
    fn unprobed_class_is_cpu_not_a_panic() {
        // A matrix with no rows for a class → CPU plan, never a panic (plan §7).
        let (budget, host) = roomy_host();
        let empty = ClassMatrix::from_per_class(vec![(KernelClass::Fft, Vec::new())]);
        let policy = ComputePolicy::from_class_matrix(empty, budget, host);
        assert!(policy.select(KernelClass::Fft, 1 << 20).is_cpu());
        assert!(policy.select(KernelClass::AllPairs, 1 << 20).is_cpu()); // class absent entirely
    }

    #[test]
    fn vram_pressure_tiles_instead_of_failing() {
        let budget = PowerThermalBudget {
            vram_budget_bytes: 16 * GIB,
            power_budget_mw: 60_000,
            thermal_headroom_c: 30.0,
        };
        let host = HostCapabilities {
            gpu_available: true,
            vram_available: 2 * GIB,
            npu_available: false,
            cpu_threads: 8,
        };
        let policy = ComputePolicy::from_class_matrix(synth_matrix(), budget, host);
        // A 5 GiB DenseLinear job on a 2 GiB GPU → 3 tiles, not an OOM.
        let plan = policy.select(KernelClass::DenseLinear, 5 * GIB);
        assert!(!plan.is_cpu());
        assert_eq!(plan.tiles, 3);
    }
}

#[cfg(all(test, not(feature = "gpu-runtime")))]
mod tests {
    use super::*;

    #[test]
    fn cpu_only_policy_always_selects_cpu() {
        let policy = ComputePolicy::cpu_only();
        for class in KernelClass::ALL {
            let plan = policy.select(class, 1 << 30);
            assert!(plan.is_cpu());
            assert_eq!(plan.backend, BackendId::CPU);
            assert_eq!(plan.tiles, 1);
        }
    }
}

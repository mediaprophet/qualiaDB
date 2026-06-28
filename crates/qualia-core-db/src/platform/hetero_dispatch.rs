//! Vendor-neutral heterogeneous compute dispatch + storage / precision policy.
//!
//! This module **replaces the former CUDA/cuFile GPUDirect-Storage bridge**
//! (`cuda_bridge.rs`, removed). That bridge was the engine's one vendor-locked
//! appendage — NVIDIA-only, Linux-only, and unverifiable without specific hardware.
//! Removing it dissolves the hardware boundary entirely. The four capabilities it
//! was meant to provide are delivered here on the engine's GENERAL stack — portable
//! `wgpu` ([`super::gpu`]) + memory-mapped I/O ([`super::host`]) — so they build,
//! run, and verify on any backend (Vulkan / DX12 / Metal / WebGPU), no vendor SDK:
//!
//! 1. **Unified-memory zero-copy** ([`ZeroCopyStrategy`]) — choose mmap-direct on
//!    integrated / unified-memory GPUs (Apple Silicon via Metal, which `wgpu` maps
//!    transparently — CPU and GPU share physical RAM, so the mmap'd region is GPU
//!    visible with no copy) vs a one-time staging upload on discrete GPUs.
//! 2. **Hardware-agnostic fallback dispatcher** ([`HeterogeneousDispatcher`]) —
//!    route a job to the GPU (`wgpu`) when it fits, else NPU, else CPU; and tile a
//!    matmul across passes when VRAM is exhausted instead of hard-failing.
//! 3. **Kernel stream fusion** ([`plan_fusion`]) — group consecutive same-shape
//!    element-wise tensor ops into a single `wgpu` compute pass (the portable
//!    analogue of CUDA stream fusion; the engine's fused shaders already do this at
//!    the shader level). Fewer passes ⇒ fewer dispatch / PCIe round-trips.
//! 4. **Mixed-precision policy** ([`select_precision`]) — pick f32/f16/q8/q4 from
//!    the host's VRAM / power / thermal budget.
//!
//! ## The one capability deliberately NOT ported
//! NVIDIA GPUDirect-Storage's *true* NVMe→VRAM DMA (bypassing system RAM) has **no
//! portable `wgpu` equivalent**. The vendor-neutral substitute (and the engine's
//! actual path) is `mmap` + OS page cache + a one-time staging upload — zero-heap,
//! standard OS mechanics, identical across an A2000 / Apple M-series / generic Linux
//! box. GDS-class throughput only matters when streaming a 70B model off an NVMe
//! array into an 80 GB datacenter GPU — the deployment the affordability rail
//! explicitly does not target, so nothing on the critical path is lost.
//!
//! ## Future, optional Vulkan zero-copy fast-path (documented, NOT built)
//! If a specific deployment ever justifies skipping the staging copy, the
//! vendor-neutral way is a `wgpu-hal` Vulkan fast-path that imports the mmap'd
//! weights as device memory via `VK_EXT_external_memory_host` (broadly supported,
//! cross-vendor) + Resizable-BAR — lit up ONLY on the Vulkan backend, behind a
//! `vulkan_zero_copy_import` feature, additive over the portable path. It is
//! deliberately unbuilt: it needs `unsafe` wgpu-hal and is backend-specific
//! (DX12/Metal have their own external-memory mechanisms), which would re-introduce
//! exactly the per-backend coupling that removing CUDA just eliminated. Build it
//! only when a real deployment needs it; never as the core storage dependency.
//!
//! All routines here are pure-scalar policy / planning logic — **zero heap**, no
//! recursion, run anywhere.

// ── 1. Unified-memory zero-copy strategy ───────────────────────────────────────

/// How weight data reaches the GPU for a given device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopyStrategy {
    /// Integrated / unified-memory GPU (Apple Silicon via Metal; integrated
    /// Intel/AMD): the `mmap`'d region is directly GPU-visible — no host→device copy.
    MmapDirect,
    /// Discrete GPU: upload the `mmap`'d region once via a staging buffer.
    StagingUpload,
}

impl ZeroCopyStrategy {
    /// Pick the strategy from whether the adapter has unified memory.
    pub fn for_device(is_unified_memory: bool) -> Self {
        if is_unified_memory {
            Self::MmapDirect
        } else {
            Self::StagingUpload
        }
    }

    /// Map a `wgpu` device type to the strategy: integrated GPUs share memory with
    /// the host (unified) ⇒ mmap-direct; discrete GPUs ⇒ staging upload. CPU/other
    /// adapters are treated as unified (the "device" buffer is host memory).
    pub fn for_wgpu_device_type(device_type: wgpu::DeviceType) -> Self {
        match device_type {
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::VirtualGpu => Self::StagingUpload,
            _ => Self::MmapDirect,
        }
    }

    /// Whether a host→device copy is required (false on unified memory).
    pub fn requires_host_copy(self) -> bool {
        matches!(self, Self::StagingUpload)
    }
}

// ── 2. Hardware-agnostic fallback dispatcher ────────────────────────────────────

/// The compute backend a job is routed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeBackend {
    /// Portable `wgpu` GPU path ([`super::gpu::WebGpuIntegrator`]).
    Gpu,
    /// A neural-processing unit, when present.
    Npu,
    /// CPU fallback — always available, never hard-fails.
    Cpu,
}

/// What the host can offer the dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostCapabilities {
    pub gpu_available: bool,
    /// Usable VRAM in bytes (e.g. `wgpu` `limits().max_buffer_size`).
    pub vram_available: u64,
    pub npu_available: bool,
    pub cpu_threads: u32,
}

/// Routes compute jobs across GPU / NPU / CPU and degrades gracefully under VRAM
/// pressure instead of hard-failing (the behaviour the CUDA bridge lacked).
#[derive(Debug, Clone, Copy)]
pub struct HeterogeneousDispatcher {
    caps: HostCapabilities,
}

impl HeterogeneousDispatcher {
    pub fn new(caps: HostCapabilities) -> Self {
        Self { caps }
    }

    /// Choose a backend for a job needing `vram_required` bytes: GPU if present and
    /// it fits → else NPU if present → else CPU (always works). When the GPU is
    /// present but the job is larger than VRAM, the caller should GPU-tile (see
    /// [`Self::gpu_tiles`]) rather than fall straight to CPU.
    pub fn select_backend(&self, vram_required: u64) -> ComputeBackend {
        if self.caps.gpu_available {
            if vram_required <= self.caps.vram_available || self.caps.vram_available > 0 {
                // GPU present: it fits, or it can be tiled to fit (see gpu_tiles).
                return ComputeBackend::Gpu;
            }
        }
        if self.caps.npu_available {
            ComputeBackend::Npu
        } else {
            ComputeBackend::Cpu
        }
    }

    /// How many sequential tiles a `total_bytes` GPU job must be split into so each
    /// tile fits in available VRAM — graceful degradation instead of an OOM hard
    /// fail. Returns 1 when it already fits (or when there's no GPU/VRAM to tile
    /// into, in which case the job runs on the CPU as a single pass).
    pub fn gpu_tiles(&self, total_bytes: u64) -> u32 {
        if !self.caps.gpu_available || self.caps.vram_available == 0 {
            return 1;
        }
        // ceil(total / vram), clamped to ≥ 1.
        let tiles = total_bytes.div_ceil(self.caps.vram_available);
        tiles.max(1).min(u32::MAX as u64) as u32
    }

    pub fn capabilities(&self) -> HostCapabilities {
        self.caps
    }
}

// ── 3. Kernel / stream fusion planning ──────────────────────────────────────────

/// The fusability class of a tensor op in a dispatch sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorOpKind {
    /// Element-wise (add, mul, activation…): fusable with adjacent same-shape
    /// element-wise ops into one compute pass.
    Elementwise,
    /// A fusion barrier (reduction, matmul, reshape): forces a new pass.
    Barrier,
}

/// One op in a planned dispatch sequence. `shape` is a shape token; element-wise ops
/// only fuse with neighbours of the same shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorOp {
    pub kind: TensorOpKind,
    pub shape: u64,
}

/// Plan stream fusion: the number of `wgpu` compute passes a sequence needs after
/// fusing each maximal run of same-shape element-wise ops into a single pass.
/// Barriers each take their own pass. Result is in `1..=ops.len()`; fewer passes ⇒
/// fewer dispatch / PCIe round-trips. Zero-heap (single linear scan).
pub fn plan_fusion(ops: &[TensorOp]) -> u32 {
    let mut passes = 0u32;
    let mut i = 0usize;
    while i < ops.len() {
        passes += 1;
        if ops[i].kind == TensorOpKind::Barrier {
            i += 1;
            continue;
        }
        // Absorb the maximal run of same-shape element-wise ops into this pass.
        let shape = ops[i].shape;
        i += 1;
        while i < ops.len() && ops[i].kind == TensorOpKind::Elementwise && ops[i].shape == shape {
            i += 1;
        }
    }
    passes
}

// ── 4. Mixed-precision policy ────────────────────────────────────────────────────

/// Numeric precision for weights / activations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    F32,
    F16,
    Q8,
    Q4,
}

impl Precision {
    /// Bytes per weight element.
    pub fn bytes_per_weight(self) -> f64 {
        match self {
            Self::F32 => 4.0,
            Self::F16 => 2.0,
            Self::Q8 => 1.0,
            Self::Q4 => 0.5,
        }
    }
}

/// The host's power / thermal / memory envelope for the precision decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerThermalBudget {
    pub vram_budget_bytes: u64,
    /// Available power headroom, milliwatts.
    pub power_budget_mw: u32,
    /// Degrees C below the thermal-throttle point.
    pub thermal_headroom_c: f32,
}

/// Pick the precision tuned to the host's thermodynamic / power / memory constraints:
/// choose the coarsest precision that still fits `param_count` weights in the VRAM
/// budget, and — when power or thermal headroom is tight — never run the heavy
/// precisions (f32/f16 draw more power and generate more heat). Returns `Q4` if even
/// q4 overflows VRAM (the caller must then tile or offload).
pub fn select_precision(param_count: u64, budget: &PowerThermalBudget) -> Precision {
    let fits = |p: Precision| {
        (param_count as f64 * p.bytes_per_weight()) as u64 <= budget.vram_budget_bytes
    };
    // Tight envelope ⇒ cap the *maximum* precision we'll consider (lower precision =
    // fewer FLOPs = less heat/draw). Thresholds: < 5°C headroom or < 15 W budget.
    let throttling = budget.thermal_headroom_c < 5.0 || budget.power_budget_mw < 15_000;
    let candidates: &[Precision] = if throttling {
        &[Precision::Q8, Precision::Q4]
    } else {
        &[Precision::F32, Precision::F16, Precision::Q8, Precision::Q4]
    };
    for &p in candidates {
        if fits(p) {
            return p;
        }
    }
    Precision::Q4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_copy_strategy_follows_memory_architecture() {
        // Unified memory (Apple Silicon / integrated) → no host copy.
        assert_eq!(
            ZeroCopyStrategy::for_device(true),
            ZeroCopyStrategy::MmapDirect
        );
        assert!(!ZeroCopyStrategy::for_device(true).requires_host_copy());
        // Discrete GPU → staging upload.
        assert_eq!(
            ZeroCopyStrategy::for_device(false),
            ZeroCopyStrategy::StagingUpload
        );
        assert!(ZeroCopyStrategy::for_device(false).requires_host_copy());
        // wgpu device-type mapping.
        assert_eq!(
            ZeroCopyStrategy::for_wgpu_device_type(wgpu::DeviceType::IntegratedGpu),
            ZeroCopyStrategy::MmapDirect
        );
        assert_eq!(
            ZeroCopyStrategy::for_wgpu_device_type(wgpu::DeviceType::DiscreteGpu),
            ZeroCopyStrategy::StagingUpload
        );
    }

    #[test]
    fn dispatcher_routes_and_falls_back() {
        // GPU present and job fits → GPU.
        let d = HeterogeneousDispatcher::new(HostCapabilities {
            gpu_available: true,
            vram_available: 8 << 30,
            npu_available: true,
            cpu_threads: 16,
        });
        assert_eq!(d.select_backend(1 << 30), ComputeBackend::Gpu);

        // No GPU, NPU present → NPU.
        let d2 = HeterogeneousDispatcher::new(HostCapabilities {
            gpu_available: false,
            vram_available: 0,
            npu_available: true,
            cpu_threads: 8,
        });
        assert_eq!(d2.select_backend(1 << 30), ComputeBackend::Npu);

        // No GPU, no NPU → CPU (always works, never hard-fails).
        let d3 = HeterogeneousDispatcher::new(HostCapabilities {
            gpu_available: false,
            vram_available: 0,
            npu_available: false,
            cpu_threads: 4,
        });
        assert_eq!(d3.select_backend(1 << 30), ComputeBackend::Cpu);
    }

    #[test]
    fn vram_exhaustion_tiles_instead_of_failing() {
        let d = HeterogeneousDispatcher::new(HostCapabilities {
            gpu_available: true,
            vram_available: 2 << 30, // 2 GiB
            npu_available: false,
            cpu_threads: 8,
        });
        assert_eq!(d.gpu_tiles(1 << 30), 1, "fits in one tile");
        assert_eq!(d.gpu_tiles(2 << 30), 1, "exactly fits");
        assert_eq!(
            d.gpu_tiles(5 << 30),
            3,
            "5 GiB / 2 GiB → 3 tiles, no OOM hard-fail"
        );
        // No GPU → single CPU pass.
        let cpu = HeterogeneousDispatcher::new(HostCapabilities {
            gpu_available: false,
            vram_available: 0,
            npu_available: false,
            cpu_threads: 4,
        });
        assert_eq!(cpu.gpu_tiles(99 << 30), 1);
    }

    #[test]
    fn fusion_collapses_elementwise_runs() {
        let ew = |s| TensorOp {
            kind: TensorOpKind::Elementwise,
            shape: s,
        };
        let barrier = |s| TensorOp {
            kind: TensorOpKind::Barrier,
            shape: s,
        };

        // Three same-shape element-wise ops fuse into one pass.
        assert_eq!(plan_fusion(&[ew(1), ew(1), ew(1)]), 1);
        // A barrier (matmul/reduction) splits the run: EW | barrier | EW = 3 passes.
        assert_eq!(plan_fusion(&[ew(1), barrier(1), ew(1)]), 3);
        // Different shapes don't fuse.
        assert_eq!(plan_fusion(&[ew(1), ew(2)]), 2);
        // Mixed run: (EW EW) | barrier | (EW EW EW) = 3 passes.
        assert_eq!(
            plan_fusion(&[ew(1), ew(1), barrier(9), ew(2), ew(2), ew(2)]),
            3
        );
        assert_eq!(plan_fusion(&[]), 0);
    }

    #[test]
    fn precision_fits_budget_and_respects_thermals() {
        let gib = 1u64 << 30;
        // 1B params, roomy VRAM + headroom → f32.
        let roomy = PowerThermalBudget {
            vram_budget_bytes: 8 * gib,
            power_budget_mw: 60_000,
            thermal_headroom_c: 30.0,
        };
        assert_eq!(select_precision(1_000_000_000, &roomy), Precision::F32);

        // 1B params, only ~1.5 GiB VRAM → must drop to q8 (1 GB) — f32/f16 overflow.
        let tight_vram = PowerThermalBudget {
            vram_budget_bytes: gib + gib / 2,
            power_budget_mw: 60_000,
            thermal_headroom_c: 30.0,
        };
        assert_eq!(select_precision(1_000_000_000, &tight_vram), Precision::Q8);

        // Throttling (low thermal headroom) → never f32/f16 even with VRAM to spare.
        let throttling = PowerThermalBudget {
            vram_budget_bytes: 64 * gib,
            power_budget_mw: 60_000,
            thermal_headroom_c: 2.0,
        };
        assert_eq!(select_precision(1_000_000_000, &throttling), Precision::Q8);

        // Model too big for any precision → Q4 (caller tiles/offloads).
        let huge = PowerThermalBudget {
            vram_budget_bytes: gib,
            power_budget_mw: 60_000,
            thermal_headroom_c: 30.0,
        };
        assert_eq!(select_precision(10_000_000_000, &huge), Precision::Q4);
    }
}

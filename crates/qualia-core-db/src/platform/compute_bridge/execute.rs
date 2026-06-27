//! The dispatch entry the STEM substrate calls: `accelerated_gemm_f32` runs `C = A·B`
//! on the GPU when the **measured** capability matrix says it wins and the job is big
//! enough to be worth the dispatch, and on a `rayon` CPU path otherwise. The CPU path
//! is always present and never hard-fails (§7).
//!
//! The machine benchmark ([`crate::device_benchmark`]) runs **once** here, lazily, to
//! build the shared [`ComputePolicy`]; every subsequent call is the O(1) `select`.
//!
//! f64 vs f32: this accelerated path is **f32** (the GPU/WGSL reality and what the
//! throughput-bound callers want). The exact-f64 scientific GEMM
//! ([`crate::solvers::linear_algebra::gemm`]) is unchanged and stays on the CPU — the
//! bridge never silently downcasts a caller that asked for f64.

use std::sync::OnceLock;

use super::gpu_gemm;
use super::kernel_class::KernelClass;
use super::policy::ComputePolicy;
use crate::modalities::calculus::hetero_dispatch::{HostCapabilities, PowerThermalBudget};

/// Below this FLOP count a GEMM stays on the CPU regardless of the matrix — GPU upload
/// + dispatch + readback overhead dominates a small job. (`m·k·n` multiply-adds.)
const GPU_MIN_FLOPS: u64 = 1 << 20; // ~100³

/// Which backend actually ran a dispatch (for observability and the correctness gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RanOn {
    Gpu,
    Cpu,
}

static POLICY: OnceLock<ComputePolicy> = OnceLock::new();

/// The shared compute policy, built once by probing this machine (the benchmark). The
/// heavy probe runs on first call; thereafter `select` is O(1).
pub fn shared_policy() -> &'static ComputePolicy {
    POLICY.get_or_init(|| {
        let registry = super::default_registry();
        let panel = super::backend::KernelPanel::default();
        let budget = PowerThermalBudget {
            vram_budget_bytes: 4u64 << 30,
            power_budget_mw: 45_000,
            thermal_headroom_c: 20.0,
        };
        let host = HostCapabilities {
            gpu_available: gpu_gemm::shared().is_some(),
            vram_available: 2u64 << 30,
            npu_available: false,
            cpu_threads: num_cpus::get() as u32,
        };
        ComputePolicy::probe(&registry, &panel, budget, host)
    })
}

/// Multi-threaded CPU `C = A·B` (f32, row-major) — the always-present fallback.
fn cpu_gemm_f32(m: usize, k: usize, n: usize, a: &[f32], b: &[f32], c: &mut [f32]) {
    use rayon::prelude::*;
    c.par_chunks_mut(n).enumerate().for_each(|(i, row)| {
        let a_row = &a[i * k..(i + 1) * k];
        for (j, cell) in row.iter_mut().enumerate() {
            let mut s = 0.0f32;
            for l in 0..k {
                s += a_row[l] * b[l * n + j];
            }
            *cell = s;
        }
    });
}

/// Accelerated `C = A·B` (f32). `a` is `m×k`, `b` is `k×n`, `c` is `m×n` (overwritten).
/// Routes to the GPU when the measured matrix favours it for `DenseLinear`, the job
/// clears [`GPU_MIN_FLOPS`], and it fits in GPU buffers; otherwise the CPU path. Returns
/// which backend ran. On any GPU shortfall it falls back to CPU — never a hard fail.
pub fn accelerated_gemm_f32(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
    c: &mut [f32],
) -> RanOn {
    debug_assert_eq!(a.len(), m * k);
    debug_assert_eq!(b.len(), k * n);
    debug_assert_eq!(c.len(), m * n);

    let flops = (m as u64) * (k as u64) * (n as u64);
    if flops >= GPU_MIN_FLOPS {
        if let Some(ctx) = gpu_gemm::shared() {
            if ctx.fits(m, k, n) {
                let plan = shared_policy().select(KernelClass::DenseLinear, (m * n * 4) as u64);
                if !plan.is_cpu() {
                    if let Some(result) = ctx.gemm(m, k, n, a, b) {
                        c.copy_from_slice(&result);
                        return RanOn::Gpu;
                    }
                }
            }
        }
    }
    cpu_gemm_f32(m, k, n, a, b, c);
    RanOn::Cpu
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_gemm(m: usize, k: usize, n: usize, a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut c = vec![0.0f32; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0f32;
                for l in 0..k {
                    s += a[i * k + l] * b[l * n + j];
                }
                c[i * n + j] = s;
            }
        }
        c
    }

    #[test]
    fn small_job_runs_on_cpu_and_is_correct() {
        let (m, k, n) = (4, 5, 3);
        let a: Vec<f32> = (0..m * k).map(|i| i as f32 * 0.1).collect();
        let b: Vec<f32> = (0..k * n).map(|i| i as f32 * 0.2 - 0.5).collect();
        let mut c = vec![0.0f32; m * n];
        let ran = accelerated_gemm_f32(m, k, n, &a, &b, &mut c);
        assert_eq!(ran, RanOn::Cpu, "a tiny job must stay on CPU");
        assert_eq!(c, ref_gemm(m, k, n, &a, &b));
    }

    #[test]
    fn large_job_is_correct_on_whichever_backend_the_machine_chose() {
        // 128³ clears the FLOP floor. Whether it runs on GPU or CPU depends on the
        // measured matrix; either way the result must match the reference.
        let (m, k, n) = (128, 128, 128);
        let a: Vec<f32> = (0..m * k).map(|i| ((i % 17) as f32) * 0.05 - 0.3).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i % 11) as f32) * 0.07 - 0.2).collect();
        let mut c = vec![0.0f32; m * n];
        let ran = accelerated_gemm_f32(m, k, n, &a, &b, &mut c);
        eprintln!("[accelerated_gemm_f32] 128³ ran on {ran:?}");
        let reference = ref_gemm(m, k, n, &a, &b);
        let max_err = c.iter().zip(&reference).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(max_err < 1e-2, "max abs err {max_err} on {ran:?}");
    }
}

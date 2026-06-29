//! Capability-aware "best path on this machine" compute dispatcher, keystoned on GEMM.
//!
//! The forge proves and tunes individual kernels; this module is the layer that, at
//! runtime, picks the **best compute path actually available on this machine** for a
//! given call, while keeping a CPU floor so the call is *never* broken. All backends
//! are present in the code; which one activates is decided by the machine's probed
//! [`ComputeCaps`], not by the call site.
//!
//! # Why f32 and f64 take different best-paths
//!
//! WGSL has **no `f64`** — only `f32`/`f16`/`i32`/`u32`. So the GPU best-path for
//! single precision is the certified WGSL GEMM (via [`ForgeRuntime`]); but for
//! *double* precision there is no WGSL analogue at all. Native `f64` on the GPU has
//! to come from **CUDA/PTX**, which has a real `double` type and `fma.rn.f64`. That
//! is the whole reason [`gemm_f32`] and [`gemm_f64`] resolve to different backends:
//!
//! | dtype | best path (if available)        | floor (always present) |
//! |-------|---------------------------------|------------------------|
//! | f32   | WGSL GEMM ([`ForgeRuntime::gemm`]) | [`gemm_cpu`] (f32)   |
//! | f64   | native CUDA-f64 GEMM            | [`gemm_cpu_f64`]       |
//!
//! ## The remaining f64-on-non-NVIDIA slot (documented, not implemented)
//!
//! For `f64` on a **non-NVIDIA** GPU (no CUDA, but a wgpu adapter is present) the
//! known technique is *emulated* double precision in WGSL: double-single / `df64`
//! pair-arithmetic (a hi/lo `vec2<f32>` carrying ~44 effective mantissa bits via
//! error-free transforms — Dekker/TwoSum/TwoProd). That is **real, separate work**
//! (its own kernel, its own oracle, its own precision contract) and is **not**
//! implemented here — it is a clearly-marked future slot, deliberately not faked.
//! Today the f64 chain is exactly: **native CUDA-f64 → CPU**. On a non-NVIDIA GPU,
//! f64 therefore runs on the CPU floor (correct, just not GPU-accelerated) until the
//! df64-WGSL path is built and certified.

use std::sync::{Mutex, OnceLock};

use super::oracle::{gemm_cpu, gemv_cpu};
use super::ForgeError;
use super::execute::WgpuComputeContext;
use super::ForgeRuntime;

/// Problem-size threshold (in `m * n * k` multiply-adds) below which GEMM stays on
/// the CPU regardless of available accelerators. Small GEMMs are dominated by
/// dispatch/transfer overhead, so the GPU path only earns its keep above this size.
/// `1 << 15` (32768 FMAs, e.g. a 32×32×32 GEMM) is a conservative crossover that
/// keeps the hand-checked unit tests (well below it) firmly on the CPU floor.
pub const GEMM_GPU_THRESHOLD: usize = 1 << 15;

/// Probed compute capabilities of *this* machine. Every flag reflects what was
/// actually constructible at probe time, not what the build was compiled with — a
/// `cuda`-feature build on a machine with no NVIDIA device still reports
/// `cuda == false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeCaps {
    /// A wgpu adapter could be acquired (the WGSL GPU path is available).
    pub wgpu: bool,
    /// A CUDA device could be acquired (the native-f64 GPU path is available).
    /// Always `false` unless built with the `cuda` feature.
    pub cuda: bool,
    /// The wgpu adapter advertises cooperative-matrix (tensor-core) support.
    /// Read from the probe context's constraints; `false` when `wgpu` is `false`.
    pub coopmat: bool,
    /// The wgpu adapter advertises ray-query (RT-core) support. Read from the probe
    /// context's constraints; `false` when `wgpu` is `false`.
    pub rt: bool,
}

/// Probe size for the throwaway capability contexts. Small — these contexts are
/// built only to answer "can this backend initialise on this machine?" and are then
/// dropped; no real workload runs on them.
const PROBE_CAPACITY_BYTES: usize = 4 << 20;

static CAPS: OnceLock<ComputeCaps> = OnceLock::new();

/// The probed [`ComputeCaps`] for this machine, computed once and cached for the
/// process lifetime. The probe never panics: each backend is tried with
/// `..::new(_).is_ok()`, and any failure (no adapter, no driver, no device) is
/// simply recorded as the corresponding flag being `false`.
///
/// `coopmat`/`rt` are taken from the wgpu probe context's
/// [`AdapterConstraints`](super::AdapterConstraints) (the same flags the tuner uses
/// to prune tensor-core / ray-query kernels), so they are honest hardware bits, not
/// build-time assumptions.
pub fn caps() -> ComputeCaps {
    *CAPS.get_or_init(probe_caps)
}

fn probe_caps() -> ComputeCaps {
    // wgpu: build a throwaway context. If it constructs, the WGSL GPU path is live,
    // and its constraints carry the coopmat/rt hardware bits.
    let (wgpu, coopmat, rt) = match WgpuComputeContext::new(PROBE_CAPACITY_BYTES) {
        Ok(ctx) => (
            true,
            ctx.constraints.supports_coopmat,
            ctx.constraints.supports_rt_cores,
        ),
        Err(_) => (false, false, false),
    };

    let cuda = probe_cuda();

    ComputeCaps {
        wgpu,
        cuda,
        coopmat,
        rt,
    }
}

/// CUDA availability probe. Behind the `cuda` feature: try to build a throwaway
/// [`CudaComputeContext`](super::execute::CudaComputeContext); a missing toolkit /
/// device degrades to `Err` (the backend is built with `fallback-dynamic-loading`),
/// which we map to `false`. Without the `cuda` feature this is unconditionally
/// `false`.
#[cfg(feature = "cuda")]
fn probe_cuda() -> bool {
    use super::execute::CudaComputeContext;
    CudaComputeContext::new(PROBE_CAPACITY_BYTES).is_ok()
}

#[cfg(not(feature = "cuda"))]
fn probe_cuda() -> bool {
    false
}

/// Process-wide shared [`ForgeRuntime`] for the WGSL GEMM path.
///
/// Building a `ForgeRuntime` acquires a wgpu device/queue and slab, which is
/// expensive, so the dispatcher caches a single instance and reuses it across calls.
/// It lives behind `Mutex<Option<_>>` in a `OnceLock`: the `OnceLock` makes the
/// cell itself one-time-initialised, the `Mutex` serialises the `&mut self`
/// `ForgeRuntime::gemm` call (the GPU context is not `Sync`-shareable for concurrent
/// dispatch). `Option` lets a failed/again-unavailable runtime be retried lazily
/// without poisoning the slot permanently.
static FORGE_RT: OnceLock<Mutex<Option<ForgeRuntime>>> = OnceLock::new();

fn forge_rt_cell() -> &'static Mutex<Option<ForgeRuntime>> {
    FORGE_RT.get_or_init(|| Mutex::new(None))
}

/// Best-path single-precision dense GEMM: row-major `C[M×N] = A[M×K] · B[K×N]`.
///
/// Path selection:
/// 1. **WGSL GPU** — when [`caps().wgpu`](caps) is set *and* the problem is at least
///    [`GEMM_GPU_THRESHOLD`] FMAs, run the certified GEMM via the shared
///    [`ForgeRuntime`]. If the runtime cannot be built or the dispatch errors at
///    runtime, the error is **not** propagated — the call falls through to the CPU
///    floor so it is never broken.
/// 2. **CPU floor** — otherwise (no GPU, sub-threshold, or GPU fell through) compute
///    on the CPU via [`gemm_cpu`].
///
/// `a` must have `m * k` elements, `b` must have `k * n`; both row-major. Returns
/// `m * n` row-major elements. Dimension/length mismatches are the only hard errors.
pub fn gemm_f32(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    validate_dims(m, k, n, a.len(), b.len())?;

    let work = m.saturating_mul(n).saturating_mul(k);
    if caps().wgpu && work >= GEMM_GPU_THRESHOLD {
        if let Some(out) = gemm_f32_gpu(m, k, n, a, b) {
            return Ok(out);
        }
        // GPU path was eligible but failed at runtime — fall through to the CPU
        // floor rather than propagating, so the call is never broken.
    }

    Ok(gemm_cpu(a, b, m, k, n))
}

/// Run the f32 GEMM through the shared [`ForgeRuntime`], returning `None` on any
/// runtime failure (runtime un-buildable now, or dispatch error) so the caller can
/// fall through to the CPU floor. Never propagates a GPU error.
fn gemm_f32_gpu(m: usize, k: usize, n: usize, a: &[f32], b: &[f32]) -> Option<Vec<f32>> {
    let cell = forge_rt_cell();
    let mut guard = cell.lock().ok()?;
    if guard.is_none() {
        // Size the slab generously enough for the inputs+output of a typical GEMM;
        // ForgeRuntime allocates transiently per call within this capacity.
        match ForgeRuntime::new(64 * 1024 * 1024, None) {
            Ok(rt) => *guard = Some(rt),
            Err(_) => return None,
        }
    }
    let rt = guard.as_mut()?;
    rt.gemm(a, b, m, k, n).ok()
}

/// Best-path double-precision dense GEMM: row-major `C[M×N] = A[M×K] · B[K×N]`, all
/// `f64`.
///
/// Path selection (see the module doc for *why* this differs from [`gemm_f32`]):
/// 1. **native CUDA-f64 GPU** — when [`caps().cuda`](caps) is set *and* the problem
///    is at least [`GEMM_GPU_THRESHOLD`] FMAs, run the native double-precision CUDA
///    GEMM. On any runtime error the call falls through to the CPU floor (never
///    propagated).
/// 2. **CPU floor** — otherwise compute on the CPU via [`gemm_cpu_f64`].
///
/// There is intentionally **no WGSL path here**: WGSL has no `f64`. The
/// emulated-double (`df64` / double-single) WGSL path for f64 on non-NVIDIA GPUs is
/// a documented future slot (see the module doc), not implemented — so today the f64
/// chain is exactly **CUDA-f64 → CPU**.
///
/// `a` must have `m * k` elements, `b` must have `k * n`; both row-major. Returns
/// `m * n` row-major elements.
pub fn gemm_f64(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    validate_dims(m, k, n, a.len(), b.len())?;

    #[cfg(feature = "cuda")]
    {
        let work = m.saturating_mul(n).saturating_mul(k);
        if caps().cuda && work >= GEMM_GPU_THRESHOLD {
            if let Ok(out) = gemm_f64_cuda(m, k, n, a, b) {
                return Ok(out);
            }
            // CUDA path was eligible but errored — fall through to the CPU floor.
        }
    }

    Ok(gemm_cpu_f64(a, b, m, k, n))
}

/// Native CUDA double-precision GEMM. Builds a transient
/// [`CudaComputeContext`](super::execute::CudaComputeContext), uploads `a` (binding
/// 0) / `b` (binding 1) / a zeroed `c` (binding 2) and the `dims = [m, n, k]` u32
/// storage buffer (binding 3), compiles
/// [`GEMM_F64_SRC`](super::emit::cuda_c::GEMM_F64_SRC) via NVRTC, dispatches one
/// thread per output element (`element_count = m * n`), and reads back the `c`
/// buffer as `f64`. This is the exact-double path WGSL cannot provide.
#[cfg(feature = "cuda")]
fn gemm_f64_cuda(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    use super::emit::cuda_c::{GEMM_F64_ENTRY, GEMM_F64_SRC};
    use super::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    use super::Schedule;

    let mut ctx = CudaComputeContext::new(64 * 1024 * 1024)?;

    let element_count = m * n;
    let view_a = ctx.allocate_and_write(bytemuck::cast_slice(a), 0, 0)?;
    let view_b = ctx.allocate_and_write(bytemuck::cast_slice(b), 1, 0)?;
    let zeros = vec![0.0f64; element_count];
    let view_c = ctx.allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)?;
    // dims = [m, n, k] as u32, written as a storage buffer (binding 3) — no by-value
    // uniform, matching compile_cuda_c_source's pointer-only binding ABI.
    let dims: [u32; 3] = [m as u32, n as u32, k as u32];
    let view_dims = ctx.allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)?;

    let buffers = vec![view_a, view_b, view_c, view_dims];
    let pipeline =
        CudaPipeline::compile_cuda_c_source(&ctx, GEMM_F64_SRC, GEMM_F64_ENTRY, &[0, 1, 2, 3])?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    pipeline.dispatch(&buffers, &schedule, element_count)?;
    let mut out = ctx.read_buffer_f64(&view_c)?;
    out.truncate(element_count);
    Ok(out)
}

/// CPU reference for the double-precision dense GEMM — the `f64` mirror of
/// [`gemm_cpu`]: row-major `C[M×N] = A[M×K] · B[K×N]`,
/// `C[i][j] = sum_{k<K} A[i*K + k] * B[k*N + j]`. The inner `kk` sum order matches
/// the CUDA-f64 kernel so the two agree to f64 summation precision. This is the
/// always-present f64 floor.
pub fn gemm_cpu_f64(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    let mut c = vec![0.0f64; m * n];
    for i in 0..m {
        let a_row = i * k;
        for j in 0..n {
            let mut acc = 0.0f64;
            for kk in 0..k {
                acc += a[a_row + kk] * b[kk * n + j];
            }
            c[i * n + j] = acc;
        }
    }
    c
}

/// Best-path single-precision dense GEMV: row-major `y[M] = A[M×N] · x[N]`.
///
/// Path selection (mirrors [`gemm_f32`]):
/// 1. **WGSL GPU** — when [`caps().wgpu`](caps) is set *and* the problem is at least
///    [`GEMM_GPU_THRESHOLD`] MACs (`m * n`), run the certified GEMV via the shared
///    [`ForgeRuntime`]. A runtime build/dispatch failure is **not** propagated — the
///    call falls through to the CPU floor so it is never broken.
/// 2. **CPU floor** — otherwise compute on the CPU via [`gemv_cpu`].
///
/// `a` must have `m * n` elements (row-major) and `x` must have `n`. Returns `m`
/// row elements. Dimension/length mismatches are the only hard errors.
pub fn gemv_f32(
    m: usize,
    n: usize,
    a: &[f32],
    x: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    validate_gemv_dims(m, n, a.len(), x.len())?;

    let work = m.saturating_mul(n);
    if caps().wgpu && work >= GEMM_GPU_THRESHOLD {
        if let Some(out) = gemv_f32_gpu(m, n, a, x) {
            return Ok(out);
        }
        // GPU path was eligible but failed at runtime — fall through to the CPU
        // floor rather than propagating, so the call is never broken.
    }

    Ok(gemv_cpu(a, x, m, n))
}

/// Run the f32 GEMV through the shared [`ForgeRuntime`], returning `None` on any
/// runtime failure (runtime un-buildable now, or dispatch error) so the caller can
/// fall through to the CPU floor. Never propagates a GPU error.
fn gemv_f32_gpu(m: usize, n: usize, a: &[f32], x: &[f32]) -> Option<Vec<f32>> {
    let cell = forge_rt_cell();
    let mut guard = cell.lock().ok()?;
    if guard.is_none() {
        match ForgeRuntime::new(64 * 1024 * 1024, None) {
            Ok(rt) => *guard = Some(rt),
            Err(_) => return None,
        }
    }
    let rt = guard.as_mut()?;
    rt.gemv(a, x, m, n).ok()
}

/// Best-path double-precision dense GEMV: row-major `y[M] = A[M×N] · x[N]`, all
/// `f64`.
///
/// Path selection (see the module doc for *why* this differs from [`gemv_f32`] —
/// WGSL has no `f64`):
/// 1. **native CUDA-f64 GPU** — when [`caps().cuda`](caps) is set *and* the problem
///    is at least [`GEMM_GPU_THRESHOLD`] MACs (`m * n`), run the native
///    double-precision CUDA GEMV. On any runtime error the call falls through to the
///    CPU floor (never propagated).
/// 2. **CPU floor** — otherwise compute on the CPU via [`gemv_cpu_f64`].
///
/// There is intentionally **no WGSL path here**: WGSL has no `f64`. Today the f64
/// chain is exactly **CUDA-f64 → CPU**.
///
/// `a` must have `m * n` elements (row-major) and `x` must have `n`. Returns `m`
/// row elements.
pub fn gemv_f64(
    m: usize,
    n: usize,
    a: &[f64],
    x: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    validate_gemv_dims(m, n, a.len(), x.len())?;

    #[cfg(feature = "cuda")]
    {
        let work = m.saturating_mul(n);
        if caps().cuda && work >= GEMM_GPU_THRESHOLD {
            if let Ok(out) = gemv_f64_cuda(m, n, a, x) {
                return Ok(out);
            }
            // CUDA path was eligible but errored — fall through to the CPU floor.
        }
    }

    Ok(gemv_cpu_f64(a, x, m, n))
}

/// Native CUDA double-precision GEMV. Builds a transient
/// [`CudaComputeContext`](super::execute::CudaComputeContext), uploads `a` (binding
/// 0) / `x` (binding 1) / a zeroed `y` (binding 2) and the `dims = [m, n]` u32
/// storage buffer (binding 3), compiles
/// [`GEMV_F64_SRC`](super::emit::cuda_c::GEMV_F64_SRC) via NVRTC, dispatches one
/// thread per output row (`element_count = m`), and reads back the `y` buffer as
/// `f64`. This is the exact-double path WGSL cannot provide.
#[cfg(feature = "cuda")]
fn gemv_f64_cuda(
    m: usize,
    n: usize,
    a: &[f64],
    x: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    use super::emit::cuda_c::{GEMV_F64_ENTRY, GEMV_F64_SRC};
    use super::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    use super::Schedule;

    let mut ctx = CudaComputeContext::new(64 * 1024 * 1024)?;

    let element_count = m;
    let view_a = ctx.allocate_and_write(bytemuck::cast_slice(a), 0, 0)?;
    let view_x = ctx.allocate_and_write(bytemuck::cast_slice(x), 1, 0)?;
    let zeros = vec![0.0f64; element_count];
    let view_y = ctx.allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)?;
    // dims = [m, n] as u32, written as a storage buffer (binding 3) — no by-value
    // uniform, matching compile_cuda_c_source's pointer-only binding ABI.
    let dims: [u32; 2] = [m as u32, n as u32];
    let view_dims = ctx.allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)?;

    let buffers = vec![view_a, view_x, view_y, view_dims];
    let pipeline =
        CudaPipeline::compile_cuda_c_source(&ctx, GEMV_F64_SRC, GEMV_F64_ENTRY, &[0, 1, 2, 3])?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    pipeline.dispatch(&buffers, &schedule, element_count)?;
    let mut out = ctx.read_buffer_f64(&view_y)?;
    out.truncate(element_count);
    Ok(out)
}

/// CPU reference for the double-precision dense GEMV — the `f64` mirror of
/// [`gemv_cpu`]: row-major `y[M] = A[M×N] · x[N]`,
/// `y[i] = sum_{j<N} A[i*N + j] * x[j]`. The inner `j` sum order matches the
/// CUDA-f64 kernel so the two agree to f64 summation precision. This is the
/// always-present f64 floor.
pub fn gemv_cpu_f64(a: &[f64], x: &[f64], m: usize, n: usize) -> Vec<f64> {
    let mut y = vec![0.0f64; m];
    for i in 0..m {
        let a_row = i * n;
        let mut acc = 0.0f64;
        for j in 0..n {
            acc += a[a_row + j] * x[j];
        }
        y[i] = acc;
    }
    y
}

/// Shared dimension/length validation for both GEMV entry points: `a` is `m*n`
/// (row-major) and `x` is `n`.
fn validate_gemv_dims(
    m: usize,
    n: usize,
    a_len: usize,
    x_len: usize,
) -> Result<(), ForgeError> {
    if m == 0 || n == 0 {
        return Err(ForgeError::GpuValidation(
            "gemv requires m > 0 and n > 0".to_string(),
        ));
    }
    if a_len != m * n {
        return Err(ForgeError::GpuValidation(format!(
            "a must have m*n = {} elements; got {}",
            m * n,
            a_len
        )));
    }
    if x_len != n {
        return Err(ForgeError::GpuValidation(format!(
            "x must have n = {} elements; got {}",
            n, x_len
        )));
    }
    Ok(())
}

/// Shared dimension/length validation for both GEMM entry points.
fn validate_dims(
    m: usize,
    k: usize,
    n: usize,
    a_len: usize,
    b_len: usize,
) -> Result<(), ForgeError> {
    if m == 0 || k == 0 || n == 0 {
        return Err(ForgeError::GpuValidation(
            "gemm requires m > 0, k > 0, and n > 0".to_string(),
        ));
    }
    if a_len != m * k {
        return Err(ForgeError::GpuValidation(format!(
            "a must have m*k = {} elements; got {}",
            m * k,
            a_len
        )));
    }
    if b_len != k * n {
        return Err(ForgeError::GpuValidation(format!(
            "b must have k*n = {} elements; got {}",
            k * n,
            b_len
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The capability probe must never panic, on any machine, regardless of which
    /// backends are present. (It is also memoised, so this just calls it.)
    #[test]
    fn caps_probe_never_panics() {
        let c = caps();
        // A second call returns the same cached value.
        assert_eq!(c, caps());
    }

    /// Non-GPU: a sub-threshold f32 GEMM is forced onto the CPU floor and must match
    /// the hand-checked 2×3 · 3×2 reference [58, 64, 139, 154].
    /// A=[[1,2,3],[4,5,6]], B=[[7,8],[9,10],[11,12]].
    #[test]
    fn gemm_f32_cpu_fallback_is_correct() {
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0f32, 8.0, 9.0, 10.0, 11.0, 12.0];
        // 2*2*3 = 12 FMAs, far below GEMM_GPU_THRESHOLD, so this is the CPU path
        // even on a GPU machine.
        let out = gemm_f32(2, 3, 2, &a, &b).expect("gemm_f32");
        assert_eq!(out, vec![58.0, 64.0, 139.0, 154.0]);
    }

    /// Non-GPU: the f64 twin of the above, on the f64 CPU floor.
    #[test]
    fn gemm_f64_cpu_fallback_is_correct() {
        let a = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0f64, 8.0, 9.0, 10.0, 11.0, 12.0];
        let out = gemm_f64(2, 3, 2, &a, &b).expect("gemm_f64");
        assert_eq!(out, vec![58.0, 64.0, 139.0, 154.0]);
    }

    /// Dimension mismatches are hard errors on both entry points.
    #[test]
    fn gemm_dim_mismatch_errors() {
        assert!(gemm_f32(2, 3, 2, &[1.0; 5], &[1.0; 6]).is_err());
        assert!(gemm_f64(2, 3, 2, &[1.0; 6], &[1.0; 5]).is_err());
        assert!(gemm_f32(0, 3, 2, &[], &[1.0; 6]).is_err());
    }

    /// `gemm_cpu_f64` agrees with the f32 reference on small exact-integer inputs,
    /// pinning the f64 floor's layout/sum order independently of the dispatcher.
    #[test]
    fn gemm_cpu_f64_matches_hand_checked() {
        let a = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0f64, 8.0, 9.0, 10.0, 11.0, 12.0];
        assert_eq!(gemm_cpu_f64(&a, &b, 2, 3, 2), vec![58.0, 64.0, 139.0, 154.0]);
    }

    /// Non-GPU: a sub-threshold f32 GEMV is forced onto the CPU floor and must match
    /// the hand-checked 2×3 · 3 reference [6, 15].
    /// A=[[1,2,3],[4,5,6]], x=[1,1,1].
    #[test]
    fn gemv_f32_cpu_fallback_is_correct() {
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let x = [1.0f32, 1.0, 1.0];
        // 2*3 = 6 MACs, far below GEMM_GPU_THRESHOLD, so this is the CPU path even on
        // a GPU machine.
        let out = gemv_f32(2, 3, &a, &x).expect("gemv_f32");
        assert_eq!(out, vec![6.0, 15.0]);
    }

    /// Non-GPU: the f64 twin of the above, on the f64 CPU floor.
    #[test]
    fn gemv_f64_cpu_fallback_is_correct() {
        let a = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let x = [1.0f64, 1.0, 1.0];
        let out = gemv_f64(2, 3, &a, &x).expect("gemv_f64");
        assert_eq!(out, vec![6.0, 15.0]);
    }

    /// Dimension mismatches are hard errors on both GEMV entry points.
    #[test]
    fn gemv_dim_mismatch_errors() {
        assert!(gemv_f32(2, 3, &[1.0; 5], &[1.0; 3]).is_err()); // a too short
        assert!(gemv_f64(2, 3, &[1.0; 6], &[1.0; 2]).is_err()); // x too short
        assert!(gemv_f32(0, 3, &[], &[1.0; 3]).is_err()); // m == 0
    }

    /// `gemv_cpu_f64` agrees with the hand-checked reference on small exact-integer
    /// inputs, pinning the f64 floor's layout/sum order independently of the
    /// dispatcher.
    #[test]
    fn gemv_cpu_f64_matches_hand_checked() {
        let a = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let x = [1.0f64, 1.0, 1.0];
        assert_eq!(gemv_cpu_f64(&a, &x, 2, 3), vec![6.0, 15.0]);
    }

    // ── GPU / CUDA end-to-end tests (require a real device; run by the orchestrator) ──

    /// Deterministic xorshift fill in [-1, 1], so GPU and CPU see identical inputs.
    #[cfg(test)]
    fn det_f32(len: usize, seed: u64) -> Vec<f32> {
        let mut state = seed.max(1);
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unit = (state as u32) as f32 / u32::MAX as f32;
            v.push(unit.mul_add(2.0, -1.0));
        }
        v
    }

    /// Above-threshold f32 GEMM on the WGSL GPU path must match the CPU reference
    /// within f32 summation tolerance. m=k=n=64 → 262144 FMAs ≥ threshold.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn gemm_f32_gpu_matches_cpu() {
        let (m, k, n) = (64usize, 64, 64);
        let a = det_f32(m * k, 0x6745_4D4D_4633_3201);
        let b = det_f32(k * n, 0x6745_4D4D_4633_3202);
        let gpu = gemm_f32(m, k, n, &a, &b).expect("gemm_f32 gpu");
        let cpu = gemm_cpu(&a, &b, m, k, n);
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert!(
                (g - c).abs() <= 1.0e-3,
                "f32 GPU/CPU mismatch: {g} vs {c}"
            );
        }
    }

    /// Above-threshold f32 GEMV on the WGSL GPU path must match the CPU reference
    /// within f32 summation tolerance. m=n=256 → 65536 MACs ≥ threshold.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn gemv_f32_gpu_matches_cpu() {
        let (m, n) = (256usize, 256);
        let a = det_f32(m * n, 0x6745_4D56_4633_3201);
        let x = det_f32(n, 0x6745_4D56_4633_3202);
        let gpu = gemv_f32(m, n, &a, &x).expect("gemv_f32 gpu");
        let cpu = gemv_cpu(&a, &x, m, n);
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert!((g - c).abs() <= 1.0e-3, "f32 GPU/CPU mismatch: {g} vs {c}");
        }
    }

    /// Above-threshold f64 GEMM on the native CUDA path must match the f64 CPU
    /// reference to near-exact precision (native double, no emulation).
    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn gemm_f64_cuda_matches_cpu() {
        let (m, k, n) = (64usize, 64, 64);
        let a: Vec<f64> = det_f32(m * k, 0x6745_4D4D_4636_3401)
            .into_iter()
            .map(|x| x as f64)
            .collect();
        let b: Vec<f64> = det_f32(k * n, 0x6745_4D4D_4636_3402)
            .into_iter()
            .map(|x| x as f64)
            .collect();
        let gpu = gemm_f64(m, k, n, &a, &b).expect("gemm_f64 cuda");
        let cpu = gemm_cpu_f64(&a, &b, m, k, n);
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert!((g - c).abs() <= 1.0e-9, "f64 CUDA/CPU mismatch: {g} vs {c}");
        }
    }

    /// Above-threshold f64 GEMV on the native CUDA path must match the f64 CPU
    /// reference to near-exact precision (native double, no emulation). m=n=256.
    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn gemv_f64_cuda_matches_cpu() {
        let (m, n) = (256usize, 256);
        let a: Vec<f64> = det_f32(m * n, 0x6745_4D56_4636_3401)
            .into_iter()
            .map(|v| v as f64)
            .collect();
        let x: Vec<f64> = det_f32(n, 0x6745_4D56_4636_3402)
            .into_iter()
            .map(|v| v as f64)
            .collect();
        let gpu = gemv_f64(m, n, &a, &x).expect("gemv_f64 cuda");
        let cpu = gemv_cpu_f64(&a, &x, m, n);
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert!((g - c).abs() <= 1.0e-9, "f64 CUDA/CPU mismatch: {g} vs {c}");
        }
    }
}

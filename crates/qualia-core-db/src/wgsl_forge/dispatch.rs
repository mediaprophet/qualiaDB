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
//! | dtype | best path (if available)                    | floor (always present) |
//! |-------|---------------------------------------------|------------------------|
//! | f32   | WGSL GEMM ([`ForgeRuntime::gemm`])          | [`gemm_cpu`] (f32)     |
//! | f64   | native CUDA-f64 → df64-WGSL (double-single) | [`gemm_cpu_f64`]       |
//!
//! ## f64 on every GPU: the 3-tier chain (native CUDA → df64-WGSL → CPU)
//!
//! `f64` now has a GPU path on **every** machine, not just NVIDIA. The chain in
//! [`gemm_f64`] is three tiers:
//!
//! 1. **native CUDA-f64** ([`gemm_f64_cuda`]) — exact double via PTX `fma.rn.f64`,
//!    NVIDIA only (`cuda` feature + a CUDA device).
//! 2. **df64 / double-single WGSL** ([`gemm_f64_df64`]) — *emulated* double on any
//!    other wgpu adapter (AMD, Intel, Apple, mobile). Each `f64` is a hi/lo pair of
//!    `f32` and the accumulation uses error-free transforms (Dekker/TwoSum/TwoProd),
//!    giving ~44–48 effective mantissa bits — well beyond a single `f32`'s 24. The
//!    kernel is the raw WGSL [`GEMM_DF64_WGSL`](super::emit::GEMM_DF64_WGSL).
//! 3. **CPU floor** ([`gemm_cpu_f64`]) — exact double, always present, never broken.
//!
//! So a non-NVIDIA GPU can get real f64 *acceleration* (tier 2) instead of dropping
//! straight to the CPU — **but only where the adapter's WGSL float arithmetic preserves
//! the df64 error-free transforms.** Many drivers (incl. the naga→SPIR-V→NVIDIA-Vulkan
//! path) reassociate floats (`c - (c - a)` → `a`, `fma(x,y,-(x*y))` → `0`), which
//! silently collapses df64 to f32 precision. WGSL exposes no portable way to forbid
//! that, so tier 2 is gated on a runtime precision probe ([`df64_usable`]): df64 runs
//! only where it actually delivers ~double precision; elsewhere the chain uses native
//! CUDA (if present) or the exact CPU floor — never a degraded df64 masquerading as f64.
//! (GEMV's f64 chain is `CUDA-f64 → CPU` — the df64 path is GEMM-only today.)

use std::sync::{Mutex, OnceLock};

use super::execute::WgpuComputeContext;
use super::oracle::{dft_cpu, gemm_cpu, gemv_cpu};
use super::ForgeError;
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
/// # The 3-tier f64 chain ("best f64 path on every machine")
///
/// WGSL has no native `f64`, so double precision on the GPU is reached two different
/// ways depending on the hardware; this is the whole reason `gemm_f64` resolves
/// through three tiers rather than the single accelerator arm of [`gemm_f32`]:
///
/// | tier | path                              | when                                                    |
/// |------|-----------------------------------|---------------------------------------------------------|
/// | 1    | **native CUDA-f64** ([`gemm_f64_cuda`], NVIDIA only) | [`caps().cuda`](caps) and ≥ [`GEMM_GPU_THRESHOLD`] FMAs |
/// | 2    | **df64 / double-single WGSL** ([`gemm_f64_df64`], any other GPU) | [`caps().wgpu`](caps) and ≥ [`GEMM_GPU_THRESHOLD`] FMAs |
/// | 3    | **CPU floor** ([`gemm_cpu_f64`])  | otherwise, or if every eligible accelerator errors      |
///
/// Tier 1 is *exact* double (native `double` + `fma.rn.f64`). Tier 2 emulates each
/// `f64` as a hi/lo pair of `f32` with error-free transforms (~44–48 effective
/// mantissa bits, well beyond a single `f32`'s 24) — so a non-NVIDIA GPU (AMD,
/// Intel, Apple, mobile) now gets real f64 *acceleration* instead of dropping
/// straight to the CPU. On any accelerator runtime error the call falls through to
/// the next tier (errors are **never** propagated), so it is never broken. The CUDA
/// arm is compiled in only under the `cuda` feature; the df64 arm is always present
/// (it needs only a wgpu adapter).
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

    let work = m.saturating_mul(n).saturating_mul(k);

    // Tier 1: native CUDA-f64 (exact double) on an NVIDIA device.
    #[cfg(feature = "cuda")]
    {
        if caps().cuda && work >= GEMM_GPU_THRESHOLD {
            if let Ok(out) = gemm_f64_cuda(m, k, n, a, b) {
                return Ok(out);
            }
            // CUDA path was eligible but errored — fall through to the next tier.
        }
    }

    // Tier 2: df64 (double-single) emulated-f64 in WGSL — but ONLY on adapters whose
    // WGSL float semantics actually preserve the error-free transforms. Many drivers
    // (incl. the naga->SPIR-V->NVIDIA-Vulkan path) reassociate floats, collapsing the
    // df64 residuals to ~0 (f32 precision); `df64_usable()` probes for that at runtime
    // so we never return f32-precision results dressed up as f64 — we drop to the CPU
    // floor (exact f64) instead.
    if caps().wgpu && work >= GEMM_GPU_THRESHOLD && df64_usable() {
        if let Ok(out) = gemm_f64_df64(m, k, n, a, b) {
            return Ok(out);
        }
        // df64 path was eligible but errored — fall through to the CPU floor.
    }

    // Tier 3: CPU floor (always present, never broken).
    Ok(gemm_cpu_f64(a, b, m, k, n))
}

/// Runtime probe: does this adapter's WGSL float arithmetic preserve the df64
/// error-free transforms (genuine ~double precision), or does the driver reassociate
/// floats and silently collapse df64 to f32? Measured once, then cached.
///
/// df64 (double-single) is correct only where each f32 `+`/`-`/`*` rounds per IEEE
/// without reassociation. Some drivers — notably the naga->SPIR-V->NVIDIA-Vulkan path
/// on this hardware — algebraically simplify `c - (c - a)` to `a` and `fma(x,y,-(x*y))`
/// to `0`, which destroys the residual (lo) terms. WGSL exposes no portable way to
/// forbid that, so we MEASURE it: run a tiny df64 GEMM whose exact f64 result differs
/// from f32 by ~1e-7, and accept df64 only if it lands within f64 tolerance. On
/// adapters that fail the probe, the f64 chain uses native CUDA (if present) or the
/// exact CPU floor — never a degraded df64.
fn df64_usable() -> bool {
    static USABLE: OnceLock<bool> = OnceLock::new();
    *USABLE.get_or_init(|| {
        if !caps().wgpu {
            return false;
        }
        // 8x8x8 with low-mantissa-bit perturbations: the exact f64 result differs from
        // an f32 evaluation by ~1e-7, so only a working df64 lands within 1e-9.
        let n = 8usize;
        let a: Vec<f64> = (0..n * n)
            .map(|i| 1.0 + (i as f64) * 1.0e-7 + 1.0e-9)
            .collect();
        let b: Vec<f64> = (0..n * n)
            .map(|i| 1.0 - (i as f64) * 1.0e-7 + 3.0e-10)
            .collect();
        let cpu = gemm_cpu_f64(&a, &b, n, n, n);
        match gemm_f64_df64(n, n, n, &a, &b) {
            Ok(df) => df.iter().zip(&cpu).all(|(d, c)| (d - c).abs() <= 1.0e-9),
            Err(_) => false,
        }
    })
}

/// Runtime probe: does this adapter's WGSL **cooperative-matrix** (tensor-core) multiply
/// actually compute, or does it return zeros? Measured once, then cached — the f32 mirror
/// of [`df64_usable`].
///
/// The coopmat kernels are correct and naga-validated, but on wgpu 29.0.3 the
/// `coopMultiplyAdd` is a no-op that returns all-zeros (gfx-rs/wgpu #9741: coopmat emits
/// Device-scope SPIR-V memory ops invalid unless `vulkanMemoryModelDeviceScope` is
/// auto-enabled at device creation — fixed on `main` after 29.0.3, the newest crates.io
/// release). So we never *assume* coopmat works from the advertised feature bit: we MEASURE
/// it by running a tiny 8×8×8 coopmat GEMM whose exact result is non-zero (all-ones inputs
/// → every output `= 8.0`) and accepting coopmat only if the result matches. On 29.0.3 this
/// returns `false` (zeros); it returns `true` automatically once a wgpu release (or the
/// [`docs/WGPU_UPSTREAM_TRACKING.md`] soft-fork) carries the fix. Gated first on
/// [`caps().wgpu`](caps) and [`caps().coopmat`](caps) so non-coopmat adapters never dispatch.
pub fn coopmat_usable() -> bool {
    static USABLE: OnceLock<bool> = OnceLock::new();
    *USABLE.get_or_init(|| {
        let c = caps();
        if !c.wgpu || !c.coopmat {
            return false;
        }
        // 8×8×8, all-ones: exact C[i][j] = sum_{0..8} 1*1 = 8.0. A working coopmat lands on
        // 8.0; the #9741 no-op returns 0.0, so the tolerance check rejects it.
        let n = 8usize;
        let a = vec![1.0f32; n * n];
        let b = vec![1.0f32; n * n];
        match gemm_f32_tc_coopmat(n, n, n, &a, &b) {
            Ok(out) => out.len() == n * n && out.iter().all(|&v| (v - 8.0).abs() <= 1.0e-3),
            Err(_) => false,
        }
    })
}

/// **Cooperative-matrix (tensor-core) f32 GEMM on a wgpu adapter**: row-major
/// `C[m×n] = A[m×k]·B[k×n]`, all `f32`, computed by the tiled coopmat kernel
/// ([`matmul_tc_wgsl_tiled`](super::emit::matmul_tc_wgsl_tiled)). `m`, `n`, `k` must be
/// non-zero multiples of 8 (the 8×8×8 cooperative-matrix tile).
///
/// This is the *portable* tensor-core path — it needs only a coopmat-capable wgpu adapter
/// (no CUDA), so it covers NVIDIA/AMD/Intel/Apple alike once the driver computes coopmat.
/// **It is dormant on wgpu 29.0.3** (the multiply returns zeros, #9741); callers gate on
/// [`coopmat_usable`] so it is invoked only where it actually computes.
///
/// Mechanics mirror [`gemm_f64_df64`]: a transient [`WgpuComputeContext`], `a`(0,
/// [`StorageRead`]) / `b`(1, [`StorageRead`]) / zeroed `c`(2, [`StorageReadWrite`]) /
/// `dims=[m,n,k]`(3, [`StorageRead`]), compile the tiled kernel, dispatch one workgroup
/// (== one subgroup, `@workgroup_size(32)`) per 8×8 output tile (`num_tiles = (m/8)·(n/8)`),
/// read back `c`.
///
/// [`StorageRead`]: super::execute::BindingUsage::StorageRead
/// [`StorageReadWrite`]: super::execute::BindingUsage::StorageReadWrite
pub fn gemm_f32_tc_coopmat(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    use super::emit::{matmul_tc_wgsl_tiled, MATMUL_TC_TILED_ENTRY};
    use super::execute::{BindingUsage, QualiaCompute, WgpuPipeline};
    use super::Schedule;

    if m == 0 || n == 0 || k == 0 || m % 8 != 0 || n % 8 != 0 || k % 8 != 0 {
        return Err(ForgeError::GpuValidation(format!(
            "gemm_f32_tc_coopmat: m={m}, n={n}, k={k} must be non-zero multiples of 8 (coopmat tile)"
        )));
    }
    validate_dims(m, k, n, a.len(), b.len())?;

    let element_count = m.checked_mul(n).ok_or_else(|| {
        ForgeError::GpuValidation("m*n overflow in gemm_f32_tc_coopmat".to_string())
    })?;
    let capacity = (element_count.saturating_mul(8)).max(4 << 20);
    let mut ctx = WgpuComputeContext::new(capacity)?;

    let view_a =
        ctx.allocate_and_write(bytemuck::cast_slice(a), 0, 0, BindingUsage::StorageRead)?;
    let view_b =
        ctx.allocate_and_write(bytemuck::cast_slice(b), 1, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; element_count];
    let view_c = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let dims: [u32; 3] = [m as u32, n as u32, k as u32];
    let view_dims =
        ctx.allocate_and_write(bytemuck::cast_slice(&dims), 3, 0, BindingUsage::StorageRead)?;

    let buffers = vec![view_a, view_b, view_c, view_dims];
    let src = matmul_tc_wgsl_tiled();
    let pipeline = WgpuPipeline::compile(&ctx, &src, MATMUL_TC_TILED_ENTRY)?;
    // One workgroup (== one subgroup, @workgroup_size(32)) per 8×8 output tile.
    let num_tiles = (m / 8) * (n / 8);
    let schedule = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    pipeline.dispatch(&buffers, &schedule, num_tiles * 32)?;
    let mut out = ctx.read_buffer_f32(&view_c)?;
    out.truncate(element_count);
    Ok(out)
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

/// **Opt-in tensor-core GEMM** (reduced precision) — the capability-selected entry point
/// that makes the `MatMul.tc` request real. Row-major `C[m×n] = A[m×k]·B[k×n]`, f32 in/out.
///
/// Selection (best tensor-core path on this machine, with a correct floor):
/// 1. **WGSL coopmat** ([`gemm_f32_tc_coopmat`]) — the *portable* wgpu tensor-core path
///    (f32), the intended first choice. Built (tiled kernel + runtime probe), but gated on
///    [`coopmat_usable`]: on wgpu 29.0.3 the coopmat multiply returns zeros (#9741), so the
///    probe is `false` and this tier is **dormant until a wgpu release / soft-fork carries
///    the fix**, then self-activates (see [`docs/WGPU_UPSTREAM_TRACKING.md`]). 8-multiple dims.
/// 2. **CUDA WMMA** ([`gemm_tc_cuda`]) — genuine NVIDIA tensor cores at f16-input precision,
///    when `cuda` is available and `m,n,k` are multiples of 16. Carries tensor cores **today**.
/// 3. **plain f32 GEMM** ([`gemm_f32`]) — the always-correct floor (full f32 precision).
///
/// This is **opt-in** because tiers 1–2 trade f32 precision for tensor-core throughput; the
/// default [`gemm_f32`] stays full-precision. Use for LLM matmuls (already f16-tolerant).
pub fn gemm_f32_tc(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    validate_dims(m, k, n, a.len(), b.len())?;

    // Tier 1: WGSL coopmat — the *portable* wgpu tensor-core path (f32), gated on the
    // runtime probe `coopmat_usable()`. On wgpu 29.0.3 the coopmat multiply returns zeros
    // (#9741), so the probe is `false` and this tier stays dormant — it self-activates the
    // moment a wgpu release (or the soft-fork) carries the fix. Requires 8-multiple dims
    // (the 8×8×8 tile). The probe runs at most one tiny GPU dispatch, then caches.
    if caps().wgpu
        && caps().coopmat
        && m % 8 == 0
        && n % 8 == 0
        && k % 8 == 0
        && m.min(n).min(k) > 0
        && coopmat_usable()
    {
        if let Ok(out) = gemm_f32_tc_coopmat(m, k, n, a, b) {
            return Ok(out);
        }
        // Coopmat path eligible but errored — fall through to the next tier.
    }

    // Tier 2: CUDA WMMA (genuine NVIDIA tensor cores, f16-input precision).
    #[cfg(feature = "cuda")]
    {
        if caps().cuda && m % 16 == 0 && n % 16 == 0 && k % 16 == 0 && m.min(n).min(k) > 0 {
            if let Ok(out) = gemm_tc_cuda(m, k, n, a, b) {
                return Ok(out);
            }
            // Tensor-core path eligible but errored — fall through to the exact floor.
        }
    }
    gemm_f32(m, k, n, a, b)
}

/// **Tensor-core** GEMM via the tiled CUDA WMMA kernel: row-major `C[m×n] = A[m×k]·B[k×n]`,
/// with `A`/`B` rounded to **f16** and accumulated in **f32** on NVIDIA tensor cores. This
/// is the genuine reduced-precision tensor-core path — the throughput win that the plain
/// f32 GEMM cannot get — exposed as an **opt-in** (`MatMul.tc`) because it trades f32
/// precision for speed. `m`, `n`, `k` must be non-zero multiples of 16 (the WMMA tile);
/// callers with other shapes pad or fall back to the plain path.
///
/// f32 inputs are converted to f16 bit patterns host-side and uploaded as `u16`; the
/// `dims = [m, n, k]` storage buffer drives the kernel's tiling. Returns `m*n` f32 outputs.
#[cfg(feature = "cuda")]
pub fn gemm_tc_cuda(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::emit::cuda_c::{WMMA_GEMM_TILED_ENTRY, WMMA_GEMM_TILED_SRC};
    use crate::wgsl_forge::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};

    if m == 0 || n == 0 || k == 0 || m % 16 != 0 || n % 16 != 0 || k % 16 != 0 {
        return Err(ForgeError::GpuValidation(format!(
            "gemm_tc_cuda: m={m}, n={n}, k={k} must be non-zero multiples of 16 (WMMA tile)"
        )));
    }
    validate_dims(m, k, n, a.len(), b.len())?;

    let a_bits: Vec<u16> = a
        .iter()
        .map(|&x| half::f16::from_f32(x).to_bits())
        .collect();
    let b_bits: Vec<u16> = b
        .iter()
        .map(|&x| half::f16::from_f32(x).to_bits())
        .collect();

    let mut ctx = CudaComputeContext::new(64 * 1024 * 1024)?;
    let view_a = ctx.allocate_and_write(bytemuck::cast_slice(&a_bits), 0, 0)?;
    let view_b = ctx.allocate_and_write(bytemuck::cast_slice(&b_bits), 1, 0)?;
    let zeros = vec![0.0f32; m * n];
    let view_c = ctx.allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)?;
    let dims: [u32; 3] = [m as u32, n as u32, k as u32];
    let view_dims = ctx.allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)?;

    let buffers = vec![view_a, view_b, view_c, view_dims];
    let num_tiles = (m / 16) * (n / 16);
    // workgroup_size 32 (one warp/tile) → element_count = num_tiles*32 gives num_tiles blocks.
    let schedule = super::Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    let pipeline = CudaPipeline::compile_cuda_c_source(
        &ctx,
        WMMA_GEMM_TILED_SRC,
        WMMA_GEMM_TILED_ENTRY,
        &[0, 1, 2, 3],
    )?;
    pipeline.dispatch(&buffers, &schedule, num_tiles * 32)?;
    let mut out = ctx.read_buffer_f32(&view_c)?;
    out.truncate(m * n);
    Ok(out)
}

/// Split one `f64` into a double-single (`df64`) hi/lo pair of `f32`. `hi` is the
/// `f64` rounded to nearest `f32`; `lo` is the (exactly representable in `f32`)
/// residual `v - hi`. Together the pair carries ~44–48 effective mantissa bits, far
/// beyond a single `f32`'s 24. Inverse of [`df32_to_f64`].
fn f64_to_df32(v: f64) -> [f32; 2] {
    let hi = v as f32;
    let lo = (v - hi as f64) as f32;
    [hi, lo]
}

/// Recombine a double-single (`df64`) hi/lo `f32` pair back into an `f64`. The sum
/// is exact in `f64` (both operands are `f32`-representable and `|lo| ≤ ½ ulp(hi)`),
/// so this is the exact inverse of [`f64_to_df32`] up to the `f32` rounding of `hi`.
fn df32_to_f64(hi: f32, lo: f32) -> f64 {
    hi as f64 + lo as f64
}

/// Pack an `&[f64]` into a flat `Vec<f32>` of twice the length, hi/lo interleaved
/// per element (`[hi0, lo0, hi1, lo1, …]`) — the df64 GEMM's input layout. Inverse
/// of [`unpack_df32`].
fn pack_df32(values: &[f64]) -> Vec<f32> {
    let mut out = Vec::with_capacity(values.len() * 2);
    for &v in values {
        let [hi, lo] = f64_to_df32(v);
        out.push(hi);
        out.push(lo);
    }
    out
}

/// Unpack a flat `&[f32]` of hi/lo-interleaved df64 pairs (`[hi0, lo0, hi1, lo1, …]`)
/// back into an `&[f64]` of half the length. Inverse of [`pack_df32`]. A trailing
/// half-pair (odd length) is ignored.
fn unpack_df32(packed: &[f32]) -> Vec<f64> {
    packed
        .chunks_exact(2)
        .map(|pair| df32_to_f64(pair[0], pair[1]))
        .collect()
}

/// Emulated double-precision (`df64` / double-single) dense GEMM on **any** wgpu
/// adapter: row-major `C[M×N] = A[M×K] · B[K×N]`, all `f64`.
///
/// WGSL has no `f64`, so each double is carried as a hi/lo pair of `f32` and the
/// accumulation runs with error-free transforms (Dekker/Knuth `two_prod`/`two_sum`)
/// inside the raw kernel [`GEMM_DF64_WGSL`]. This is the portable f64-on-GPU path
/// that complements the NVIDIA-only native-CUDA-f64 path: an AMD/Intel/Apple/mobile
/// GPU gets real f64 acceleration here, at ~44–48 effective mantissa bits (vs a
/// single f32's 24).
///
/// Mechanics (mirrors the raw-source path of
/// [`crate::wgsl_forge::oracle::evaluate_coopmat_loadstore`]): build a transient
/// [`WgpuComputeContext`], pack `a`→`2*M*K` f32 (binding 0, [`StorageRead`]) and
/// `b`→`2*K*N` f32 (binding 1, [`StorageRead`]), allocate a zeroed `c` of `2*M*N`
/// f32 (binding 2, [`StorageReadWrite`]) and `dims = [m, n, k]` as `u32` (binding 3,
/// [`StorageRead`]), compile [`GEMM_DF64_WGSL`] / [`GEMM_DF64_ENTRY`], dispatch one
/// invocation per output element (`element_count = m * n`, `workgroup_size = 64`),
/// read back `c` as `2*M*N` f32 and unpack to `M*N` f64.
///
/// [`StorageRead`]: super::execute::BindingUsage::StorageRead
/// [`StorageReadWrite`]: super::execute::BindingUsage::StorageReadWrite
pub fn gemm_f64_df64(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    use super::emit::{GEMM_DF64_ENTRY, GEMM_DF64_WGSL};
    use super::execute::{BindingUsage, QualiaCompute, WgpuPipeline};
    use super::Schedule;

    // Slab must hold a (2*M*K f32) + b (2*K*N f32) on the read slab and c (2*M*N f32)
    // + dims (3 u32) on the out/read slabs. Size to M*N*16 bytes of headroom (>= the
    // 2*M*N f32 = M*N*8-byte output, doubled), floored at 4 MiB so small GEMMs still
    // fit comfortably alongside the inputs.
    let element_count = m
        .checked_mul(n)
        .ok_or_else(|| ForgeError::GpuValidation("m*n overflow in gemm_f64_df64".to_string()))?;
    let capacity = (element_count.saturating_mul(16)).max(4 << 20);
    let mut ctx = WgpuComputeContext::new(capacity)?;

    let a_packed = pack_df32(a); // 2*M*K f32
    let b_packed = pack_df32(b); // 2*K*N f32
    let view_a = ctx.allocate_and_write(
        bytemuck::cast_slice(&a_packed),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let view_b = ctx.allocate_and_write(
        bytemuck::cast_slice(&b_packed),
        1,
        0,
        BindingUsage::StorageRead,
    )?;
    let zeros = vec![0.0f32; element_count * 2]; // 2*M*N f32
    let view_c = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    // dims is a u32 storage buffer [m, n, k] (binding 3, StorageRead) — note the
    // kernel reads dims[0]=m, dims[1]=n, dims[2]=k.
    let dims: [u32; 3] = [m as u32, n as u32, k as u32];
    let view_dims =
        ctx.allocate_and_write(bytemuck::cast_slice(&dims), 3, 0, BindingUsage::StorageRead)?;

    let buffers = vec![view_a, view_b, view_c, view_dims];
    let pipeline = WgpuPipeline::compile(&ctx, GEMM_DF64_WGSL, GEMM_DF64_ENTRY)?;
    // @workgroup_size(64); one invocation per output element. The Schedule's
    // dispatch_workgroups computes ceil(element_count / 64) workgroups.
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    pipeline.dispatch(&buffers, &schedule, element_count)?;
    let packed = ctx.read_buffer_f32(&view_c)?; // 2*M*N f32
    Ok(unpack_df32(&packed))
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

/// All-pairs squared Euclidean distance `D[i][j] = ‖a_i − b_j‖²` between the rows of
/// `a` (`n×p`, row-major) and `b` (`m×p`, row-major), returned row-major `n×m`.
///
/// This is the kernel under **k-means assignment**, the **GMM E-step**, and the
/// **RBF-kernel Gram matrix** — the dominant cost when `n·m·p` is large. It is computed
/// with the best path on this machine via the identity
///
/// ```text
/// ‖a_i − b_j‖² = ‖a_i‖² + ‖b_j‖² − 2·(a_i · b_j)
/// ```
///
/// where the cross-term Gram matrix `a · bᵀ` (`n×m`) is the dense product `A·Bᵀ` routed
/// through [`gemm_f64`], so it inherits that function's CUDA-f64 / CPU-floor best-path
/// selection and the [`GEMM_GPU_THRESHOLD`] crossover automatically. The per-row norms
/// and the final combine are a linear-time CPU pass. Float cancellation can make a
/// near-zero entry slightly negative; such entries are clamped to `0.0`.
///
/// Because it uses the `‖·‖²` identity (not a direct `Σ(a−b)²` loop), entries whose true
/// distance is tiny *relative to the operands' norms* carry the usual catastrophic-
/// cancellation error of that identity — fine for argmin-style clustering/kernels, which
/// is what every caller does. [`pairwise_sq_dist_cpu_f64`] is the exact direct reference.
///
/// On any shape mismatch (`p == 0`, or a slice length that disagrees with `n`/`m`/`p`),
/// or if the GEMM cross-term errors, it returns the exact CPU floor instead of failing.
pub fn pairwise_sq_dist_f64(a: &[f64], b: &[f64], n: usize, m: usize, p: usize) -> Vec<f64> {
    if p == 0 || a.len() != n * p || b.len() != m * p {
        return pairwise_sq_dist_cpu_f64(a, b, n, m, p);
    }

    // Row norms ‖a_i‖² and ‖b_j‖².
    let mut norm_a = vec![0.0_f64; n];
    for (i, na) in norm_a.iter_mut().enumerate() {
        let row = &a[i * p..i * p + p];
        *na = row.iter().map(|&v| v * v).sum();
    }
    let mut norm_b = vec![0.0_f64; m];
    for (j, nb) in norm_b.iter_mut().enumerate() {
        let row = &b[j * p..j * p + p];
        *nb = row.iter().map(|&v| v * v).sum();
    }

    // Cross term A·Bᵀ = [n×m]. Materialise Bᵀ (p×m, row-major) and route the dense
    // product through the best-path GEMM: gemm_f64(m=n, k=p, n=m) computes A[n×p]·Bᵀ[p×m].
    let mut bt = vec![0.0_f64; p * m];
    for j in 0..m {
        for d in 0..p {
            bt[d * m + j] = b[j * p + d];
        }
    }
    let cross = match gemm_f64(n, p, m, a, &bt) {
        Ok(c) => c,
        Err(_) => return pairwise_sq_dist_cpu_f64(a, b, n, m, p),
    };

    let mut out = vec![0.0_f64; n * m];
    for i in 0..n {
        for j in 0..m {
            let d = norm_a[i] + norm_b[j] - 2.0 * cross[i * m + j];
            out[i * m + j] = if d > 0.0 { d } else { 0.0 };
        }
    }
    out
}

/// Exact direct reference for [`pairwise_sq_dist_f64`]: `D[i][j] = Σ_d (a[i][d] − b[j][d])²`
/// computed without the `‖·‖²` identity, so there is no cancellation. Always on the CPU,
/// always correct; this is the always-present floor and the differential oracle for the
/// accelerated form. Returns a zero-filled `n×m` for any shape mismatch.
pub fn pairwise_sq_dist_cpu_f64(a: &[f64], b: &[f64], n: usize, m: usize, p: usize) -> Vec<f64> {
    let mut out = vec![0.0_f64; n * m];
    if a.len() != n * p || b.len() != m * p {
        return out;
    }
    for i in 0..n {
        for j in 0..m {
            let mut s = 0.0_f64;
            for d in 0..p {
                let diff = a[i * p + d] - b[j * p + d];
                s += diff * diff;
            }
            out[i * m + j] = s;
        }
    }
    out
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
pub fn gemv_f32(m: usize, n: usize, a: &[f32], x: &[f32]) -> Result<Vec<f32>, ForgeError> {
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
pub fn gemv_f64(m: usize, n: usize, a: &[f64], x: &[f64]) -> Result<Vec<f64>, ForgeError> {
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
fn gemv_f64_cuda(m: usize, n: usize, a: &[f64], x: &[f64]) -> Result<Vec<f64>, ForgeError> {
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

/// Best-path forward FFT: `out = DFT(in)` over `n = complex_interleaved.len()/2`
/// complex points, input and output interleaved f32 (`[re0, im0, re1, im1, …]`,
/// length `2*n`). The transform is **un-normalized** and uses the forward sign
/// convention `X[k] = Σ_j x[j] · e^{−2πi kj/N}`, identical on both paths.
///
/// # Why this differs from the GEMM/GEMV dispatch shape
///
/// Unlike [`gemm_f32`]/[`gemm_f64`], the FFT has **no CUDA/df64 arm** — the forge
/// only ships a *WGSL* radix-2 kernel today, so the accelerated path is
/// wgpu-only. There is therefore exactly one accelerator branch:
///
/// | path (in order)              | when                                            |
/// |------------------------------|-------------------------------------------------|
/// | WGSL forge ([`ForgeRuntime::fft`]) | `caps().wgpu` and `n` a power of two, `2 ≤ n ≤ 1024` |
/// | CPU floor ([`dft_cpu`])      | otherwise, or if the forge errors at runtime    |
///
/// The CPU floor is the naive O(N²) DFT [`dft_cpu`] — always present, never
/// broken. The forge kernel runs ONE workgroup of `n` threads, which is why `n`
/// must be a power of two and `≤ 1024` (the single-workgroup cap); inputs outside
/// that window fall straight to the CPU floor. On any forge build/dispatch error
/// the call falls through to the CPU floor rather than propagating (mirrors
/// [`gemm_f32`]).
///
/// `complex_interleaved.len()` must be even (it is `2*n`); an odd length is the
/// only hard error.
pub fn fft_f32(complex_interleaved: &[f32]) -> Result<Vec<f32>, ForgeError> {
    if complex_interleaved.len() % 2 != 0 {
        return Err(ForgeError::GpuValidation(format!(
            "fft input must be interleaved complex (even length = 2*n); got {}",
            complex_interleaved.len()
        )));
    }
    let n = complex_interleaved.len() / 2;

    // Accelerated path is WGSL-only and single-workgroup: power-of-two n in
    // [2, 1024]. (n == 1 is a trivial identity the CPU floor handles directly.)
    if caps().wgpu && n.is_power_of_two() && (2..=1024).contains(&n) {
        if let Some(out) = fft_f32_gpu(complex_interleaved) {
            return Ok(out);
        }
        // Forge path was eligible but failed at runtime — fall through to the CPU
        // floor rather than propagating, so the call is never broken.
    }

    Ok(dft_cpu(complex_interleaved, n))
}

/// Run the forward FFT through the shared [`ForgeRuntime`], returning `None` on
/// any runtime failure (runtime un-buildable now, or dispatch error) so the
/// caller can fall through to the CPU floor. Never propagates a GPU error.
/// Reuses the same process-wide [`forge_rt_cell`] as [`gemm_f32`]/[`gemv_f32`].
fn fft_f32_gpu(complex_interleaved: &[f32]) -> Option<Vec<f32>> {
    let cell = forge_rt_cell();
    let mut guard = cell.lock().ok()?;
    if guard.is_none() {
        match ForgeRuntime::new(64 * 1024 * 1024, None) {
            Ok(rt) => *guard = Some(rt),
            Err(_) => return None,
        }
    }
    let rt = guard.as_mut()?;
    rt.fft(complex_interleaved).ok()
}

/// Shared dimension/length validation for both GEMV entry points: `a` is `m*n`
/// (row-major) and `x` is `n`.
fn validate_gemv_dims(m: usize, n: usize, a_len: usize, x_len: usize) -> Result<(), ForgeError> {
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

    /// Non-GPU / non-16-multiple: the opt-in tensor-core `gemm_f32_tc` falls through to the
    /// exact plain f32 floor (the 2×3·3×2 case is neither a 16-multiple nor on an
    /// accelerator), so it returns the hand-checked [58, 64, 139, 154] — proving the
    /// tensor-core path never breaks a call that can't use it.
    #[test]
    fn gemm_f32_tc_falls_to_plain_floor() {
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0f32, 8.0, 9.0, 10.0, 11.0, 12.0];
        let out = gemm_f32_tc(2, 3, 2, &a, &b).expect("gemm_f32_tc");
        assert_eq!(out, vec![58.0, 64.0, 139.0, 154.0]);
    }

    /// The coopmat (WGSL tensor-core) probe is honest and memoised: it can only be `true`
    /// where the adapter actually advertises coopmat, it is `false` without one (or no GPU),
    /// and repeated calls agree. On wgpu 29.0.3 it is `false` even on a coopmat-capable
    /// adapter (the multiply returns zeros, #9741) — so this never wrongly enables the path.
    #[test]
    fn coopmat_usable_respects_caps_and_is_cached() {
        let first = coopmat_usable();
        if !caps().coopmat {
            assert!(
                !first,
                "coopmat_usable must be false without a coopmat-capable adapter"
            );
        }
        // usable ⇒ the adapter advertises coopmat (never the other way on 29.0.3).
        assert!(!first || caps().coopmat);
        // Memoised: a second call returns the same verdict.
        assert_eq!(first, coopmat_usable());
    }

    /// Wrong dims are a hard error on the coopmat GEMM (non-8-multiple / zero), so the
    /// executor/dispatcher never dispatches an ill-formed tile. Non-GPU safe (validates
    /// before touching the device).
    #[test]
    fn gemm_f32_tc_coopmat_rejects_non_8_multiples() {
        assert!(gemm_f32_tc_coopmat(8, 8, 12, &[0.0; 96], &[0.0; 96]).is_err());
        assert!(gemm_f32_tc_coopmat(0, 8, 8, &[], &[0.0; 64]).is_err());
    }

    /// **GPU certify (A2000)** — the tiled coopmat f32 GEMM matches the exact f32 CPU
    /// reference. **Dormant on wgpu 29.0.3**: the coopmat multiply returns zeros (#9741), so
    /// this is `#[ignore]` until a wgpu release / the soft-fork carries the fix — at which
    /// point [`coopmat_usable`] flips `true` and this asserts the real tensor-core result.
    /// 16×16×16 = a 2×2 grid of 8×8 output tiles, 2 K-tiles each (so it exercises the loop +
    /// `workgroup_id` tiling, not just a single tile).
    #[test]
    #[ignore = "coopmat multiply dormant on wgpu 29.0.3 (#9741); lights up via wgpu release / soft-fork"]
    fn gemm_f32_tc_coopmat_matches_cpu_reference() {
        let (m, k, n) = (16usize, 16usize, 16usize);
        let a: Vec<f32> = (0..m * k).map(|i| ((i % 7) as f32) * 0.5 - 1.0).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i % 5) as f32) * 0.25 + 0.1).collect();
        let got = gemm_f32_tc_coopmat(m, k, n, &a, &b).expect("coopmat gemm");
        let want = crate::wgsl_forge::oracle::gemm_cpu(&a, &b, m, k, n);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            assert!(
                (g - w).abs() <= 1.0e-3 + 1.0e-3 * w.abs(),
                "coopmat {g} vs cpu {w}"
            );
        }
        // Non-zero sanity — the #9741 no-op returns all-zeros, which this would catch.
        assert!(got.iter().any(|&v| v.abs() > 1.0e-6));
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
        assert_eq!(
            gemm_cpu_f64(&a, &b, 2, 3, 2),
            vec![58.0, 64.0, 139.0, 154.0]
        );
    }

    /// Non-GPU: the df64 (double-single) host pack/unpack helpers round-trip a
    /// handful of `f64` values to ~1e-15. `f64_to_df32` splits a double into a hi/lo
    /// `f32` pair carrying ~44–48 mantissa bits; `df32_to_f64` recombines them. The
    /// residual is the `f32` rounding of `hi` refined by `lo`, far tighter than a
    /// single `f32` (~1e-7) — this pins the host side of the df64 path independently
    /// of any GPU.
    #[test]
    fn df64_pack_roundtrips() {
        let values = [
            0.0f64,
            1.0,
            -1.0,
            0.1,
            std::f64::consts::PI,
            -std::f64::consts::E,
            123.456_789,
            1.0 / 3.0,
        ];
        for &v in &values {
            let [hi, lo] = f64_to_df32(v);
            let back = df32_to_f64(hi, lo);
            assert!(
                (back - v).abs() <= 1.0e-15 * (1.0 + v.abs()),
                "df64 roundtrip {v} -> {back} (hi={hi}, lo={lo})"
            );
        }
        // And the flat-vector pack/unpack is the elementwise round-trip.
        let packed = pack_df32(&values);
        assert_eq!(packed.len(), values.len() * 2);
        let unpacked = unpack_df32(&packed);
        assert_eq!(unpacked.len(), values.len());
        for (u, v) in unpacked.iter().zip(values.iter()) {
            assert!((u - v).abs() <= 1.0e-15 * (1.0 + v.abs()), "{u} vs {v}");
        }
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

    /// `pairwise_sq_dist_cpu_f64` — the exact direct reference — on a hand-checked
    /// 2-point × 2-point, 2-D case. a=[[0,0],[1,1]], b=[[1,0],[0,1]]:
    /// ‖a0−b0‖²=1, ‖a0−b1‖²=1, ‖a1−b0‖²=1, ‖a1−b1‖²=1.
    #[test]
    fn pairwise_cpu_matches_hand_checked() {
        let a = [0.0f64, 0.0, 1.0, 1.0];
        let b = [1.0f64, 0.0, 0.0, 1.0];
        let d = pairwise_sq_dist_cpu_f64(&a, &b, 2, 2, 2);
        assert_eq!(d, vec![1.0, 1.0, 1.0, 1.0]);
        // A point's distance to itself is exactly 0.
        let self_d = pairwise_sq_dist_cpu_f64(&a, &a, 2, 2, 2);
        assert_eq!(self_d[0], 0.0);
        assert_eq!(self_d[3], 0.0);
    }

    /// The best-path `pairwise_sq_dist_f64` (GEMM-identity form) must agree with the
    /// exact direct CPU reference within f64 tolerance. Sub-threshold here, so the
    /// cross-term GEMM takes its own CPU floor — exercising the identity arithmetic and
    /// the norm/combine pass on a GPU-less box. Deterministic data, no RNG.
    #[test]
    fn pairwise_identity_matches_direct_reference() {
        let (n, m, p) = (5usize, 4usize, 3usize);
        let mut a = vec![0.0f64; n * p];
        for i in 0..n {
            for d in 0..p {
                a[i * p + d] = ((i * 3 + d * 2) % 7) as f64 * 0.5 - 1.0;
            }
        }
        let mut b = vec![0.0f64; m * p];
        for j in 0..m {
            for d in 0..p {
                b[j * p + d] = ((j * 5 + d) % 6) as f64 * 0.25 - 0.5;
            }
        }
        let identity = pairwise_sq_dist_f64(&a, &b, n, m, p);
        let direct = pairwise_sq_dist_cpu_f64(&a, &b, n, m, p);
        assert_eq!(identity.len(), direct.len());
        for (id, dr) in identity.iter().zip(direct.iter()) {
            assert!((id - dr).abs() < 1e-9, "pairwise mismatch: {id} vs {dr}");
        }
        // All squared distances are non-negative (clamp holds).
        assert!(identity.iter().all(|&v| v >= 0.0));
    }

    /// Shape mismatch is not a panic — it falls to the zero-filled CPU floor.
    #[test]
    fn pairwise_shape_mismatch_is_graceful() {
        // a has 5 elems but n*p = 2*3 = 6.
        let out = pairwise_sq_dist_f64(&[1.0; 5], &[1.0; 6], 2, 2, 3);
        assert_eq!(out.len(), 4);
    }

    /// Either path: the forward FFT of a real unit impulse at index 0 (`x[0] = 1`,
    /// rest 0) has a flat spectrum — every bin is exactly `(1, 0)` — because
    /// `X[k] = Σ_j x[j] e^{−2πi kj/N} = x[0] = 1` for all `k`. This identity is
    /// exact regardless of whether the WGSL forge or the CPU DFT floor runs, so it
    /// validates `fft_f32` on a GPU-less box (CPU floor) and a GPU box (forge)
    /// alike. N=4 (a power of two ≤ 1024, so the forge path is eligible when a
    /// wgpu adapter is present).
    #[test]
    fn fft_cpu_fallback_matches_dft() {
        let n = 4usize;
        let mut signal = vec![0.0f32; 2 * n]; // interleaved (real, imag)
        signal[0] = 1.0; // unit impulse at j=0
        let spectrum = fft_f32(&signal).expect("fft_f32");
        assert_eq!(spectrum.len(), 2 * n);
        for k in 0..n {
            assert!(
                (spectrum[2 * k] - 1.0).abs() < 1e-4,
                "bin {k} real should be 1, got {}",
                spectrum[2 * k]
            );
            assert!(
                spectrum[2 * k + 1].abs() < 1e-4,
                "bin {k} imag should be 0, got {}",
                spectrum[2 * k + 1]
            );
        }
    }

    /// An odd-length (not `2*n`) input is the only hard error on `fft_f32`.
    #[test]
    fn fft_f32_odd_length_errors() {
        assert!(fft_f32(&[1.0f32, 2.0, 3.0]).is_err());
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
            assert!((g - c).abs() <= 1.0e-3, "f32 GPU/CPU mismatch: {g} vs {c}");
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

    /// Forward FFT on the WGSL forge path must match the naive DFT floor within
    /// f32-vs-(f64-accumulated)-DFT tolerance. N=256 (a power-of-two single
    /// workgroup); both fed the SAME deterministic interleaved signal so the GPU
    /// FFT and the CPU `dft_cpu` reference compute the identical transform.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn fft_f32_gpu_matches_dft() {
        let n = 256usize;
        // 2*n interleaved (real, imag) samples, deterministic and identical for
        // both paths.
        let signal = det_f32(2 * n, 0x4646_545F_4D54_4348);
        let gpu = fft_f32(&signal).expect("fft_f32 gpu");
        let cpu = dft_cpu(&signal, n);
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert!((g - c).abs() <= 1.0e-2, "f32 FFT/DFT mismatch: {g} vs {c}");
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

    /// Tiled tensor-core (WMMA) GEMM on the CUDA backend: f16-input / f32-accumulate, so it
    /// is graded against an f32 matmul of the SAME inputs rounded through f16 first (the
    /// reduced-precision contract). 64×64×64 = a 4×4 grid of output tiles, each looping 4
    /// K-tiles — this exercises the tiling orchestration, not just a single tile.
    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn gemm_tc_cuda_tiled_matches_f16_reference() {
        let (m, k, n) = (64usize, 64, 64);
        // Small-magnitude data so f16 rounding error stays bounded over the K=64 sum.
        let a: Vec<f32> = det_f32(m * k, 0x574D_4D41_5449_4C45)
            .iter()
            .map(|&x| x * 0.5)
            .collect();
        let b: Vec<f32> = det_f32(k * n, 0x574D_4D41_5449_4C46)
            .iter()
            .map(|&x| x * 0.5)
            .collect();
        // Reference: f32 matmul of the f16-rounded inputs.
        let ar: Vec<f32> = a.iter().map(|&x| half::f16::from_f32(x).to_f32()).collect();
        let br: Vec<f32> = b.iter().map(|&x| half::f16::from_f32(x).to_f32()).collect();
        let expected = gemm_cpu(&ar, &br, m, k, n);
        let actual = gemm_tc_cuda(m, k, n, &a, &b).expect("gemm_tc_cuda");
        assert_eq!(actual.len(), expected.len());
        for (e, g) in expected.iter().zip(actual.iter()) {
            assert!(
                (e - g).abs() <= 5.0e-2 + 5.0e-2 * e.abs(),
                "WMMA tiled GEMM mismatch: cpu {e} vs gpu {g}"
            );
        }
        // Sanity: real output, not the all-zeros symptom of a broken tensor-core multiply.
        assert!(actual.iter().any(|&v| v.abs() > 1.0e-3));
    }

    /// df64 (double-single) emulated-f64 GEMM is correct only on adapters that do NOT
    /// reassociate f32 arithmetic (which would collapse the error-free transforms to
    /// f32 precision). `df64_usable()` probes this at runtime. This test verifies the
    /// probe is HONEST and the public `gemm_f64` is correct on every adapter:
    ///   - where the probe reports usable, the direct df64 GEMM is genuinely f64-precise;
    ///   - regardless, `gemm_f64` lands within f64 tolerance via the best working path
    ///     (df64 if usable, else native CUDA-f64, else the exact CPU floor).
    /// On the naga->SPIR-V->NVIDIA-Vulkan path here, the probe reports NOT usable (the
    /// driver reassociates floats), so df64 is correctly skipped and CUDA/CPU is used.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn df64_precision_is_probed_and_honest() {
        let (m, k, n) = (64usize, 64, 64);
        let a: Vec<f64> = det_f32(m * k, 0x6446_3634_4D4D_3401)
            .into_iter()
            .map(|x| x as f64)
            .collect();
        let b: Vec<f64> = det_f32(k * n, 0x6446_3634_4D4D_3402)
            .into_iter()
            .map(|x| x as f64)
            .collect();
        let cpu = gemm_cpu_f64(&a, &b, m, k, n);

        if df64_usable() {
            // The probe says this adapter preserves the error-free transforms — so the
            // direct df64 GEMM MUST be genuinely f64-precise.
            let df = gemm_f64_df64(m, k, n, &a, &b).expect("df64 gpu");
            for (d, c) in df.iter().zip(cpu.iter()) {
                assert!(
                    (d - c).abs() <= 1.0e-9,
                    "df64 reported usable but imprecise: {d} vs {c}"
                );
            }
        } else {
            eprintln!(
                "df64 not usable on this adapter (driver reassociates floats) — \
                 the f64 chain uses native CUDA or the exact CPU floor instead."
            );
        }

        // The PUBLIC f64 entry point must be correct on every adapter, via whichever
        // tier actually works (df64 / CUDA-f64 / CPU).
        let chain = gemm_f64(m, k, n, &a, &b).expect("gemm_f64");
        for (g, c) in chain.iter().zip(cpu.iter()) {
            assert!(
                (g - c).abs() <= 1.0e-9,
                "gemm_f64 chain incorrect: {g} vs {c}"
            );
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

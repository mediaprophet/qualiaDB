//! Double-single (`df64`) emulated double-precision GEMM in **raw WGSL**.
//!
//! WGSL has no `f64` — only `f32`/`f16`/`i32`/`u32`. The native exact-double GPU
//! path is therefore CUDA-only ([`crate::wgsl_forge::emit::cuda_c::GEMM_F64_SRC`]),
//! which covers NVIDIA. This module fills the *other* half of the "best f64 path on
//! every machine" story: on **any** wgpu-capable GPU (AMD, Intel, Apple, Adreno,
//! Mali, llvmpipe, …) it emulates each `f64` as a hi/lo pair of `f32` — a
//! *double-single* number carrying ~44–48 effective mantissa bits — and does the
//! GEMM accumulation with **error-free transforms** (Dekker `two_prod`, Knuth
//! `two_sum`, `quick_two_sum`). This is the df64/double-single technique.
//!
//! # Why this is a RAW WGSL string, not the portable IR
//!
//! The df64 arithmetic (the `two_sum`/`two_prod`/`df_add`/`df_mul` helpers operating
//! on `vec2<f32>` hi/lo pairs, with a Veltkamp-split Dekker `two_prod`) does not map onto the
//! forge's portable scalar-op IR. So — exactly like the cooperative-matrix kernel
//! ([`crate::wgsl_forge::oracle::evaluate_coopmat_loadstore`]) — it is shipped as a
//! hand-written WGSL source string, compiled via
//! [`crate::wgsl_forge::execute::WgpuPipeline::compile`] and dispatched directly,
//! rather than emitted from a [`crate::wgsl_forge::KernelSpec`].
//!
//! # Buffer ABI (matches the CUDA-f64 / forge GEMM binding layout)
//!
//! | binding | name   | usage              | layout                                   |
//! |---------|--------|--------------------|------------------------------------------|
//! | 0       | `a`    | storage, read      | `M*K` df64 = `2*M*K` f32, `[hi,lo,hi,lo…]`|
//! | 1       | `b`    | storage, read      | `K*N` df64 = `2*K*N` f32, `[hi,lo,…]`     |
//! | 2       | `c`    | storage, read_write| `M*N` df64 = `2*M*N` f32, `[hi,lo,…]`     |
//! | 3       | `dims` | storage, read      | `[m, n, k]` as `u32`                      |
//!
//! Row-major `C[M×N] = A[M×K] · B[K×N]`. One invocation computes one output element
//! (`@workgroup_size(64)`, `gid.x` over `m*n`). The inner `kk` accumulation order
//! matches the f64 CPU reference [`crate::wgsl_forge::dispatch::gemm_cpu_f64`], so the
//! two agree to df64 precision (~1e-12 for K≈64 O(1) data) **on adapters whose WGSL
//! float arithmetic is not reassociated by the driver**.
//!
//! # Correctness depends on per-op IEEE rounding (probed at runtime)
//!
//! Every df64 algorithm (Veltkamp split, Dekker `two_prod`, Knuth `two_sum`) relies on
//! each f32 `+`/`-`/`*` rounding exactly once, with no algebraic reassociation. Some GPU
//! shader toolchains break that: on the naga→SPIR-V→NVIDIA-Vulkan path on this hardware,
//! the compiler simplifies `c - (c - a)` to `a` (and an fma-based residual `fma(x,y,-(x*y))`
//! to `0`), which collapses the residual (lo) terms — df64 then silently degrades to f32
//! (~2e-7 error) instead of ~double precision. Switching `two_prod` from `fma` to the
//! Veltkamp split gave a **byte-identical** wrong result, confirming reassociation (not a
//! missing fma) as the cause; WGSL has no portable pragma to disable it. The
//! [`crate::wgsl_forge::dispatch::gemm_f64`] chain therefore **probes** this kernel at
//! runtime ([`df64_usable`](crate::wgsl_forge::dispatch)) and uses it only on adapters
//! where it actually delivers f64 precision; elsewhere it falls to native CUDA-f64 or the
//! exact CPU floor. The kernel below is correct on a faithful-IEEE adapter and is kept as
//! the portable f64-GPU path for those.

/// Entry-point name of [`GEMM_DF64_WGSL`].
pub const GEMM_DF64_ENTRY: &str = "gemm_df64";

/// Raw WGSL source for the df64 (double-single) emulated-f64 GEMM. See the module
/// docs for the binding ABI and precision contract. The error-free-transform helpers
/// are transcribed verbatim — `quick_two_sum`/`two_sum`/`two_prod` and the
/// `df_add`/`df_mul` pair-arithmetic are subtle and must not be "simplified".
pub const GEMM_DF64_WGSL: &str = r#"@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;
@group(0) @binding(3) var<storage, read> dims: array<u32>;
fn quick_two_sum(x: f32, y: f32) -> vec2<f32> { let s = x + y; let e = y - (s - x); return vec2<f32>(s, e); }
fn two_sum(x: f32, y: f32) -> vec2<f32> { let s = x + y; let v = s - x; let e = (x - (s - v)) + (y - v); return vec2<f32>(s, e); }
// Veltkamp split of an f32 into two 12-bit halves (factor 2^12+1 = 4097). Uses only
// IEEE +/-/* that naga preserves, so it does NOT depend on a fused fma (WGSL `fma`
// does not reliably lower to a true single-rounding FMA on the naga->SPIR-V->NVIDIA
// path, which silently collapses an fma-based two_prod residual to ~0 = f32 precision).
fn split(a: f32) -> vec2<f32> { let c = 4097.0 * a; let hi = c - (c - a); let lo = a - hi; return vec2<f32>(hi, lo); }
// Dekker TwoProduct: exact product as (p, e) with no fma. p = round(x*y); e = the
// rounding error reconstructed from the split partial products.
fn two_prod(x: f32, y: f32) -> vec2<f32> { let p = x * y; let xs = split(x); let ys = split(y); let e = ((xs.x * ys.x - p) + xs.x * ys.y + xs.y * ys.x) + xs.y * ys.y; return vec2<f32>(p, e); }
fn df_add(x: vec2<f32>, y: vec2<f32>) -> vec2<f32> { var s = two_sum(x.x, y.x); s.y = s.y + x.y + y.y; return quick_two_sum(s.x, s.y); }
fn df_mul(x: vec2<f32>, y: vec2<f32>) -> vec2<f32> { var p = two_prod(x.x, y.x); p.y = p.y + (x.x * y.y + x.y * y.x); return quick_two_sum(p.x, p.y); }
@compute @workgroup_size(64)
fn gemm_df64(@builtin(global_invocation_id) gid: vec3<u32>) {
    let m = dims[0]; let n = dims[1]; let k = dims[2];
    let o = gid.x;
    if (o >= m * n) { return; }
    let row = o / n; let col = o % n;
    var acc = vec2<f32>(0.0, 0.0);
    for (var kk: u32 = 0u; kk < k; kk = kk + 1u) {
        let ai = (row * k + kk) * 2u;
        let bi = (kk * n + col) * 2u;
        let av = vec2<f32>(a[ai], a[ai + 1u]);
        let bv = vec2<f32>(b[bi], b[bi + 1u]);
        acc = df_add(acc, df_mul(av, bv));
    }
    c[o * 2u] = acc.x;
    c[o * 2u + 1u] = acc.y;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    /// The df64 GEMM source must naga-validate with `gemm_df64` as its entry point.
    /// This pins that the `vec2<f32>` hi/lo helpers (`two_sum`/`two_prod` and the
    /// `df_add`/`df_mul` pair-arithmetic, including the `fma` error term) parse and
    /// type-check under naga — independently of any GPU adapter being present.
    #[test]
    fn gemm_df64_wgsl_validates() {
        let report = validate_wgsl(GEMM_DF64_WGSL).expect("df64 GEMM WGSL must naga-validate");
        assert!(
            report.entry_points.iter().any(|e| e == GEMM_DF64_ENTRY),
            "validated module must expose the {GEMM_DF64_ENTRY} entry point; got {:?}",
            report.entry_points
        );
    }
}

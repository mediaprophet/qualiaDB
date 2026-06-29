//! CUDA-C emission, compiled to PTX by NVRTC at runtime (mirrors the
//! HLSL -> DXC -> DXIL path). Storage buffers become pointer parameters in
//! binding order; a uniform block is passed by value as the last parameter.
//! This is what the native CUDA backend executes for the differential oracle.

use std::fmt::Write;

use super::GeneratedShader;
use crate::wgsl_forge::{ForgeError, KernelSpec, Schedule};

pub fn emit_cuda_c(kernel: &KernelSpec, schedule: Schedule) -> Result<GeneratedShader, ForgeError> {
    kernel.validate()?;
    let semantic_hash = kernel.semantic_hash()?;
    let mut source = String::with_capacity(2048);
    writeln!(source, "// CUDA-C emitted for {}@{}", kernel.id, kernel.semantic_version)
        .map_err(|e| ForgeError::Emission(e.to_string()))?;

    let wg = schedule.workgroup_size;
    match kernel.id.as_str() {
        "affine-f32" => emit_affine(&mut source)?,
        "fused-ffn" => emit_ffn(&mut source)?,
        "topk" => emit_topk(&mut source, wg)?,
        // gemm / gemv lower through the compute-graph IR: the KernelSpec becomes a one-node
        // ComputeGraph (`to_graph`) and `lower_graph` walks it into the CudaCLowerer — the
        // SAME graph the WGSL backend lowers, no per-id CUDA branch. (`fft` has no CUDA-C
        // lowering this phase, so it stays out of the route and errors via `other` below.)
        "gemm" | "gemv" => {
            let graph = kernel.to_graph()?;
            let mut lowerer = super::cuda_graph::CudaCLowerer { source: &mut source };
            crate::wgsl_forge::ir::graph::lower_graph(&graph, &mut lowerer)?;
        }
        other => {
            return Err(ForgeError::Emission(format!(
                "CUDA-C emission not implemented for kernel {other}"
            )))
        }
    }

    let source_hash = blake3::hash(source.as_bytes()).to_hex().to_string();
    Ok(GeneratedShader {
        kernel_id: kernel.id.clone(),
        semantic_hash,
        source_hash,
        schedule,
        source,
    })
}

/// CUDA-C WMMA (tensor-core) GEMM: `C(16x16,f32) = A(16x16,f16) * B(16x16,f16)`,
/// one warp (32 threads) per tile via the `nvcuda::wmma` fragment API. This is the
/// genuine reduced-precision tensor-core path — f16 A/B inputs with an f32
/// accumulator — which the wgpu/naga 29 cooperative-matrix backend cannot express
/// (29 only implements all-f32 8x8x8; mixed-precision MulAdd landed upstream after
/// 29). It is compiled by NVRTC with `--gpu-architecture=compute_XX` and
/// `--include-path=<toolkit>/include` (NVRTC's default header search list is empty,
/// so `<mma.h>` must be located explicitly — see `execute::cuda`).
///
/// Row-major fragment layouts + `ldm = 16` + a `mem_row_major` store reproduce a
/// standard host row-major reference `C[i][j] = sum_k A[i][k]*B[k][j]`, so it
/// verifies bit-approximately (f16 input precision) against
/// [`crate::wgsl_forge::oracle::matmul_cpu`].
pub const WMMA_GEMM_16X16_ENTRY: &str = "wmma_gemm_16x16";

/// Source for [`WMMA_GEMM_16X16_ENTRY`]. NVRTC-safe: the single `#include <mma.h>`
/// pulls in `cuda_fp16.h` transitively and adds no host-only headers.
pub const WMMA_GEMM_16X16_SRC: &str = r#"#include <mma.h>
using namespace nvcuda;

// Single-warp WMMA GEMM: C(16x16,f32) = A(16x16,f16) * B(16x16,f16).
// Launch with gridDim=(1,1,1), blockDim=(32,1,1). All 32 lanes must execute the
// fragment ops uniformly (no divergence) — WMMA is a warp-collective operation.
extern "C" __global__ void wmma_gemm_16x16(const __half *A,
                                           const __half *B,
                                           float *C) {
    wmma::fragment<wmma::matrix_a, 16, 16, 16, __half, wmma::row_major> a_frag;
    wmma::fragment<wmma::matrix_b, 16, 16, 16, __half, wmma::row_major> b_frag;
    wmma::fragment<wmma::accumulator, 16, 16, 16, float> c_frag;

    wmma::fill_fragment(c_frag, 0.0f);          // C starts at zero (D = A*B)
    wmma::load_matrix_sync(a_frag, A, 16);      // ldm = row stride = 16
    wmma::load_matrix_sync(b_frag, B, 16);
    wmma::mma_sync(c_frag, a_frag, b_frag, c_frag);
    wmma::store_matrix_sync(C, c_frag, 16, wmma::mem_row_major);
}"#;

/// Entry point of [`WMMA_GEMM_TILED_SRC`].
pub const WMMA_GEMM_TILED_ENTRY: &str = "wmma_gemm_tiled";

/// Source for [`WMMA_GEMM_TILED_ENTRY`] — a **tiled** WMMA GEMM that loops the proven
/// single-tile primitive over arbitrary `M/N/K` (each a multiple of 16): the real
/// tensor-core GEMM backend, not just one tile. `C[M×N]` f32 = `A[M×K]` f16 · `B[K×N]`
/// f16, row-major. One warp (a 32-thread block) computes one 16×16 output tile,
/// accumulating across the `K/16` inner tiles in registers (`c_frag`). Launch with
/// `workgroup_size = 32` and `element_count = (M/16)·(N/16)·32` so `gridDim.x` equals the
/// number of output tiles; `blockIdx.x` selects the tile. `dims = [M, N, K]` is a
/// 3-element `unsigned` **storage** buffer (binding 3), matching the pointer-only ABI of
/// [`crate::wgsl_forge::execute::CudaPipeline::compile_cuda_c_source`].
///
/// This is the **reduced-precision** path (f16 inputs, f32 accumulate) — it is opt-in via
/// `MatMul.tc`, since it trades f32 precision for tensor-core throughput; the plain f32
/// GEMM stays the default. `K = 16` reduces to the single-tile [`WMMA_GEMM_16X16_SRC`].
pub const WMMA_GEMM_TILED_SRC: &str = r#"#include <mma.h>
using namespace nvcuda;

// Tiled WMMA GEMM: C[M,N] f32 = A[M,K] f16 * B[K,N] f16, all row-major.
// M, N, K must be multiples of 16. One warp (block of 32) per 16x16 output tile.
extern "C" __global__ void wmma_gemm_tiled(const __half *A,
                                           const __half *B,
                                           float *C,
                                           const unsigned *dims) {
    unsigned M = dims[0];
    unsigned N = dims[1];
    unsigned K = dims[2];
    unsigned tiles_n = N / 16u;
    unsigned num_tiles = (M / 16u) * tiles_n;
    unsigned tile = blockIdx.x;
    if (tile >= num_tiles) {
        return;
    }
    unsigned tile_row = tile / tiles_n;   // 16-row block
    unsigned tile_col = tile % tiles_n;   // 16-col block

    wmma::fragment<wmma::matrix_a, 16, 16, 16, __half, wmma::row_major> a_frag;
    wmma::fragment<wmma::matrix_b, 16, 16, 16, __half, wmma::row_major> b_frag;
    wmma::fragment<wmma::accumulator, 16, 16, 16, float> c_frag;
    wmma::fill_fragment(c_frag, 0.0f);

    for (unsigned kt = 0u; kt < K; kt += 16u) {
        const __half *a_tile = A + (tile_row * 16u) * K + kt;   // ldm = K
        const __half *b_tile = B + kt * N + (tile_col * 16u);   // ldm = N
        wmma::load_matrix_sync(a_frag, a_tile, K);
        wmma::load_matrix_sync(b_frag, b_tile, N);
        wmma::mma_sync(c_frag, a_frag, b_frag, c_frag);
    }

    float *c_tile = C + (tile_row * 16u) * N + (tile_col * 16u);
    wmma::store_matrix_sync(c_tile, c_frag, N, wmma::mem_row_major);
}"#;

/// Native double-precision dense GEMM entry point. WGSL has no `f64` (only
/// f32/f16/i32/u32), so an exact-double GEMM has no WGSL/IR analogue; PTX/CUDA-C,
/// by contrast, has native `double` and `fma.rn.f64`, which is why the f64
/// best-path is CUDA, not WGSL (the dispatcher's `gemm_f64`).
pub const GEMM_F64_ENTRY: &str = "gemm_f64";

/// Source for [`GEMM_F64_ENTRY`]: row-major `C[M×N] = A[M×K] · B[K×N]`, all
/// `double`, one thread per output element. `dims` is a 3-element `unsigned`
/// **storage** buffer `[m, n, k]` (binding 3) — a storage buffer, not a by-value
/// uniform, so it rides the same pointer-only ABI
/// [`crate::wgsl_forge::execute::CudaPipeline::compile_cuda_c_source`] uses (no
/// `AffineParamsRaw` uniform handling needed). The inner `kk` sum order matches the
/// WGSL/f32 GEMM and the CPU reference so results agree to f64 summation precision.
pub const GEMM_F64_SRC: &str = r#"extern "C" __global__ void gemm_f64(const double* a,
                                    const double* b,
                                    double* c,
                                    const unsigned* dims) {
    unsigned m = dims[0];
    unsigned n = dims[1];
    unsigned k = dims[2];
    unsigned o = blockIdx.x * blockDim.x + threadIdx.x;
    if (o >= m * n) return;
    unsigned row = o / n;
    unsigned col = o % n;
    double acc = 0.0;
    for (unsigned kk = 0; kk < k; kk++) {
        acc += a[row * k + kk] * b[kk * n + col];
    }
    c[o] = acc;
}"#;

/// Native double-precision dense GEMV entry point. WGSL has no `f64` (only
/// f32/f16/i32/u32), so an exact-double GEMV has no WGSL/IR analogue; PTX/CUDA-C,
/// by contrast, has native `double` and `fma.rn.f64`, which is why the f64
/// best-path is CUDA, not WGSL (the dispatcher's `gemv_f64`).
pub const GEMV_F64_ENTRY: &str = "gemv_f64";

/// Source for [`GEMV_F64_ENTRY`]: row-major `y[M] = A[M×N] · x[N]`, all `double`,
/// one thread per output **row**. `dims` is a 2-element `unsigned` **storage** buffer
/// `[m, n]` (binding 3) — a storage buffer, not a by-value uniform, so it rides the
/// same pointer-only ABI
/// [`crate::wgsl_forge::execute::CudaPipeline::compile_cuda_c_source`] uses. The inner
/// `j` sum order matches the WGSL/f32 GEMV and the CPU reference so results agree to
/// f64 summation precision.
pub const GEMV_F64_SRC: &str = r#"extern "C" __global__ void gemv_f64(const double* a,
                                    const double* x,
                                    double* y,
                                    const unsigned* dims) {
    unsigned m = dims[0];
    unsigned n = dims[1];
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= m) return;
    double acc = 0.0;
    unsigned a_row = i * n;
    for (unsigned j = 0; j < n; j++) {
        acc += a[a_row + j] * x[j];
    }
    y[i] = acc;
}"#;

/// Single-precision dense GEMM entry point — the CUDA-C twin of the WGSL
/// [`emit_gemm_wgsl`](super::wgsl) kernel, for the `CudaCLowerer` plain (`tc=false`)
/// `MatMul` path. Lets the **same** compute-graph node lower to f32 on both backends so
/// the cross-backend differential oracle compares like with like (the WMMA path is the
/// reduced-precision `tc=true` alternative).
pub const GEMM_F32_ENTRY: &str = "gemm_f32";

/// Source for [`GEMM_F32_ENTRY`]: row-major `C[M×N] = A[M×K] · B[K×N]`, all `float`, one
/// thread per output element. `dims = [m, n, k]` is a 3-element `unsigned` **storage**
/// buffer (binding 3), matching the pointer-only ABI of
/// [`crate::wgsl_forge::execute::CudaPipeline::compile_cuda_c_source`]. The inner `kk` sum
/// order matches the WGSL f32 GEMM and the CPU reference [`crate::wgsl_forge::oracle::matmul_cpu`].
pub const GEMM_F32_SRC: &str = r#"extern "C" __global__ void gemm_f32(const float* a,
                                    const float* b,
                                    float* c,
                                    const unsigned* dims) {
    unsigned m = dims[0];
    unsigned n = dims[1];
    unsigned k = dims[2];
    unsigned o = blockIdx.x * blockDim.x + threadIdx.x;
    if (o >= m * n) return;
    unsigned row = o / n;
    unsigned col = o % n;
    float acc = 0.0f;
    for (unsigned kk = 0; kk < k; kk++) {
        acc += a[row * k + kk] * b[kk * n + col];
    }
    c[o] = acc;
}"#;

/// Single-precision dense GEMV entry point — the CUDA-C twin of the WGSL GEMV kernel, for
/// the `CudaCLowerer` `Gemv` node. `y[M] = A[M×N] · x[N]`, all `float`, one thread per row.
pub const GEMV_F32_ENTRY: &str = "gemv_f32";

/// Source for [`GEMV_F32_ENTRY`]: row-major `y[M] = A[M×N] · x[N]`, all `float`, one thread
/// per output **row**. `dims = [m, n]` is a 2-element `unsigned` **storage** buffer
/// (binding 3). The inner `j` sum order matches the WGSL f32 GEMV and the CPU reference.
pub const GEMV_F32_SRC: &str = r#"extern "C" __global__ void gemv_f32(const float* a,
                                    const float* x,
                                    float* y,
                                    const unsigned* dims) {
    unsigned m = dims[0];
    unsigned n = dims[1];
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= m) return;
    float acc = 0.0f;
    unsigned a_row = i * n;
    for (unsigned j = 0; j < n; j++) {
        acc += a[a_row + j] * x[j];
    }
    y[i] = acc;
}"#;

fn emit_affine(source: &mut String) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"struct AffineParams {{ unsigned length; float scale; float bias; unsigned _pad; }};
extern "C" __global__ void affine_f32(const float* input, float* output, AffineParams params) {{
    unsigned gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid < params.length) {{ output[gid] = input[gid] * params.scale + params.bias; }}
}}"#
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))
}

fn emit_ffn(source: &mut String) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"struct FfnParams {{ unsigned input_size; unsigned hidden_size; unsigned output_size; unsigned _pad; }};
extern "C" __global__ void fused_ffn(const float* input, const float* w1, const float* w2, float* output, FfnParams params) {{
    unsigned o = blockIdx.x * blockDim.x + threadIdx.x;
    if (o >= params.output_size) return;
    float acc = 0.0f;
    for (unsigned h = 0; h < params.hidden_size; h++) {{
        float hv = 0.0f;
        unsigned w1_row = h * params.input_size;
        for (unsigned i = 0; i < params.input_size; i++) hv += w1[w1_row + i] * input[i];
        float g = 0.5f * hv * (1.0f + tanhf(0.7978845608f * (hv + 0.044715f * hv * hv * hv)));
        acc += w2[o * params.hidden_size + h] * g;
    }}
    output[o] = acc;
}}"#
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))
}

fn emit_topk(source: &mut String, wg: u32) -> Result<(), ForgeError> {
    // Mirrors the WGSL top-k: one block per chunk, barrier-synchronised tree
    // arg-max over statically-sized shared memory.
    writeln!(
        source,
        r#"struct TopKParams {{ unsigned length; unsigned k; unsigned block_size; unsigned _pad; }};
extern "C" __global__ void topk(const float* input, float* output, TopKParams params) {{
    const unsigned WG = {wg}u;
    __shared__ float s_val[{wg}];
    __shared__ unsigned s_idx[{wg}];
    __shared__ float r_val[{wg}];
    __shared__ unsigned r_idx[{wg}];
    unsigned tid = threadIdx.x;
    unsigned block = blockIdx.x;
    unsigned gidx = block * WG + tid;
    float sentinel = __int_as_float(0xff7fffff);
    float v = sentinel;
    if (gidx < params.length) v = input[gidx];
    s_val[tid] = v;
    s_idx[tid] = tid;
    __syncthreads();
    for (unsigned i = 0; i < params.k; i++) {{
        r_val[tid] = s_val[tid];
        r_idx[tid] = s_idx[tid];
        __syncthreads();
        for (unsigned stride = WG / 2u; stride > 0u; stride /= 2u) {{
            if (tid < stride) {{
                if (r_val[tid + stride] > r_val[tid]) {{
                    r_val[tid] = r_val[tid + stride];
                    r_idx[tid] = r_idx[tid + stride];
                }}
            }}
            __syncthreads();
        }}
        if (tid == 0u) {{
            output[block * params.k + i] = r_val[0];
            s_val[r_idx[0]] = sentinel;
        }}
        __syncthreads();
    }}
}}"#
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))
}

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
    writeln!(
        source,
        "// CUDA-C emitted for {}@{}",
        kernel.id, kernel.semantic_version
    )
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
            let mut lowerer = super::cuda_graph::CudaCLowerer {
                source: &mut source,
            };
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

/// Entry point of [`Q4K_SOA_GEMV_SRC`] — on-device dequant · vector for Qualia SoA Q4_K.
pub const Q4K_SOA_GEMV_ENTRY: &str = "q4k_soa_gemv";

/// Device-side **Q4_K SoA multi-row coop dequant-GEMV**.
///
/// Each CUDA block owns `Q4K_SOA_GEMV_ROWS` consecutive output rows, loads the
/// activation **once** per K-superblock, and FMA-s into all live rows (3B lever:
/// gate/up n_out=8192 reloaded act 8192× under 1-row-per-block).
///
/// Layout: per 256-weight superblock = 160 B: qs[128] | d_sub f16[8] | m_sub f16[8].
/// Bindings: `x` f32[n_in], `W` uchar[n_out * row_bytes], `y` f32[n_out],
/// `dims` uint[3] = {n_in, n_out, row_bytes}.
///
/// Dispatch: `grid = ceil(n_out / Q4K_SOA_GEMV_ROWS)`, `block = 256`.
/// 16 rows/block amortizes act loads vs serial 1-row (3B FFN n_out=8192).
pub const Q4K_SOA_GEMV_ROWS: u32 = 16;

pub const Q4K_SOA_GEMV_SRC: &str = r#"
#define Q4K_ROWS 16u
__device__ __forceinline__ float q4k_f16_to_f32(unsigned short h) {
    unsigned sign = (h >> 15) & 1u;
    unsigned exp  = (h >> 10) & 0x1fu;
    unsigned mant = h & 0x3ffu;
    unsigned fbits;
    if (exp == 0) {
        if (mant == 0) {
            fbits = sign << 31;
        } else {
            exp = 127 - 15 + 1;
            while ((mant & 0x400u) == 0) { mant <<= 1; exp--; }
            mant &= 0x3ffu;
            fbits = (sign << 31) | (exp << 23) | (mant << 13);
        }
    } else if (exp == 31) {
        fbits = (sign << 31) | 0x7f800000u | (mant << 13);
    } else {
        fbits = (sign << 31) | ((exp + (127 - 15)) << 23) | (mant << 13);
    }
    return __int_as_float(fbits);
}

// Multi-row coop GEMV: blockIdx.x = row_group, threadIdx.x = 0..255 column partial.
// Parallel multi-accumulator reduce (one barrier ladder for all Q4K_ROWS).
extern "C" __global__ void q4k_soa_gemv(const float *x,
                                        const unsigned char *W,
                                        float *y,
                                        const unsigned *dims) {
    unsigned n_in = dims[0];
    unsigned n_out = dims[1];
    unsigned row_bytes = dims[2];
    unsigned row0 = blockIdx.x * Q4K_ROWS;
    unsigned t = threadIdx.x;
    if (row0 >= n_out) return;

    __shared__ float act[256];
    // 16 * 256 = 16 KiB — fits SM shared; parallel reduce needs this layout.
    __shared__ float red[Q4K_ROWS * 256u];
    float acc[Q4K_ROWS];
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) acc[r] = 0.0f;

    unsigned n_blocks = n_in / 256u;
    for (unsigned b = 0u; b < n_blocks; b++) {
        act[t] = x[b * 256u + t];
        __syncthreads();
        unsigned sub = t / 32u;
        unsigned group = t / 64u;
        unsigned local = t % 64u;
        unsigned d_off = 128u + sub * 2u;
        unsigned m_off = 144u + sub * 2u;
        unsigned q_off = group * 32u;
        float xv = act[t];
        #pragma unroll
        for (unsigned r = 0u; r < Q4K_ROWS; r++) {
            unsigned row = row0 + r;
            if (row >= n_out) continue;
            const unsigned char *blk =
                W + (size_t)row * (size_t)row_bytes + (size_t)b * 160u;
            unsigned short dh = (unsigned short)(blk[d_off] | (blk[d_off + 1u] << 8));
            unsigned short mh = (unsigned short)(blk[m_off] | (blk[m_off + 1u] << 8));
            float dsub = q4k_f16_to_f32(dh);
            float msub = q4k_f16_to_f32(mh);
            float nib = (local < 32u)
                ? (float)(blk[q_off + local] & 0xFu)
                : (float)(blk[q_off + (local - 32u)] >> 4);
            acc[r] += (dsub * nib - msub) * xv;
        }
        __syncthreads();
    }

    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) {
        red[r * 256u + t] = acc[r];
    }
    __syncthreads();
    for (unsigned stride = 128u; stride > 0u; stride >>= 1u) {
        if (t < stride) {
            #pragma unroll
            for (unsigned r = 0u; r < Q4K_ROWS; r++) {
                red[r * 256u + t] += red[r * 256u + t + stride];
            }
        }
        __syncthreads();
    }
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_out) y[row] = red[t * 256u];
    }
}
"#;

/// Fused Q/K/V projection from one activation (GQA-safe: n_q ≥ n_kv).
/// One shared act load; Q written for all rows, K/V only when `row < n_kv`.
///
/// Bindings: x, Wq, Wk, Wv, yq, yk, yv, dims={n_in, n_q, n_kv, row_bytes}.
/// Dispatch: `grid = ceil(n_q / Q4K_SOA_GEMV_ROWS)`, `block = 256`.
pub const Q4K_SOA_QKV_ENTRY: &str = "q4k_soa_qkv";
pub const Q4K_SOA_QKV_SRC: &str = r#"
#define Q4K_ROWS 16u
__device__ __forceinline__ float q4k_f16_to_f32_qkv(unsigned short h) {
    unsigned sign = (h >> 15) & 1u;
    unsigned exp  = (h >> 10) & 0x1fu;
    unsigned mant = h & 0x3ffu;
    unsigned fbits;
    if (exp == 0) {
        if (mant == 0) fbits = sign << 31;
        else {
            exp = 127 - 15 + 1;
            while ((mant & 0x400u) == 0) { mant <<= 1; exp--; }
            mant &= 0x3ffu;
            fbits = (sign << 31) | (exp << 23) | (mant << 13);
        }
    } else if (exp == 31) fbits = (sign << 31) | 0x7f800000u | (mant << 13);
    else fbits = (sign << 31) | ((exp + (127 - 15)) << 23) | (mant << 13);
    return __int_as_float(fbits);
}
__device__ __forceinline__ float q4k_row_partial(
    const unsigned char *W, unsigned row, unsigned row_bytes,
    unsigned b, unsigned t, float xv
) {
    const unsigned char *blk = W + (size_t)row * (size_t)row_bytes + (size_t)b * 160u;
    unsigned sub = t / 32u;
    unsigned group = t / 64u;
    unsigned local = t % 64u;
    unsigned d_off = 128u + sub * 2u;
    unsigned m_off = 144u + sub * 2u;
    unsigned short dh = (unsigned short)(blk[d_off] | (blk[d_off + 1u] << 8));
    unsigned short mh = (unsigned short)(blk[m_off] | (blk[m_off + 1u] << 8));
    float dsub = q4k_f16_to_f32_qkv(dh);
    float msub = q4k_f16_to_f32_qkv(mh);
    unsigned q_off = group * 32u;
    float nib = (local < 32u)
        ? (float)(blk[q_off + local] & 0xFu)
        : (float)(blk[q_off + (local - 32u)] >> 4);
    return (dsub * nib - msub) * xv;
}
__device__ __forceinline__ void q4k_reduce_write(
    float *red, unsigned t, unsigned row0, unsigned n_lim, float *y, float *acc
) {
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) red[r * 256u + t] = acc[r];
    __syncthreads();
    for (unsigned s = 128u; s > 0u; s >>= 1u) {
        if (t < s) {
            #pragma unroll
            for (unsigned r = 0u; r < Q4K_ROWS; r++)
                red[r * 256u + t] += red[r * 256u + t + s];
        }
        __syncthreads();
    }
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_lim) y[row] = red[t * 256u];
    }
    __syncthreads();
}
extern "C" __global__ void q4k_soa_qkv(
    const float *x,
    const unsigned char *Wq,
    const unsigned char *Wk,
    const unsigned char *Wv,
    float *yq,
    float *yk,
    float *yv,
    const unsigned *dims
) {
    unsigned n_in = dims[0];
    unsigned n_q = dims[1];
    unsigned n_kv = dims[2];
    unsigned row_bytes = dims[3];
    unsigned row0 = blockIdx.x * Q4K_ROWS;
    unsigned t = threadIdx.x;
    if (row0 >= n_q) return;

    __shared__ float act[256];
    __shared__ float red[Q4K_ROWS * 256u];
    float acc_q[Q4K_ROWS], acc_k[Q4K_ROWS], acc_v[Q4K_ROWS];
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) {
        acc_q[r] = 0.0f; acc_k[r] = 0.0f; acc_v[r] = 0.0f;
    }
    unsigned n_blocks = n_in / 256u;
    for (unsigned b = 0u; b < n_blocks; b++) {
        act[t] = x[b * 256u + t];
        __syncthreads();
        float xv = act[t];
        #pragma unroll
        for (unsigned r = 0u; r < Q4K_ROWS; r++) {
            unsigned row = row0 + r;
            if (row < n_q)
                acc_q[r] += q4k_row_partial(Wq, row, row_bytes, b, t, xv);
            if (row < n_kv) {
                acc_k[r] += q4k_row_partial(Wk, row, row_bytes, b, t, xv);
                acc_v[r] += q4k_row_partial(Wv, row, row_bytes, b, t, xv);
            }
        }
        __syncthreads();
    }
    q4k_reduce_write(red, t, row0, n_q, yq, acc_q);
    q4k_reduce_write(red, t, row0, n_kv, yk, acc_k);
    q4k_reduce_write(red, t, row0, n_kv, yv, acc_v);
}
"#;

/// Residual fused GEMV: `y[i] = residual[i] + W[i]·x`. Same multi-row geometry.
/// Bindings: x, W, y, dims, residual. Dispatch: ceil(n_out / Q4K_SOA_GEMV_ROWS) × 256.
pub const Q4K_SOA_GEMV_RESID_ENTRY: &str = "q4k_soa_gemv_resid";
pub const Q4K_SOA_GEMV_RESID_SRC: &str = r#"
#define Q4K_ROWS 16u
__device__ __forceinline__ float q4k_f16_to_f32_r(unsigned short h) {
    unsigned sign = (h >> 15) & 1u;
    unsigned exp  = (h >> 10) & 0x1fu;
    unsigned mant = h & 0x3ffu;
    unsigned fbits;
    if (exp == 0) {
        if (mant == 0) fbits = sign << 31;
        else {
            exp = 127 - 15 + 1;
            while ((mant & 0x400u) == 0) { mant <<= 1; exp--; }
            mant &= 0x3ffu;
            fbits = (sign << 31) | (exp << 23) | (mant << 13);
        }
    } else if (exp == 31) fbits = (sign << 31) | 0x7f800000u | (mant << 13);
    else fbits = (sign << 31) | ((exp + (127 - 15)) << 23) | (mant << 13);
    return __int_as_float(fbits);
}
extern "C" __global__ void q4k_soa_gemv_resid(const float *x,
                                              const unsigned char *W,
                                              float *y,
                                              const unsigned *dims,
                                              const float *residual) {
    unsigned n_in = dims[0];
    unsigned n_out = dims[1];
    unsigned row_bytes = dims[2];
    unsigned row0 = blockIdx.x * Q4K_ROWS;
    unsigned t = threadIdx.x;
    if (row0 >= n_out) return;
    __shared__ float act[256];
    __shared__ float red[Q4K_ROWS * 256u];
    float acc[Q4K_ROWS];
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) acc[r] = 0.0f;
    unsigned n_blocks = n_in / 256u;
    for (unsigned b = 0u; b < n_blocks; b++) {
        act[t] = x[b * 256u + t];
        __syncthreads();
        unsigned sub = t / 32u;
        unsigned group = t / 64u;
        unsigned local = t % 64u;
        unsigned d_off = 128u + sub * 2u;
        unsigned m_off = 144u + sub * 2u;
        unsigned q_off = group * 32u;
        float xv = act[t];
        #pragma unroll
        for (unsigned r = 0u; r < Q4K_ROWS; r++) {
            unsigned row = row0 + r;
            if (row >= n_out) continue;
            const unsigned char *blk =
                W + (size_t)row * (size_t)row_bytes + (size_t)b * 160u;
            unsigned short dh = (unsigned short)(blk[d_off] | (blk[d_off + 1u] << 8));
            unsigned short mh = (unsigned short)(blk[m_off] | (blk[m_off + 1u] << 8));
            float dsub = q4k_f16_to_f32_r(dh);
            float msub = q4k_f16_to_f32_r(mh);
            float nib = (local < 32u)
                ? (float)(blk[q_off + local] & 0xFu)
                : (float)(blk[q_off + (local - 32u)] >> 4);
            acc[r] += (dsub * nib - msub) * xv;
        }
        __syncthreads();
    }
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) red[r * 256u + t] = acc[r];
    __syncthreads();
    for (unsigned stride = 128u; stride > 0u; stride >>= 1u) {
        if (t < stride) {
            #pragma unroll
            for (unsigned r = 0u; r < Q4K_ROWS; r++)
                red[r * 256u + t] += red[r * 256u + t + stride];
        }
        __syncthreads();
    }
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_out) y[row] = residual[row] + red[t * 256u];
    }
}
"#;

/// Entry: fused SwiGLU expansion on two sticky Q4_K SoA weight matrices.
/// `silu(gate·x) * (up·x)` → `y[n_out]`. Bindings: x, W_gate, W_up, y, dims={n_in,n_out,row_bytes}.
pub const Q4K_SOA_FUSED_SWIGLU_ENTRY: &str = "q4k_soa_fused_swiglu";

/// Dual-weight coop fused FFN expansion (T-A2 slice): one block per output row, shared act,
/// both matrices dequant-FMA, silu·mul in registers. Weights stay in the multi-weight slab.
pub const Q4K_SOA_FUSED_SWIGLU_SRC: &str = r#"
__device__ __forceinline__ float q4k_f16_to_f32_sw(unsigned short h) {
    unsigned sign = (h >> 15) & 1u;
    unsigned exp  = (h >> 10) & 0x1fu;
    unsigned mant = h & 0x3ffu;
    unsigned fbits;
    if (exp == 0) {
        if (mant == 0) { fbits = sign << 31; }
        else {
            exp = 127 - 15 + 1;
            while ((mant & 0x400u) == 0) { mant <<= 1; exp--; }
            mant &= 0x3ffu;
            fbits = (sign << 31) | (exp << 23) | (mant << 13);
        }
    } else if (exp == 31) {
        fbits = (sign << 31) | 0x7f800000u | (mant << 13);
    } else {
        fbits = (sign << 31) | ((exp + (127 - 15)) << 23) | (mant << 13);
    }
    return __int_as_float(fbits);
}

extern "C" __global__ void q4k_soa_fused_swiglu(const float *x,
                                                const unsigned char *W_gate,
                                                const unsigned char *W_up,
                                                float *y,
                                                const unsigned *dims) {
    unsigned n_in = dims[0];
    unsigned n_out = dims[1];
    unsigned row_bytes = dims[2];
    unsigned row = blockIdx.x;
    unsigned t = threadIdx.x;
    if (row >= n_out) return;

    const unsigned char *g_row = W_gate + (size_t)row * (size_t)row_bytes;
    const unsigned char *u_row = W_up + (size_t)row * (size_t)row_bytes;
    __shared__ float partial_g[256];
    __shared__ float partial_u[256];
    __shared__ float act[256];
    float acc_g = 0.0f;
    float acc_u = 0.0f;
    unsigned n_blocks = n_in / 256u;
    for (unsigned b = 0u; b < n_blocks; b++) {
        act[t] = x[b * 256u + t];
        __syncthreads();
        const unsigned char *gb = g_row + b * 160u;
        const unsigned char *ub = u_row + b * 160u;
        unsigned sub = t / 32u;
        unsigned group = t / 64u;
        unsigned local = t % 64u;
        unsigned d_off = 128u + sub * 2u;
        unsigned m_off = 144u + sub * 2u;
        float gd = q4k_f16_to_f32_sw((unsigned short)(gb[d_off] | (gb[d_off + 1u] << 8)));
        float gm = q4k_f16_to_f32_sw((unsigned short)(gb[m_off] | (gb[m_off + 1u] << 8)));
        float ud = q4k_f16_to_f32_sw((unsigned short)(ub[d_off] | (ub[d_off + 1u] << 8)));
        float um = q4k_f16_to_f32_sw((unsigned short)(ub[m_off] | (ub[m_off + 1u] << 8)));
        unsigned q_off = group * 32u;
        float gnib, unib;
        if (local < 32u) {
            gnib = (float)(gb[q_off + local] & 0xFu);
            unib = (float)(ub[q_off + local] & 0xFu);
        } else {
            gnib = (float)(gb[q_off + (local - 32u)] >> 4);
            unib = (float)(ub[q_off + (local - 32u)] >> 4);
        }
        float ax = act[t];
        acc_g += (gd * gnib - gm) * ax;
        acc_u += (ud * unib - um) * ax;
        __syncthreads();
    }
    partial_g[t] = acc_g;
    partial_u[t] = acc_u;
    __syncthreads();
    for (unsigned stride = 128u; stride > 0u; stride >>= 1u) {
        if (t < stride) {
            partial_g[t] += partial_g[t + stride];
            partial_u[t] += partial_u[t + stride];
        }
        __syncthreads();
    }
    if (t == 0u) {
        float g = partial_g[0];
        float u = partial_u[0];
        // silu(g) = g / (1 + exp(-g))
        float sg = g / (1.0f + expf(-g));
        y[row] = sg * u;
    }
}
"#;

/// Interleaved RoPE (Llama / SmolLM GGUF): rotate adjacent pairs `(2i, 2i+1)`.
/// Bindings: `vec` f32[n_heads*head_dim], `params` u32[5] =
/// `{n_heads, head_dim, pos, base_bits, scale_bits}` (base/scale as f32 bit patterns).
/// Dispatch: `grid = ceil(n_heads * (head_dim/2) / 256)`, `block = 256`.
pub const ROPE_INTERLEAVED_ENTRY: &str = "rope_interleaved";
pub const ROPE_INTERLEAVED_SRC: &str = r#"
extern "C" __global__ void rope_interleaved(float *vec, const unsigned *params) {
    unsigned n_heads = params[0];
    unsigned head_dim = params[1];
    unsigned pos = params[2];
    float base = __int_as_float((int)params[3]);
    float scale = __int_as_float((int)params[4]);
    unsigned half = head_dim / 2u;
    if (half == 0u) return;
    if (!(scale > 0.0f) || !isfinite(scale)) scale = 1.0f;
    unsigned n_pairs = n_heads * half;
    unsigned gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= n_pairs) return;
    unsigned head = gid / half;
    unsigned i = gid % half;
    float scaled_pos = (float)pos / scale;
    float theta = scaled_pos * powf(base, -2.0f * (float)i / (float)head_dim);
    float s = sinf(theta);
    float c = cosf(theta);
    unsigned off = head * head_dim + 2u * i;
    float x0 = vec[off];
    float x1 = vec[off + 1u];
    vec[off] = x0 * c - x1 * s;
    vec[off + 1u] = x0 * s + x1 * c;
}
"#;

/// Write one token's K (or V) head stack into the permanent device KV cache.
/// Layout matches `KvCacheLayout::k_index` / `v_index` (f32, non-int8, non-dict):
/// `base = layer*layer_stride + slot*slot_kv_elems*2 + stream_off + kv_h*head_dim + d`
/// where `stream_off = 0` for K and `n_kv_head*head_dim` for V.
/// Bindings: `src` f32[n_kv*head_dim], `kv` f32[total], `params` u32[7] =
/// `{n_kv, head_dim, layer, slot, layer_stride, slot_kv_elems, is_v}`.
/// Dispatch: `grid = ceil(n_kv*head_dim / 256)`, `block = 256`.
pub const KV_SLOT_WRITE_ENTRY: &str = "kv_slot_write";
pub const KV_SLOT_WRITE_SRC: &str = r#"
extern "C" __global__ void kv_slot_write(
    const float *src,
    float *kv,
    const unsigned *params
) {
    unsigned n_kv = params[0];
    unsigned head_dim = params[1];
    unsigned layer = params[2];
    unsigned slot = params[3];
    unsigned layer_stride = params[4];
    unsigned slot_kv_elems = params[5];
    unsigned is_v = params[6];
    unsigned n = n_kv * head_dim;
    unsigned gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= n) return;
    unsigned kv_h = gid / head_dim;
    unsigned d = gid % head_dim;
    unsigned stream_off = is_v ? (n_kv * head_dim) : 0u;
    unsigned base = layer * layer_stride + slot * slot_kv_elems * 2u + stream_off;
    kv[base + kv_h * head_dim + d] = src[gid];
}
"#;

/// Single-token GQA causal SDPA over device KV (decode).
/// One block per Q head. Scores past positions `0..=pos` against the matching KV head.
/// Bindings: `q` f32[n_head*head_dim] (already RoPE'd), `kv` f32[total],
/// `out` f32[n_head*head_dim], `params` u32[9] =
/// `{n_head, n_kv, head_dim, layer, pos, max_context, layer_stride, slot_kv_elems, q_heads_per_kv}`,
/// `scale_bits` u32[1] = f32 scale bit pattern (`1/sqrt(head_dim)`).
/// Dispatch: `grid = n_head`, `block = 256` (coop-reduce dots along head_dim).
/// Caps: `head_dim ≤ 256`, `pos < 1024`, `max_context ≤ 1024` (engine MAX_CONTEXT_WINDOW).
pub const SDPA_DECODE_ENTRY: &str = "sdpa_decode_gqa";
pub const SDPA_DECODE_SRC: &str = r#"
extern "C" __global__ void sdpa_decode_gqa(
    const float *q,
    const float *kv,
    float *out,
    const unsigned *params,
    const unsigned *scale_bits
) {
    unsigned n_head = params[0];
    unsigned n_kv = params[1];
    unsigned head_dim = params[2];
    unsigned layer = params[3];
    unsigned pos = params[4];
    unsigned max_context = params[5];
    unsigned layer_stride = params[6];
    unsigned slot_kv_elems = params[7];
    unsigned q_per_kv = params[8];
    if (q_per_kv == 0u) q_per_kv = 1u;
    float scale = __int_as_float((int)scale_bits[0]);
    unsigned q_h = blockIdx.x;
    unsigned t = threadIdx.x;
    if (q_h >= n_head) return;
    unsigned kv_h = q_h / q_per_kv;
    if (kv_h >= n_kv) return;
    if (head_dim > 256u || pos >= 1024u || max_context == 0u || max_context > 1024u) return;

    __shared__ float q_sh[256];
    __shared__ float red[256];
    __shared__ float scores[1024];
    __shared__ float max_sh;
    __shared__ float sum_sh;

    if (t < head_dim) q_sh[t] = q[q_h * head_dim + t];
    __syncthreads();

    // Phase 1: scores[past] = (q · K[past]) * scale
    for (unsigned past = 0u; past <= pos; past++) {
        unsigned past_slot = past % max_context;
        unsigned k_base = layer * layer_stride
            + past_slot * slot_kv_elems * 2u
            + kv_h * head_dim;
        float partial = 0.0f;
        if (t < head_dim) partial = q_sh[t] * kv[k_base + t];
        red[t] = (t < head_dim) ? partial : 0.0f;
        __syncthreads();
        for (unsigned s = 128u; s > 0u; s >>= 1u) {
            if (t < s) red[t] += red[t + s];
            __syncthreads();
        }
        if (t == 0u) scores[past] = red[0] * scale;
        __syncthreads();
    }

    // Phase 2: max + softmax (thread 0; broadcast via shared)
    if (t == 0u) {
        float mx = scores[0];
        for (unsigned past = 1u; past <= pos; past++) {
            if (scores[past] > mx) mx = scores[past];
        }
        max_sh = mx;
        float sum = 0.0f;
        for (unsigned past = 0u; past <= pos; past++) {
            float e = expf(scores[past] - mx);
            scores[past] = e;
            sum += e;
        }
        if (sum == 0.0f) sum = 1.0f;
        for (unsigned past = 0u; past <= pos; past++) scores[past] /= sum;
        sum_sh = sum;
    }
    __syncthreads();
    (void)sum_sh;

    // Phase 3: out = sum_p softmax[p] * V[p]
    if (t < head_dim) {
        float acc = 0.0f;
        for (unsigned past = 0u; past <= pos; past++) {
            unsigned past_slot = past % max_context;
            unsigned v_base = layer * layer_stride
                + past_slot * slot_kv_elems * 2u
                + n_kv * head_dim
                + kv_h * head_dim;
            acc += kv[v_base + t] * scores[past];
        }
        out[q_h * head_dim + t] = acc;
    }
}
"#;

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

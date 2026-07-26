//! Two-stage Q8_0 × Q8 activation CUDA candidates.
//!
//! This follows llama.cpp's decode architecture: quantize the transient activation once, then
//! reuse it across one or more projection matrices. The production gate must account for both
//! the quantizer and dot kernel, including whether QKV/SwiGLU amortize quantization.

mod qkv_warp8;

pub(crate) use qkv_warp8::{
    source as q8_dp4a_qkv_rope_warp8_source, ENTRY as Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY,
};

pub(crate) const Q8_ACTIVATION_QUANT_ENTRY: &str = "quantize_q8_activation";
pub(crate) const Q8_0_DP4A_GEMV_ENTRY: &str = "q8_0_dp4a_gemv";
pub(crate) const Q8_0_DP4A_SWIGLU_ENTRY: &str = "q8_0_dp4a_swiglu";
pub(crate) const Q8_0_DP4A_QKV_ROPE_ENTRY: &str = "q8_0_dp4a_qkv_rope";
pub(crate) const Q8_0_DP4A_GEMV_RESID_ENTRY: &str = "q8_0_dp4a_gemv_resid";

pub(crate) const Q8_ACTIVATION_QUANT_SRC: &str = r#"
extern "C" __global__ void quantize_q8_activation(
    const float *x,
    signed char *quantized,
    float *scales,
    const unsigned *dims
) {
    const unsigned n_in = dims[0];
    const unsigned warp = threadIdx.x >> 5u;
    const unsigned lane = threadIdx.x & 31u;
    const unsigned block = blockIdx.x * 8u + warp;
    const unsigned n_blocks = n_in >> 5u;
    if (block >= n_blocks) return;

    const unsigned index = block * 32u + lane;
    const float value = x[index];
    float absmax = fabsf(value);
    for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
        absmax = fmaxf(absmax, __shfl_down_sync(0xffffffffu, absmax, delta));
    absmax = __shfl_sync(0xffffffffu, absmax, 0u);
    const float scale = absmax > 0.0f ? absmax * (1.0f / 127.0f) : 0.0f;
    const float inverse = absmax > 0.0f ? 127.0f / absmax : 0.0f;
    quantized[index] = (signed char)__float2int_rn(value * inverse);
    if (lane == 0u) scales[block] = scale;
}
"#;

pub(crate) fn q8_dp4a_swiglu_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            r#"
#include <cuda_fp16.h>
__device__ __forceinline__ float q8_scale(const unsigned char *block) {
    __half_raw raw;
    raw.x = (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    return __half2float((__half)raw);
}

__device__ __forceinline__ int load_i8x4_aligned2(const void *address) {
    const unsigned short *words = (const unsigned short *)address;
    return (int)words[0] | ((int)words[1] << 16);
}
extern "C" __global__ void q8_0_dp4a_swiglu(
    const signed char *x,
    const float *x_scales,
    const unsigned char *gate_weights,
    const unsigned char *up_weights,
    float *y,
    const unsigned *dims
) {
    const unsigned n_in = dims[0];
    const unsigned n_out = dims[1];
    const unsigned row_bytes = dims[2];
    const unsigned tid = threadIdx.x;
    const unsigned warp = tid >> 5u;
    const unsigned lane = tid & 31u;
    const unsigned row = blockIdx.x;
    float gate = 0.0f;
    float up = 0.0f;

    if (row < n_out) {
        const unsigned n_blocks = n_in >> 5u;
        const unsigned packed_index = 2u * (tid & 3u);
        for (unsigned block_index = tid >> 2u;
             block_index < n_blocks;
             block_index += 32u) {
            const unsigned char *gate_block =
                gate_weights + (size_t)row * row_bytes + block_index * 34u;
            const unsigned char *up_block =
                up_weights + (size_t)row * row_bytes + block_index * 34u;
            const signed char *activation_values = x + block_index * 32u;
            int gate_integer = 0;
            int up_integer = 0;
            #pragma unroll
            for (unsigned item = 0u; item < 2u; item++) {
                const unsigned index = packed_index + item;
                const int packed_x = ((const int *)activation_values)[index];
                gate_integer = __dp4a(
                    load_i8x4_aligned2(gate_block + 2u + index * 4u),
                    packed_x, gate_integer);
                up_integer = __dp4a(
                    load_i8x4_aligned2(up_block + 2u + index * 4u),
                    packed_x, up_integer);
            }
            const float activation_scale = x_scales[block_index];
            gate = fmaf(q8_scale(gate_block) * activation_scale,
                        (float)gate_integer, gate);
            up = fmaf(q8_scale(up_block) * activation_scale,
                      (float)up_integer, up);
        }
    }

    for (unsigned delta = 16u; delta > 0u; delta >>= 1u) {
        gate += __shfl_down_sync(0xffffffffu, gate, delta);
        up += __shfl_down_sync(0xffffffffu, up, delta);
    }
    __shared__ float gate_warp_sums[4];
    __shared__ float up_warp_sums[4];
    if (lane == 0u) {
        gate_warp_sums[warp] = gate;
        up_warp_sums[warp] = up;
    }
    __syncthreads();
    if (tid == 0u && row < n_out) {
        const float gate_sum = gate_warp_sums[0] + gate_warp_sums[1]
            + gate_warp_sums[2] + gate_warp_sums[3];
        const float up_sum = up_warp_sums[0] + up_warp_sums[1]
            + up_warp_sums[2] + up_warp_sums[3];
        y[row] = (gate_sum / (1.0f + expf(-gate_sum))) * up_sum;
    }
}
"#
            .to_string()
        })
        .as_str()
}

/// Q/K/V projection using a single pre-quantized activation and four warps per output row.
///
/// Adjacent rows share one block so the even/odd RoPE pair is available without a second
/// kernel or global-memory round trip.
pub(crate) fn q8_dp4a_qkv_rope_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            r#"
#include <cuda_fp16.h>
__device__ __forceinline__ float q8_scale(const unsigned char *block) {
    __half_raw raw;
    raw.x = (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    return __half2float((__half)raw);
}
__device__ __forceinline__ int load_i8x4_aligned2(const void *address) {
    const unsigned short *words = (const unsigned short *)address;
    return (int)words[0] | ((int)words[1] << 16);
}
extern "C" __global__ void q8_0_dp4a_qkv_rope(
    const signed char *x,
    const float *x_scales,
    const unsigned char *wq,
    const unsigned char *wk,
    const unsigned char *wv,
    float *yq,
    float *yk,
    float *yv,
    const unsigned *dims,
    const unsigned *rope_q,
    const unsigned *rope_k,
    const unsigned *step
) {
    const unsigned n_in = dims[0];
    const unsigned n_q = dims[1];
    const unsigned n_kv = dims[2];
    const unsigned row_bytes = dims[3];
    const unsigned tid = threadIdx.x;
    const unsigned row_in_pair = tid >> 7u;
    const unsigned local_tid = tid & 127u;
    const unsigned local_warp = local_tid >> 5u;
    const unsigned lane = local_tid & 31u;
    const unsigned row_base = blockIdx.x * 2u;
    const unsigned row = row_base + row_in_pair;
    float q = 0.0f;
    float k = 0.0f;
    float v = 0.0f;

    const unsigned n_blocks = n_in >> 5u;
    const unsigned packed_index = 2u * (local_tid & 3u);
    for (unsigned block_index = local_tid >> 2u;
         block_index < n_blocks;
         block_index += 32u) {
        const signed char *activation_values = x + block_index * 32u;
        const int packed_x0 = ((const int *)activation_values)[packed_index];
        const int packed_x1 = ((const int *)activation_values)[packed_index + 1u];
        const float activation_scale = x_scales[block_index];
        if (row < n_q) {
            const unsigned char *qb =
                wq + (size_t)row * row_bytes + block_index * 34u;
            int integer_sum = __dp4a(
                load_i8x4_aligned2(qb + 2u + packed_index * 4u),
                packed_x0, 0);
            integer_sum = __dp4a(
                load_i8x4_aligned2(qb + 2u + (packed_index + 1u) * 4u),
                packed_x1, integer_sum);
            q = fmaf(q8_scale(qb) * activation_scale, (float)integer_sum, q);
        }
        if (row < n_kv) {
            const unsigned char *kb =
                wk + (size_t)row * row_bytes + block_index * 34u;
            const unsigned char *vb =
                wv + (size_t)row * row_bytes + block_index * 34u;
            int k_integer = __dp4a(
                load_i8x4_aligned2(kb + 2u + packed_index * 4u),
                packed_x0, 0);
            k_integer = __dp4a(
                load_i8x4_aligned2(kb + 2u + (packed_index + 1u) * 4u),
                packed_x1, k_integer);
            int v_integer = __dp4a(
                load_i8x4_aligned2(vb + 2u + packed_index * 4u),
                packed_x0, 0);
            v_integer = __dp4a(
                load_i8x4_aligned2(vb + 2u + (packed_index + 1u) * 4u),
                packed_x1, v_integer);
            k = fmaf(q8_scale(kb) * activation_scale, (float)k_integer, k);
            v = fmaf(q8_scale(vb) * activation_scale, (float)v_integer, v);
        }
    }

    for (unsigned delta = 16u; delta > 0u; delta >>= 1u) {
        q += __shfl_down_sync(0xffffffffu, q, delta);
        k += __shfl_down_sync(0xffffffffu, k, delta);
        v += __shfl_down_sync(0xffffffffu, v, delta);
    }
    __shared__ float q_warp[2][4];
    __shared__ float k_warp[2][4];
    __shared__ float v_warp[2][4];
    if (lane == 0u) {
        q_warp[row_in_pair][local_warp] = q;
        k_warp[row_in_pair][local_warp] = k;
        v_warp[row_in_pair][local_warp] = v;
    }
    __syncthreads();

    if (tid == 0u && row_base + 1u < n_q) {
        const float even = q_warp[0][0] + q_warp[0][1] + q_warp[0][2] + q_warp[0][3];
        const float odd = q_warp[1][0] + q_warp[1][1] + q_warp[1][2] + q_warp[1][3];
        const unsigned head_dim = rope_q[1];
        const unsigned d = row_base % head_dim;
        const float base = __int_as_float((int)rope_q[3]);
        float scale = __int_as_float((int)rope_q[4]);
        if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale)
            * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float sine = sinf(theta);
        const float cosine = cosf(theta);
        yq[row_base] = even * cosine - odd * sine;
        yq[row_base + 1u] = even * sine + odd * cosine;
    }
    if (tid == 1u && row_base + 1u < n_kv) {
        const float even = k_warp[0][0] + k_warp[0][1] + k_warp[0][2] + k_warp[0][3];
        const float odd = k_warp[1][0] + k_warp[1][1] + k_warp[1][2] + k_warp[1][3];
        const unsigned head_dim = rope_k[1];
        const unsigned d = row_base % head_dim;
        const float base = __int_as_float((int)rope_k[3]);
        float scale = __int_as_float((int)rope_k[4]);
        if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale)
            * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float sine = sinf(theta);
        const float cosine = cosf(theta);
        yk[row_base] = even * cosine - odd * sine;
        yk[row_base + 1u] = even * sine + odd * cosine;
        yv[row_base] = v_warp[0][0] + v_warp[0][1] + v_warp[0][2] + v_warp[0][3];
        yv[row_base + 1u] =
            v_warp[1][0] + v_warp[1][1] + v_warp[1][2] + v_warp[1][3];
    }
}
"#
            .to_string()
        })
        .as_str()
}

pub(crate) const Q8_0_DP4A_GEMV_RESID_SRC: &str = r#"
#include <cuda_fp16.h>
__device__ __forceinline__ float q8_scale(const unsigned char *block) {
    __half_raw raw;
    raw.x = (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    return __half2float((__half)raw);
}
__device__ __forceinline__ int load_i8x4_aligned2(const void *address) {
    const unsigned short *words = (const unsigned short *)address;
    return (int)words[0] | ((int)words[1] << 16);
}
extern "C" __global__ void q8_0_dp4a_gemv_resid(
    const signed char *x,
    const float *x_scales,
    const unsigned char *w,
    float *y,
    const unsigned *dims,
    const float *residual
) {
    const unsigned n_in = dims[0];
    const unsigned n_out = dims[1];
    const unsigned row_bytes = dims[2];
    const unsigned tid = threadIdx.x;
    const unsigned warp = tid >> 5u;
    const unsigned lane = tid & 31u;
    const unsigned row = blockIdx.x;
    float sum = 0.0f;
    if (row < n_out) {
        const unsigned n_blocks = n_in >> 5u;
        const unsigned packed_index = 2u * (tid & 3u);
        for (unsigned block_index = tid >> 2u;
             block_index < n_blocks;
             block_index += 32u) {
            const unsigned char *block =
                w + (size_t)row * row_bytes + block_index * 34u;
            const signed char *activation_values = x + block_index * 32u;
            int integer_sum = 0;
            #pragma unroll
            for (unsigned item = 0u; item < 2u; item++) {
                const unsigned index = packed_index + item;
                integer_sum = __dp4a(
                    load_i8x4_aligned2(block + 2u + index * 4u),
                    ((const int *)activation_values)[index],
                    integer_sum);
            }
            sum = fmaf(
                q8_scale(block) * x_scales[block_index],
                (float)integer_sum,
                sum);
        }
    }
    for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
        sum += __shfl_down_sync(0xffffffffu, sum, delta);
    __shared__ float warp_sums[4];
    if (lane == 0u) warp_sums[warp] = sum;
    __syncthreads();
    if (tid == 0u && row < n_out) {
        y[row] = residual[row]
            + warp_sums[0] + warp_sums[1] + warp_sums[2] + warp_sums[3];
    }
}
"#;

pub(crate) const Q8_0_DP4A_GEMV_SRC: &str = r#"
#include <cuda_fp16.h>
__device__ __forceinline__ float q8_scale(const unsigned char *block) {
    __half_raw raw;
    raw.x = (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    return __half2float((__half)raw);
}

__device__ __forceinline__ int load_i8x4_aligned2(const void *address) {
    const unsigned short *words = (const unsigned short *)address;
    return (int)words[0] | ((int)words[1] << 16);
}

extern "C" __global__ void q8_0_dp4a_gemv(
    const signed char *x,
    const float *x_scales,
    const unsigned char *w,
    float *y,
    const unsigned *dims
) {
    const unsigned n_in = dims[0];
    const unsigned n_out = dims[1];
    const unsigned row_bytes = dims[2];
    const unsigned tid = threadIdx.x;
    const unsigned warp = threadIdx.x >> 5u;
    const unsigned lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x;
    float sum = 0.0f;

    if (row < n_out) {
        const unsigned n_blocks = n_in >> 5u;
        const unsigned packed_index = 2u * (tid & 3u);
        for (unsigned block_index = tid >> 2u;
             block_index < n_blocks;
             block_index += 32u) {
            const unsigned char *block =
                w + (size_t)row * row_bytes + block_index * 34u;
            const unsigned char *weight_values = block + 2u;
            const signed char *activation_values = x + block_index * 32u;
            int integer_sum = 0;
            #pragma unroll
            for (unsigned item = 0u; item < 2u; item++) {
                const unsigned index = packed_index + item;
                const int packed_w =
                    load_i8x4_aligned2(weight_values + index * 4u);
                const int packed_x =
                    ((const int *)activation_values)[index];
                integer_sum = __dp4a(packed_w, packed_x, integer_sum);
            }
            sum = fmaf(q8_scale(block) * x_scales[block_index],
                       (float)integer_sum, sum);
        }
    }

    for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
        sum += __shfl_down_sync(0xffffffffu, sum, delta);
    __shared__ float warp_sums[4];
    if (lane == 0u) warp_sums[warp] = sum;
    __syncthreads();
    if (tid == 0u && row < n_out)
        y[row] = warp_sums[0] + warp_sums[1] + warp_sums[2] + warp_sums[3];
}
"#;

//! Eight-row DP4A QKV/RoPE schedule.
//!
//! One warp owns one output row. Each warp evaluates four Q8 blocks per loop by assigning
//! eight lanes to the eight packed `i8x4` values of each block.

pub(crate) const ENTRY: &str = "q8_0_dp4a_qkv_rope_warp8";

pub(crate) fn source() -> &'static str {
    r#"
#include <cuda_fp16.h>
#define Q8_ROWS 8u
__device__ __forceinline__ float q8_scale(const unsigned char *block) {
    __half_raw raw;
    raw.x = (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    return __half2float((__half)raw);
}
__device__ __forceinline__ int load_i8x4_aligned2(const void *address) {
    const unsigned short *words = (const unsigned short *)address;
    return (int)words[0] | ((int)words[1] << 16);
}
__device__ __forceinline__ float warp_sum(float value) {
    for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
        value += __shfl_down_sync(0xffffffffu, value, delta);
    return value;
}
extern "C" __global__ void q8_0_dp4a_qkv_rope_warp8(
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
    const unsigned warp = threadIdx.x >> 5u;
    const unsigned lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    const unsigned block_lane = lane >> 3u;
    const unsigned packed_index = lane & 7u;
    const unsigned n_blocks = n_in >> 5u;
    float q = 0.0f;
    float k = 0.0f;
    float v = 0.0f;

    for (unsigned block_base = 0u; block_base < n_blocks; block_base += 4u) {
        const unsigned block_index = block_base + block_lane;
        if (block_index < n_blocks) {
            const int packed_x =
                ((const int *)(x + block_index * 32u))[packed_index];
            const float activation_scale = x_scales[block_index];
            if (row < n_q) {
                const unsigned char *qb =
                    wq + (size_t)row * row_bytes + block_index * 34u;
                const int integer = __dp4a(
                    load_i8x4_aligned2(qb + 2u + packed_index * 4u),
                    packed_x, 0);
                q = fmaf(q8_scale(qb) * activation_scale, (float)integer, q);
            }
            if (row < n_kv) {
                const unsigned char *kb =
                    wk + (size_t)row * row_bytes + block_index * 34u;
                const unsigned char *vb =
                    wv + (size_t)row * row_bytes + block_index * 34u;
                const int k_integer = __dp4a(
                    load_i8x4_aligned2(kb + 2u + packed_index * 4u),
                    packed_x, 0);
                const int v_integer = __dp4a(
                    load_i8x4_aligned2(vb + 2u + packed_index * 4u),
                    packed_x, 0);
                k = fmaf(q8_scale(kb) * activation_scale, (float)k_integer, k);
                v = fmaf(q8_scale(vb) * activation_scale, (float)v_integer, v);
            }
        }
    }

    q = warp_sum(q);
    k = warp_sum(k);
    v = warp_sum(v);
    __shared__ float qrow[Q8_ROWS];
    __shared__ float krow[Q8_ROWS];
    __shared__ float vrow[Q8_ROWS];
    if (lane == 0u) {
        qrow[warp] = q;
        krow[warp] = k;
        vrow[warp] = v;
    }
    __syncthreads();
    if (threadIdx.x < Q8_ROWS && row < n_q) {
        const unsigned local = threadIdx.x;
        const unsigned absolute = blockIdx.x * Q8_ROWS + local;
        const unsigned head_dim = rope_q[1];
        const unsigned d = absolute % head_dim;
        const unsigned pair = local ^ 1u;
        const float base = __int_as_float((int)rope_q[3]);
        float scale = __int_as_float((int)rope_q[4]);
        if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale)
            * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float a = qrow[local];
        const float b = qrow[pair];
        const float sine = sinf(theta);
        const float cosine = cosf(theta);
        yq[absolute] = (d & 1u) ? b * sine + a * cosine : a * cosine - b * sine;
    }
    if (threadIdx.x < Q8_ROWS && row < n_kv) {
        const unsigned local = threadIdx.x;
        const unsigned absolute = blockIdx.x * Q8_ROWS + local;
        const unsigned head_dim = rope_k[1];
        const unsigned d = absolute % head_dim;
        const unsigned pair = local ^ 1u;
        const float base = __int_as_float((int)rope_k[3]);
        float scale = __int_as_float((int)rope_k[4]);
        if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale)
            * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float a = krow[local];
        const float b = krow[pair];
        const float sine = sinf(theta);
        const float cosine = cosf(theta);
        yk[absolute] = (d & 1u) ? b * sine + a * cosine : a * cosine - b * sine;
        yv[absolute] = vrow[local];
    }
}
"#
}

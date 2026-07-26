//! Fused Q8_0 kernels used by prepared whole-model decode.

pub(crate) const Q8_0_GEMV_RESID_ENTRY: &str = "q8_0_gemv_resid";
pub(crate) const Q8_0_RMSNORM_QKV_ROPE_ENTRY: &str = "q8_0_rmsnorm_qkv_rope";
pub(crate) const Q8_0_RMSNORM_SWIGLU_ENTRY: &str = "q8_0_rmsnorm_swiglu";
pub(crate) const Q8_0_QKV_ROPE_ENTRY: &str = "q8_0_qkv_rope";
pub(crate) const Q8_0_SWIGLU_ENTRY: &str = "q8_0_swiglu";

const Q8_PREAMBLE: &str = r#"
#include <cuda_fp16.h>
#define Q8_ROWS 8u
__device__ __forceinline__ float q8_scale(const unsigned char *block) {
    __half_raw raw;
    raw.x = (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    return __half2float((__half)raw);
}
__device__ __forceinline__ float warp_sum(float value) {
    for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
        value += __shfl_down_sync(0xffffffffu, value, delta);
    return value;
}
"#;

pub(crate) fn q8_gemv_resid_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            format!(
                r#"{Q8_PREAMBLE}
extern "C" __global__ void q8_0_gemv_resid(const float *x,
                                            const unsigned char *w,
                                            float *y,
                                            const unsigned *dims,
                                            const float *residual) {{
    const unsigned n_in = dims[0], n_out = dims[1], row_bytes = dims[2];
    const unsigned warp = threadIdx.x >> 5u, lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    float sum = 0.0f;
    for (unsigned col = 0u; col < n_in; col += 32u) {{
        const float activation = x[col + lane];
        if (row < n_out) {{
            const unsigned char *b = w + (size_t)row * row_bytes + (col >> 5u) * 34u;
            sum = fmaf(q8_scale(b) * (float)(signed char)b[2u + lane], activation, sum);
        }}
    }}
    sum = warp_sum(sum);
    if (lane == 0u && row < n_out) y[row] = residual[row] + sum;
}}
"#
            )
        })
        .as_str()
}

/// Fused Q/K/V projection and RoPE over an already-normalized activation.
///
/// RMSNorm is intentionally a separate graph node: the former fused kernel recomputed the same
/// norm once per eight output rows (120 times for SmolLM2 attention).
pub(crate) fn q8_qkv_rope_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| format!(
        r#"{Q8_PREAMBLE}
extern "C" __global__ void q8_0_qkv_rope(
    const float *x,
    const unsigned char *wq, const unsigned char *wk, const unsigned char *wv,
    float *yq, float *yk, float *yv,
    const unsigned *dims, const unsigned *rope_q, const unsigned *rope_k,
    const unsigned *step) {{
    const unsigned n_in = dims[0], n_q = dims[1], n_kv = dims[2], row_bytes = dims[3];
    const unsigned warp = threadIdx.x >> 5u, lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    float aq = 0.0f, ak = 0.0f, av = 0.0f;
    for (unsigned col = 0u; col < n_in; col += 32u) {{
        const float activation = x[col + lane];
        const unsigned bi = col >> 5u;
        if (row < n_q) {{
            const unsigned char *b = wq + (size_t)row * row_bytes + bi * 34u;
            aq = fmaf(q8_scale(b) * (float)(signed char)b[2u + lane], activation, aq);
        }}
        if (row < n_kv) {{
            const unsigned char *kb = wk + (size_t)row * row_bytes + bi * 34u;
            const unsigned char *vb = wv + (size_t)row * row_bytes + bi * 34u;
            ak = fmaf(q8_scale(kb) * (float)(signed char)kb[2u + lane], activation, ak);
            av = fmaf(q8_scale(vb) * (float)(signed char)vb[2u + lane], activation, av);
        }}
    }}
    aq = warp_sum(aq); ak = warp_sum(ak); av = warp_sum(av);
    __shared__ float qrow[Q8_ROWS], krow[Q8_ROWS], vrow[Q8_ROWS];
    if (lane == 0u) {{ qrow[warp] = aq; krow[warp] = ak; vrow[warp] = av; }}
    __syncthreads();
    if (threadIdx.x < Q8_ROWS && row < n_q) {{
        const unsigned local = threadIdx.x, absolute = blockIdx.x * Q8_ROWS + local;
        const unsigned head_dim = rope_q[1], d = absolute % head_dim, pair = local ^ 1u;
        const float base = __int_as_float((int)rope_q[3]);
        float scale = __int_as_float((int)rope_q[4]); if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale) * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float a = qrow[local], b = qrow[pair], s = sinf(theta), c = cosf(theta);
        yq[absolute] = (d & 1u) ? b * s + a * c : a * c - b * s;
    }}
    if (threadIdx.x < Q8_ROWS && row < n_kv) {{
        const unsigned local = threadIdx.x, absolute = blockIdx.x * Q8_ROWS + local;
        const unsigned head_dim = rope_k[1], d = absolute % head_dim, pair = local ^ 1u;
        const float base = __int_as_float((int)rope_k[3]);
        float scale = __int_as_float((int)rope_k[4]); if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale) * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float a = krow[local], b = krow[pair], s = sinf(theta), c = cosf(theta);
        yk[absolute] = (d & 1u) ? b * s + a * c : a * c - b * s;
        yv[absolute] = vrow[local];
    }}
}}
"#
    )).as_str()
}

/// Fused gate/up projection and SiLU over an already-normalized activation.
pub(crate) fn q8_swiglu_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            format!(
                r#"{Q8_PREAMBLE}
extern "C" __global__ void q8_0_swiglu(
    const float *x, const unsigned char *wg, const unsigned char *wu,
    float *y, const unsigned *dims) {{
    const unsigned n_in = dims[0], n_out = dims[1], row_bytes = dims[2];
    const unsigned warp = threadIdx.x >> 5u, lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    float gate = 0.0f, up = 0.0f;
    for (unsigned col = 0u; col < n_in; col += 32u) {{
        const float activation = x[col + lane];
        if (row < n_out) {{
            const unsigned bi = col >> 5u;
            const unsigned char *gb = wg + (size_t)row * row_bytes + bi * 34u;
            const unsigned char *ub = wu + (size_t)row * row_bytes + bi * 34u;
            gate = fmaf(q8_scale(gb) * (float)(signed char)gb[2u + lane], activation, gate);
            up = fmaf(q8_scale(ub) * (float)(signed char)ub[2u + lane], activation, up);
        }}
    }}
    gate = warp_sum(gate); up = warp_sum(up);
    if (lane == 0u && row < n_out) y[row] = (gate / (1.0f + expf(-gate))) * up;
}}
"#
            )
        })
        .as_str()
}

pub(crate) fn q8_rmsnorm_swiglu_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            format!(
                r#"{Q8_PREAMBLE}
extern "C" __global__ void q8_0_rmsnorm_swiglu(
    const float *x, const float *norm_weight,
    const unsigned char *wg, const unsigned char *wu,
    float *y, const unsigned *dims) {{
    const unsigned n_in = dims[0], n_out = dims[1], row_bytes = dims[2];
    const float eps = __int_as_float((int)dims[3]);
    const unsigned warp = threadIdx.x >> 5u, lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    __shared__ float reduce[256];
    __shared__ float inv_rms;
    float ss = 0.0f;
    for (unsigned i = threadIdx.x; i < n_in; i += 256u) ss = fmaf(x[i], x[i], ss);
    reduce[threadIdx.x] = ss;
    __syncthreads();
    for (unsigned stride = 128u; stride > 0u; stride >>= 1u) {{
        if (threadIdx.x < stride) reduce[threadIdx.x] += reduce[threadIdx.x + stride];
        __syncthreads();
    }}
    if (threadIdx.x == 0u) inv_rms = rsqrtf(reduce[0] / (float)n_in + eps);
    __syncthreads();
    __shared__ float tile[32];
    float gate = 0.0f, up = 0.0f;
    for (unsigned col = 0u; col < n_in; col += 32u) {{
        if (threadIdx.x < 32u)
            tile[lane] = x[col + lane] * inv_rms * norm_weight[col + lane];
        __syncthreads();
        if (row < n_out) {{
            const unsigned block_index = col >> 5u;
            const unsigned char *gb = wg + (size_t)row * row_bytes + block_index * 34u;
            const unsigned char *ub = wu + (size_t)row * row_bytes + block_index * 34u;
            gate = fmaf(q8_scale(gb) * (float)(signed char)gb[2u + lane], tile[lane], gate);
            up = fmaf(q8_scale(ub) * (float)(signed char)ub[2u + lane], tile[lane], up);
        }}
        __syncthreads();
    }}
    gate = warp_sum(gate);
    up = warp_sum(up);
    if (lane == 0u && row < n_out) y[row] = (gate / (1.0f + expf(-gate))) * up;
}}
"#
            )
        })
        .as_str()
}

pub(crate) fn q8_rmsnorm_qkv_rope_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| format!(
        r#"{Q8_PREAMBLE}
extern "C" __global__ void q8_0_rmsnorm_qkv_rope(
    const float *x, const float *norm_weight,
    const unsigned char *wq, const unsigned char *wk, const unsigned char *wv,
    float *yq, float *yk, float *yv,
    const unsigned *dims, const unsigned *rope_q, const unsigned *rope_k,
    const unsigned *step) {{
    const unsigned n_in = dims[0], n_q = dims[1], n_kv = dims[2], row_bytes = dims[3];
    const float eps = __int_as_float((int)dims[4]);
    const unsigned warp = threadIdx.x >> 5u, lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    __shared__ float reduce[256];
    __shared__ float inv_rms;
    float ss = 0.0f;
    for (unsigned i = threadIdx.x; i < n_in; i += 256u) ss = fmaf(x[i], x[i], ss);
    reduce[threadIdx.x] = ss;
    __syncthreads();
    for (unsigned stride = 128u; stride > 0u; stride >>= 1u) {{
        if (threadIdx.x < stride) reduce[threadIdx.x] += reduce[threadIdx.x + stride];
        __syncthreads();
    }}
    if (threadIdx.x == 0u) inv_rms = rsqrtf(reduce[0] / (float)n_in + eps);
    __syncthreads();
    __shared__ float tile[32];
    float aq = 0.0f, ak = 0.0f, av = 0.0f;
    for (unsigned col = 0u; col < n_in; col += 32u) {{
        if (threadIdx.x < 32u)
            tile[lane] = x[col + lane] * inv_rms * norm_weight[col + lane];
        __syncthreads();
        const unsigned bi = col >> 5u;
        if (row < n_q) {{
            const unsigned char *b = wq + (size_t)row * row_bytes + bi * 34u;
            aq = fmaf(q8_scale(b) * (float)(signed char)b[2u + lane], tile[lane], aq);
        }}
        if (row < n_kv) {{
            const unsigned char *kb = wk + (size_t)row * row_bytes + bi * 34u;
            const unsigned char *vb = wv + (size_t)row * row_bytes + bi * 34u;
            ak = fmaf(q8_scale(kb) * (float)(signed char)kb[2u + lane], tile[lane], ak);
            av = fmaf(q8_scale(vb) * (float)(signed char)vb[2u + lane], tile[lane], av);
        }}
        __syncthreads();
    }}
    aq = warp_sum(aq); ak = warp_sum(ak); av = warp_sum(av);
    __shared__ float qrow[Q8_ROWS], krow[Q8_ROWS], vrow[Q8_ROWS];
    if (lane == 0u) {{ qrow[warp] = aq; krow[warp] = ak; vrow[warp] = av; }}
    __syncthreads();
    if (threadIdx.x < Q8_ROWS && row < n_q) {{
        const unsigned local = threadIdx.x, absolute = blockIdx.x * Q8_ROWS + local;
        const unsigned head_dim = rope_q[1], d = absolute % head_dim, pair = local ^ 1u;
        const float base = __int_as_float((int)rope_q[3]);
        float scale = __int_as_float((int)rope_q[4]); if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale) * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float a = qrow[local], b = qrow[pair], s = sinf(theta), c = cosf(theta);
        yq[absolute] = (d & 1u) ? b * s + a * c : a * c - b * s;
    }}
    if (threadIdx.x < Q8_ROWS && row < n_kv) {{
        const unsigned local = threadIdx.x, absolute = blockIdx.x * Q8_ROWS + local;
        const unsigned head_dim = rope_k[1], d = absolute % head_dim, pair = local ^ 1u;
        const float base = __int_as_float((int)rope_k[3]);
        float scale = __int_as_float((int)rope_k[4]); if (!(scale > 0.0f)) scale = 1.0f;
        const float theta = ((float)step[0] / scale) * powf(base, -2.0f * (float)(d / 2u) / (float)head_dim);
        const float a = krow[local], b = krow[pair], s = sinf(theta), c = cosf(theta);
        yk[absolute] = (d & 1u) ? b * s + a * c : a * c - b * s;
        yv[absolute] = vrow[local];
    }}
}}
"#
    )).as_str()
}

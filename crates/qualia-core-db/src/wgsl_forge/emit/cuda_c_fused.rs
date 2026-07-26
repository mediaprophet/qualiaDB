//! Fused CUDA-C kernels for the mega-pass — combine multiple kernel launches
//! into one to reduce launch overhead and intermediate global memory traffic.
//!
//! # Current fusions
//!
//! - **QKV + RoPE**: Fuses the Q4K SoA QKV projection with RoPE rotation on
//!   Q and K outputs. Eliminates 2 kernel launches per layer (RoPE K + RoPE Q).
//!   V is written without RoPE (as before).
//! - **KV slot write (K+V)**: Fuses the two separate K and V slot-write dispatches
//!   into a single launch. Grid is doubled; first half writes K, second half writes V.
//!
//! # Binding ABI
//!
//! The fused QKV+RoPE kernel uses the same bindings as `Q4K_SOA_QKV_SRC` plus
//! extra RoPE parameter buffers:
//! - bindings 0-7: x, Wq, Wk, Wv, yq, yk, yv, dims (same as QKV)
//! - binding 8: rope_params_q (u32[5] = {n_head, head_dim, pos, base_bits, scale_bits})
//! - binding 9: rope_params_k (u32[5] = {n_kv, head_dim, pos, base_bits, scale_bits})

/// Entry point for the fused QKV+RoPE kernel.
pub const Q4K_SOA_QKV_ROPE_ENTRY: &str = "q4k_soa_qkv_rope";

/// Fused Q4K SoA QKV projection + RoPE rotation on Q and K.
///
/// Same multi-row coop dequant-GEMV as `Q4K_SOA_QKV_SRC`, but after the reduce
/// and before writing to global memory, applies RoPE to yq and yk in shared
/// memory. V is written without modification.
///
/// Bindings: x, Wq, Wk, Wv, yq, yk, yv, dims={n_in, n_q, n_kv, row_bytes},
/// rope_q={n_head, head_dim, pos, base_bits, scale_bits},
/// rope_k={n_kv, head_dim, pos, base_bits, scale_bits}.
/// Dispatch: `grid = ceil(n_q / Q4K_SOA_GEMV_ROWS)`, `block = 256`.
pub const Q4K_SOA_QKV_ROPE_SRC: &str = r#"
#define Q4K_ROWS 16u
__device__ __forceinline__ float q4k_f16_to_f32_fq(unsigned short h) {
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
__device__ __forceinline__ float q4k_row_partial_fq(
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
    float dsub = q4k_f16_to_f32_fq(dh);
    float msub = q4k_f16_to_f32_fq(mh);
    unsigned q_off = group * 32u;
    float nib = (local < 32u)
        ? (float)(blk[q_off + local] & 0xFu)
        : (float)(blk[q_off + (local - 32u)] >> 4);
    return (dsub * nib - msub) * xv;
}
__device__ __forceinline__ void q4k_reduce_write_fq(
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
/// Apply RoPE to Q4K_ROWS output values in shared memory, then write to global.
/// Each thread t < Q4K_ROWS has the reduced value for row (row0 + t) in red[t*256].
/// RoPE pairs: (row=head*head_dim+2*i, row+1=head*head_dim+2*i+1).
__device__ __forceinline__ void q4k_reduce_rope_write(
    float *red, unsigned t, unsigned row0, unsigned n_lim,
    float *y, float *acc,
    unsigned n_heads, unsigned head_dim, unsigned pos, float base, float scale
) {
    // Store reduced values to shared.
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) red[r * 256u + t] = acc[r];
    __syncthreads();

    // Tree reduction.
    for (unsigned s = 128u; s > 0u; s >>= 1u) {
        if (t < s) {
            #pragma unroll
            for (unsigned r = 0u; r < Q4K_ROWS; r++)
                red[r * 256u + t] += red[r * 256u + t + s];
        }
        __syncthreads();
    }

    // Apply RoPE and write. Threads t and t^1 form a RoPE pair when t is even.
    // We use a shared buffer for the rotated values.
    __shared__ float rope_buf[Q4K_ROWS];
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_lim) {
            unsigned head = row / head_dim;
            unsigned d = row % head_dim;
            unsigned half = head_dim / 2u;
            if (half > 0u && head < n_heads) {
                float val = red[t * 256u];
                unsigned i = d / 2u;
                // Determine pair thread index.
                unsigned pair_t;
                float pair_val;
                if (d % 2u == 0u) {
                    // Even: pair is t+1 (d+1).
                    pair_t = t + 1u;
                    pair_val = (pair_t < Q4K_ROWS && (row0 + pair_t) < n_lim)
                        ? red[pair_t * 256u] : 0.0f;
                } else {
                    // Odd: pair is t-1 (d-1).
                    pair_t = t - 1u;
                    pair_val = (pair_t < Q4K_ROWS)
                        ? red[pair_t * 256u] : 0.0f;
                }
                // Compute RoPE angle.
                float scaled_pos = (float)pos / scale;
                float theta = scaled_pos * powf(base, -2.0f * (float)i / (float)head_dim);
                float s_val = sinf(theta);
                float c_val = cosf(theta);
                float rotated;
                if (d % 2u == 0u) {
                    // x0 = val, x1 = pair_val
                    rotated = val * c_val - pair_val * s_val;
                } else {
                    // x0 = pair_val, x1 = val
                    rotated = pair_val * s_val + val * c_val;
                }
                rope_buf[t] = rotated;
            } else {
                rope_buf[t] = red[t * 256u];
            }
        }
    }
    __syncthreads();
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_lim) y[row] = rope_buf[t];
    }
    __syncthreads();
}
extern "C" __global__ void q4k_soa_qkv_rope(
    const float *x,
    const unsigned char *Wq,
    const unsigned char *Wk,
    const unsigned char *Wv,
    float *yq,
    float *yk,
    float *yv,
    const unsigned *dims,
    const unsigned *rope_q,
    const unsigned *rope_k
) {
    unsigned n_in = dims[0];
    unsigned n_q = dims[1];
    unsigned n_kv = dims[2];
    unsigned row_bytes = dims[3];
    unsigned row0 = blockIdx.x * Q4K_ROWS;
    unsigned t = threadIdx.x;
    if (row0 >= n_q) return;

    // RoPE params for Q.
    unsigned n_head_q = rope_q[0];
    unsigned head_dim = rope_q[1];
    unsigned pos = rope_q[2];
    float base = __int_as_float((int)rope_q[3]);
    float scale = __int_as_float((int)rope_q[4]);
    if (!(scale > 0.0f) || !isfinite(scale)) scale = 1.0f;

    // RoPE params for K (same head_dim, pos, base, scale; different n_heads).
    unsigned n_head_k = rope_k[0];

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
                acc_q[r] += q4k_row_partial_fq(Wq, row, row_bytes, b, t, xv);
            if (row < n_kv) {
                acc_k[r] += q4k_row_partial_fq(Wk, row, row_bytes, b, t, xv);
                acc_v[r] += q4k_row_partial_fq(Wv, row, row_bytes, b, t, xv);
            }
        }
        __syncthreads();
    }
    // Fused: reduce + RoPE + write for Q and K; plain reduce+write for V.
    q4k_reduce_rope_write(red, t, row0, n_q, yq, acc_q, n_head_q, head_dim, pos, base, scale);
    q4k_reduce_rope_write(red, t, row0, n_kv, yk, acc_k, n_head_k, head_dim, pos, base, scale);
    q4k_reduce_write_fq(red, t, row0, n_kv, yv, acc_v);
}
"#;

/// Entry point for the fused K+V slot write kernel.
pub const KV_SLOT_WRITE_BOTH_ENTRY: &str = "kv_slot_write_both";

/// Fused K+V slot write: writes one token's K and V head stacks into the permanent
/// device KV cache in a single kernel launch. Grid is doubled: first half writes K,
/// second half writes V.
///
/// Bindings: `src_k` f32[n_kv*head_dim], `src_v` f32[n_kv*head_dim], `kv` f32[total],
/// `params` u32[5] = `{n_kv, head_dim, block_size, blocks_per_layer, slot_kv_elems}`.
/// Dispatch: `grid = ceil(2*n_kv*head_dim / 256)`, `block = 256`.
pub const KV_SLOT_WRITE_BOTH_SRC: &str = r#"
extern "C" __global__ void kv_slot_write_both(
    const float *src_k,
    const float *src_v,
    float *kv,
    const unsigned *params,
    const unsigned *layer_id,
    const unsigned *step,
    const unsigned *block_table
) {
    unsigned n_kv = params[0];
    unsigned head_dim = params[1];
    unsigned block_size = params[2];
    unsigned blocks_per_layer = params[3];
    unsigned slot_kv_elems = params[4];
    unsigned layer = layer_id[0];
    unsigned slot = step[1];
    if (block_size == 0u || blocks_per_layer == 0u) return;
    unsigned logical_block = slot / block_size;
    if (logical_block >= blocks_per_layer) return;
    unsigned physical_block = block_table[layer * blocks_per_layer + logical_block];
    if (physical_block == 0xffffffffu) return;
    unsigned block_offset = slot % block_size;
    unsigned n = n_kv * head_dim;
    unsigned gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= n * 2u) return;
    unsigned is_v = gid / n;
    unsigned local = gid % n;
    unsigned kv_h = local / head_dim;
    unsigned d = local % head_dim;
    unsigned stream_off = is_v ? (n_kv * head_dim) : 0u;
    unsigned block_elems = block_size * slot_kv_elems * 2u;
    unsigned base = physical_block * block_elems
        + block_offset * slot_kv_elems * 2u + stream_off;
    float val = is_v ? src_v[local] : src_k[local];
    kv[base + kv_h * head_dim + d] = val;
}
"#;

/// Entry point for the fused RMSNorm+QKV+RoPE kernel.
pub const Q4K_SOA_RMSNORM_QKV_ROPE_ENTRY: &str = "q4k_soa_rmsnorm_qkv_rope";

/// Fused RMSNorm + Q4K SoA QKV projection + RoPE.
///
/// Each block redundantly computes the RMSNorm of the input hidden state (cheap for
/// n_embd ≤ 4096), applies normalization on-the-fly during input loading, then proceeds
/// with Q4K dequant-GEMV and RoPE as in `Q4K_SOA_QKV_ROPE_SRC`. Eliminates the separate
/// RMSNorm kernel launch and the global memory write of the normalized hidden state.
///
/// Bindings: x, norm_weight, Wq, Wk, Wv, yq, yk, yv,
/// dims={n_in, n_q, n_kv, row_bytes, eps_bits},
/// rope_q={n_head, head_dim, pos, base_bits, scale_bits},
/// rope_k={n_kv, head_dim, pos, base_bits, scale_bits}.
/// Dispatch: `grid = ceil(n_q / Q4K_SOA_GEMV_ROWS)`, `block = 256`.
pub const Q4K_SOA_RMSNORM_QKV_ROPE_SRC: &str = r#"
#define Q4K_ROWS 16u
__device__ __forceinline__ float q4k_f16_to_f32_rn(unsigned short h) {
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
__device__ __forceinline__ float q4k_row_partial_rn(
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
    float dsub = q4k_f16_to_f32_rn(dh);
    float msub = q4k_f16_to_f32_rn(mh);
    unsigned q_off = group * 32u;
    float nib = (local < 32u)
        ? (float)(blk[q_off + local] & 0xFu)
        : (float)(blk[q_off + (local - 32u)] >> 4);
    return (dsub * nib - msub) * xv;
}
__device__ __forceinline__ void q4k_reduce_write_rn(
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
__device__ __forceinline__ void q4k_reduce_rope_write_rn(
    float *red, unsigned t, unsigned row0, unsigned n_lim,
    float *y, float *acc,
    unsigned n_heads, unsigned head_dim, unsigned pos, float base, float scale
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
    __shared__ float rope_buf[Q4K_ROWS];
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_lim) {
            unsigned head = row / head_dim;
            unsigned d = row % head_dim;
            unsigned half = head_dim / 2u;
            if (half > 0u && head < n_heads) {
                float val = red[t * 256u];
                unsigned i = d / 2u;
                unsigned pair_t;
                float pair_val;
                if (d % 2u == 0u) {
                    pair_t = t + 1u;
                    pair_val = (pair_t < Q4K_ROWS && (row0 + pair_t) < n_lim)
                        ? red[pair_t * 256u] : 0.0f;
                } else {
                    pair_t = t - 1u;
                    pair_val = (pair_t < Q4K_ROWS)
                        ? red[pair_t * 256u] : 0.0f;
                }
                float scaled_pos = (float)pos / scale;
                float theta = scaled_pos * powf(base, -2.0f * (float)i / (float)head_dim);
                float s_val = sinf(theta);
                float c_val = cosf(theta);
                float rotated;
                if (d % 2u == 0u) {
                    rotated = val * c_val - pair_val * s_val;
                } else {
                    rotated = pair_val * s_val + val * c_val;
                }
                rope_buf[t] = rotated;
            } else {
                rope_buf[t] = red[t * 256u];
            }
        }
    }
    __syncthreads();
    if (t < Q4K_ROWS) {
        unsigned row = row0 + t;
        if (row < n_lim) y[row] = rope_buf[t];
    }
    __syncthreads();
}
extern "C" __global__ void q4k_soa_rmsnorm_qkv_rope(
    const float *x,
    const float *norm_weight,
    const unsigned char *Wq,
    const unsigned char *Wk,
    const unsigned char *Wv,
    float *yq,
    float *yk,
    float *yv,
    const unsigned *dims,
    const unsigned *rope_q,
    const unsigned *rope_k,
    const unsigned *step
) {
    unsigned n_in = dims[0];
    unsigned n_q = dims[1];
    unsigned n_kv = dims[2];
    unsigned row_bytes = dims[3];
    float eps = __int_as_float((int)dims[4]);
    unsigned row0 = blockIdx.x * Q4K_ROWS;
    unsigned t = threadIdx.x;
    if (row0 >= n_q) return;

    // RoPE params for Q.
    unsigned n_head_q = rope_q[0];
    unsigned head_dim = rope_q[1];
    unsigned pos = step[0];
    float base = __int_as_float((int)rope_q[3]);
    float scale = __int_as_float((int)rope_q[4]);
    if (!(scale > 0.0f) || !isfinite(scale)) scale = 1.0f;
    unsigned n_head_k = rope_k[0];

    // Phase 1: compute RMSNorm redundantly per block (cheap for n_in ≤ 4096).
    // Sum of squares across all n_in elements.
    __shared__ float rms_red[256];
    __shared__ float act_norm[256];
    float ss = 0.0f;
    for (unsigned i = t; i < n_in; i += 256u) {
        float v = x[i];
        ss += v * v;
    }
    rms_red[t] = ss;
    __syncthreads();
    for (unsigned s = 128u; s > 0u; s >>= 1u) {
        if (t < s) rms_red[t] += rms_red[t + s];
        __syncthreads();
    }
    float rms_inv = rsqrtf(rms_red[0] / (float)n_in + eps);

    // Phase 2: Q4K dequant-GEMV with on-the-fly normalization.
    __shared__ float red[Q4K_ROWS * 256u];
    float acc_q[Q4K_ROWS], acc_k[Q4K_ROWS], acc_v[Q4K_ROWS];
    #pragma unroll
    for (unsigned r = 0u; r < Q4K_ROWS; r++) {
        acc_q[r] = 0.0f; acc_k[r] = 0.0f; acc_v[r] = 0.0f;
    }
    unsigned n_blocks = n_in / 256u;
    for (unsigned b = 0u; b < n_blocks; b++) {
        // Load and normalize x chunk.
        unsigned xi = b * 256u + t;
        act_norm[t] = x[xi] * rms_inv * norm_weight[xi];
        __syncthreads();
        float xv = act_norm[t];
        #pragma unroll
        for (unsigned r = 0u; r < Q4K_ROWS; r++) {
            unsigned row = row0 + r;
            if (row < n_q)
                acc_q[r] += q4k_row_partial_rn(Wq, row, row_bytes, b, t, xv);
            if (row < n_kv) {
                acc_k[r] += q4k_row_partial_rn(Wk, row, row_bytes, b, t, xv);
                acc_v[r] += q4k_row_partial_rn(Wv, row, row_bytes, b, t, xv);
            }
        }
        __syncthreads();
    }
    // Phase 3: reduce + RoPE + write.
    q4k_reduce_rope_write_rn(red, t, row0, n_q, yq, acc_q, n_head_q, head_dim, pos, base, scale);
    q4k_reduce_rope_write_rn(red, t, row0, n_kv, yk, acc_k, n_head_k, head_dim, pos, base, scale);
    q4k_reduce_write_rn(red, t, row0, n_kv, yv, acc_v);
}
"#;

/// Entry point for the fused RMSNorm+SwiGLU kernel.
pub const Q4K_SOA_RMSNORM_SWIGLU_ENTRY: &str = "q4k_soa_rmsnorm_swiglu";

/// Fused RMSNorm + Q4K SoA SwiGLU expansion.
///
/// Each block redundantly computes RMSNorm of the input (cheap for n_embd ≤ 4096),
/// applies normalization on-the-fly during input loading, then proceeds with the
/// dual-weight dequant-GEMV + silu·mul as in `Q4K_SOA_FUSED_SWIGLU_SRC`.
/// Eliminates the separate FFN pre-norm kernel launch and global memory round-trip.
///
/// Bindings: x, norm_weight, W_gate, W_up, y,
/// dims={n_in, n_out, row_bytes, eps_bits}.
/// Dispatch: `grid = n_out`, `block = 256`.
pub const Q4K_SOA_RMSNORM_SWIGLU_SRC: &str = r#"
__device__ __forceinline__ float q4k_f16_to_f32_sw2(unsigned short h) {
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
extern "C" __global__ void q4k_soa_rmsnorm_swiglu(
    const float *x,
    const float *norm_weight,
    const unsigned char *W_gate,
    const unsigned char *W_up,
    float *y,
    const unsigned *dims
) {
    unsigned n_in = dims[0];
    unsigned n_out = dims[1];
    unsigned row_bytes = dims[2];
    float eps = __int_as_float((int)dims[3]);
    unsigned row = blockIdx.x;
    unsigned t = threadIdx.x;
    if (row >= n_out) return;

    // Phase 1: compute RMSNorm redundantly per block.
    __shared__ float rms_red[256];
    float ss = 0.0f;
    for (unsigned i = t; i < n_in; i += 256u) {
        float v = x[i];
        ss += v * v;
    }
    rms_red[t] = ss;
    __syncthreads();
    for (unsigned s = 128u; s > 0u; s >>= 1u) {
        if (t < s) rms_red[t] += rms_red[t + s];
        __syncthreads();
    }
    float rms_inv = rsqrtf(rms_red[0] / (float)n_in + eps);

    // Phase 2: dual-weight dequant-GEMV with on-the-fly normalization.
    const unsigned char *g_row = W_gate + (size_t)row * (size_t)row_bytes;
    const unsigned char *u_row = W_up + (size_t)row * (size_t)row_bytes;
    __shared__ float partial_g[256];
    __shared__ float partial_u[256];
    __shared__ float act_n[256];
    float acc_g = 0.0f;
    float acc_u = 0.0f;
    unsigned n_blocks = n_in / 256u;
    for (unsigned b = 0u; b < n_blocks; b++) {
        unsigned xi = b * 256u + t;
        act_n[t] = x[xi] * rms_inv * norm_weight[xi];
        __syncthreads();
        const unsigned char *gb = g_row + b * 160u;
        const unsigned char *ub = u_row + b * 160u;
        unsigned sub = t / 32u;
        unsigned group = t / 64u;
        unsigned local = t % 64u;
        unsigned d_off = 128u + sub * 2u;
        unsigned m_off = 144u + sub * 2u;
        float gd = q4k_f16_to_f32_sw2((unsigned short)(gb[d_off] | (gb[d_off + 1u] << 8)));
        float gm = q4k_f16_to_f32_sw2((unsigned short)(gb[m_off] | (gb[m_off + 1u] << 8)));
        float ud = q4k_f16_to_f32_sw2((unsigned short)(ub[d_off] | (ub[d_off + 1u] << 8)));
        float um = q4k_f16_to_f32_sw2((unsigned short)(ub[m_off] | (ub[m_off + 1u] << 8)));
        unsigned q_off = group * 32u;
        float gnib, unib;
        if (local < 32u) {
            gnib = (float)(gb[q_off + local] & 0xFu);
            unib = (float)(ub[q_off + local] & 0xFu);
        } else {
            gnib = (float)(gb[q_off + (local - 32u)] >> 4);
            unib = (float)(ub[q_off + (local - 32u)] >> 4);
        }
        float ax = act_n[t];
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
        float sg = g / (1.0f + expf(-g));
        y[row] = sg * u;
    }
}
"#;

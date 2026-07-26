//! FlashAttention-style tiled online softmax for single-token GQA decode.

pub const PAGED_GQA_TILED_ENTRY: &str = "paged_gqa_tiled_online";

/// Eight warps score thirty-two context positions per tile (four positions per warp). One
/// online-softmax merge updates value accumulators without an O(context) score buffer.
pub const PAGED_GQA_TILED_SRC: &str = r#"
extern "C" __global__ void paged_gqa_tiled_online(
    const float *q,
    const float *kv,
    float *out,
    const unsigned *params,
    const unsigned *scale_bits,
    const unsigned *layer_id,
    const unsigned *step,
    const unsigned *block_table
) {
    const unsigned n_head = params[0];
    const unsigned n_kv = params[1];
    const unsigned head_dim = params[2];
    const unsigned max_context = params[3];
    const unsigned block_size = params[4];
    const unsigned blocks_per_layer = params[5];
    const unsigned slot_kv_elems = params[6];
    unsigned q_per_kv = params[7];
    const unsigned layer = layer_id[0];
    const unsigned pos = step[0];
    const unsigned q_h = blockIdx.x;
    const unsigned tid = threadIdx.x;
    const unsigned warp = tid >> 5u;
    const unsigned lane = tid & 31u;
    if (q_per_kv == 0u) q_per_kv = 1u;
    const unsigned kv_h = q_h / q_per_kv;
    if (q_h >= n_head || kv_h >= n_kv || head_dim > 256u
        || max_context == 0u || max_context > 4096u || pos >= max_context
        || block_size == 0u || blocks_per_layer == 0u) return;

    const float scale = __int_as_float((int)scale_bits[0]);
    __shared__ float scores[32];
    __shared__ float beta[32];
    __shared__ unsigned v_bases[32];
    __shared__ float running_max;
    __shared__ float running_sum;
    __shared__ float alpha;
    float acc = 0.0f;
    if (tid == 0u) {
        running_max = -3.402823466e+38F;
        running_sum = 0.0f;
    }
    __syncthreads();

    for (unsigned tile = 0u; tile <= pos; tile += 32u) {
        #pragma unroll
        for (unsigned item = 0u; item < 4u; item++) {
            const unsigned local = warp + item * 8u;
            const unsigned past = tile + local;
            float partial = 0.0f;
            unsigned v_base = 0u;
            if (past <= pos) {
                const unsigned past_slot = past % max_context;
                const unsigned logical_block = past_slot / block_size;
                const unsigned physical_block =
                    block_table[layer * blocks_per_layer + logical_block];
                if (physical_block == 0xffffffffu) return;
                const unsigned block_offset = past_slot % block_size;
                const unsigned slot_base = physical_block * block_size * slot_kv_elems * 2u
                    + block_offset * slot_kv_elems * 2u;
                const unsigned k_base = slot_base + kv_h * head_dim;
                v_base = slot_base + n_kv * head_dim + kv_h * head_dim;
                for (unsigned d = lane; d < head_dim; d += 32u)
                    partial = fmaf(q[q_h * head_dim + d], kv[k_base + d], partial);
            }
            for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
                partial += __shfl_down_sync(0xffffffffu, partial, delta);
            if (lane == 0u) {
                scores[local] = past <= pos ? partial * scale : -3.402823466e+38F;
                v_bases[local] = v_base;
            }
        }
        __syncthreads();

        if (tid == 0u) {
            float tile_max = scores[0];
            #pragma unroll
            for (unsigned i = 1u; i < 32u; i++) tile_max = fmaxf(tile_max, scores[i]);
            const float next_max = fmaxf(running_max, tile_max);
            alpha = running_sum == 0.0f ? 0.0f : expf(running_max - next_max);
            float tile_sum = 0.0f;
            #pragma unroll
            for (unsigned i = 0u; i < 32u; i++) {
                beta[i] = tile + i <= pos ? expf(scores[i] - next_max) : 0.0f;
                tile_sum += beta[i];
            }
            running_sum = running_sum * alpha + tile_sum;
            running_max = next_max;
        }
        __syncthreads();

        if (tid < head_dim) {
            float value = acc * alpha;
            #pragma unroll
            for (unsigned i = 0u; i < 32u; i++) {
                if (tile + i <= pos)
                    value = fmaf(kv[v_bases[i] + tid], beta[i], value);
            }
            acc = value;
        }
        __syncthreads();
    }
    if (tid < head_dim)
        out[q_h * head_dim + tid] = acc / fmaxf(running_sum, 1.0e-20f);
}
"#;

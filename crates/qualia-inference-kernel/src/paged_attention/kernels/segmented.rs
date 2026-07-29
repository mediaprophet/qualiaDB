//! Multi-block online softmax for long-context single-token GQA decode.

pub const PAGED_GQA_SEGMENTED_PARTIAL_ENTRY: &str = "paged_gqa_segmented_partial";
pub const PAGED_GQA_SEGMENTED_MERGE_ENTRY: &str = "paged_gqa_segmented_merge";
pub const MAX_ATTENTION_SEGMENTS: usize = 8;

pub fn segments_for_position(position: u32) -> usize {
    match position {
        0..=511 => 1,
        512..=2047 => 4,
        _ => MAX_ATTENTION_SEGMENTS,
    }
}

pub const PAGED_GQA_SEGMENTED_SRC: &str = r#"
extern "C" __global__ void paged_gqa_segmented_partial(
    const float *q,
    const float *kv,
    float *partial_out,
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
    const unsigned segments = params[8];
    const unsigned layer = layer_id[0];
    const unsigned pos = step[0];
    const unsigned q_h = blockIdx.x / segments;
    const unsigned segment = blockIdx.x % segments;
    const unsigned tid = threadIdx.x;
    const unsigned warp = tid >> 5u;
    const unsigned lane = tid & 31u;
    if (q_per_kv == 0u) q_per_kv = 1u;
    const unsigned kv_h = q_h / q_per_kv;
    if (q_h >= n_head || kv_h >= n_kv || head_dim > 256u || segments == 0u
        || segments > 8u || max_context == 0u || pos >= max_context
        || block_size == 0u || blocks_per_layer == 0u) return;

    const unsigned token_count = pos + 1u;
    const unsigned segment_tokens = (token_count + segments - 1u) / segments;
    const unsigned begin = segment * segment_tokens;
    const unsigned end = min(begin + segment_tokens, token_count);
    const unsigned stride = head_dim + 2u;
    const unsigned partial_base = (q_h * segments + segment) * stride;
    if (begin >= end) {
        if (tid == 0u) {
            partial_out[partial_base] = -3.402823466e+38F;
            partial_out[partial_base + 1u] = 0.0f;
        }
        if (tid < head_dim) partial_out[partial_base + 2u + tid] = 0.0f;
        return;
    }

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

    for (unsigned tile = begin; tile < end; tile += 32u) {
        #pragma unroll
        for (unsigned item = 0u; item < 4u; item++) {
            const unsigned local = warp + item * 8u;
            const unsigned past = tile + local;
            float dot = 0.0f;
            unsigned v_base = 0u;
            if (past < end) {
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
                    dot = fmaf(q[q_h * head_dim + d], kv[k_base + d], dot);
            }
            for (unsigned delta = 16u; delta > 0u; delta >>= 1u)
                dot += __shfl_down_sync(0xffffffffu, dot, delta);
            if (lane == 0u) {
                scores[local] = past < end ? dot * scale : -3.402823466e+38F;
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
                beta[i] = tile + i < end ? expf(scores[i] - next_max) : 0.0f;
                tile_sum += beta[i];
            }
            running_sum = running_sum * alpha + tile_sum;
            running_max = next_max;
        }
        __syncthreads();
        if (tid < head_dim) {
            float value = acc * alpha;
            #pragma unroll
            for (unsigned i = 0u; i < 32u; i++)
                if (tile + i < end) value = fmaf(kv[v_bases[i] + tid], beta[i], value);
            acc = value;
        }
        __syncthreads();
    }
    if (tid == 0u) {
        partial_out[partial_base] = running_max;
        partial_out[partial_base + 1u] = running_sum;
    }
    if (tid < head_dim) partial_out[partial_base + 2u + tid] = acc;
}

extern "C" __global__ void paged_gqa_segmented_merge(
    const float *partial_in,
    float *out,
    const unsigned *params
) {
    const unsigned n_head = params[0];
    const unsigned head_dim = params[2];
    const unsigned segments = params[8];
    const unsigned q_h = blockIdx.x;
    const unsigned tid = threadIdx.x;
    if (q_h >= n_head || head_dim > 256u || segments == 0u || segments > 8u) return;
    const unsigned stride = head_dim + 2u;
    __shared__ float merged_max;
    __shared__ float merged_sum;
    __shared__ float scales[8];
    if (tid == 0u) {
        float max_value = -3.402823466e+38F;
        for (unsigned segment = 0u; segment < segments; segment++) {
            const unsigned base = (q_h * segments + segment) * stride;
            if (partial_in[base + 1u] > 0.0f) max_value = fmaxf(max_value, partial_in[base]);
        }
        float sum = 0.0f;
        for (unsigned segment = 0u; segment < segments; segment++) {
            const unsigned base = (q_h * segments + segment) * stride;
            const float factor = partial_in[base + 1u] > 0.0f
                ? expf(partial_in[base] - max_value) : 0.0f;
            scales[segment] = factor;
            sum += partial_in[base + 1u] * factor;
        }
        merged_max = max_value;
        merged_sum = sum;
    }
    __syncthreads();
    if (tid < head_dim) {
        float value = 0.0f;
        for (unsigned segment = 0u; segment < segments; segment++) {
            const unsigned base = (q_h * segments + segment) * stride;
            value = fmaf(partial_in[base + 2u + tid], scales[segment], value);
        }
        out[q_h * head_dim + tid] = value / fmaxf(merged_sum, 1.0e-20f);
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_thresholds_are_bounded() {
        assert_eq!(segments_for_position(511), 1);
        assert_eq!(segments_for_position(512), 4);
        assert_eq!(segments_for_position(2047), 4);
        assert_eq!(segments_for_position(2048), 8);
        assert_eq!(segments_for_position(4095), MAX_ATTENTION_SEGMENTS);
    }
}

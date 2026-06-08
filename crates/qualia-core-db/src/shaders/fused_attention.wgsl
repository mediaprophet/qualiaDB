// Fused GQA attention: Q4_K/Q6_K GEMM, in-shader RoPE, ring-buffer KV write, online softmax.

struct AttentionParams {
    n_embd: u32,
    n_head: u32,
    n_kv_head: u32,
    head_dim: u32,
    q_heads_per_kv: u32,
    token_idx: u32,
    max_context: u32,
    layer_idx: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    proj_kind: u32, // 0=Q+attn 1=K 2=V
    rope_theta_base: f32,
    num_tokens_in_batch: u32, // 1 during decode; >1 during chunked prefill
    batch_start_token_idx: u32,
}

@group(0) @binding(0) var<storage, read> hidden: array<f32>;
@group(0) @binding(1) var<storage, read> weight_words: array<u32>;
@group(0) @binding(2) var<uniform> params: AttentionParams;
@group(0) @binding(3) var<storage, read_write> kv_cache: array<f32>;
@group(0) @binding(4) var<storage, read_write> attn_out: array<f32>;

const BLOCK_Q6K_BYTES: u32 = 210u;
const BLOCK_Q6K_ELEMS: u32 = 256u;
const BLOCK_Q4K_BYTES: u32 = 144u;
const BLOCK_Q4K_ELEMS: u32 = 256u;
const GGML_TYPE_Q4_K: u32 = 12u;
const GGML_TYPE_Q6_K: u32 = 14u;
const MAX_HEAD_DIM: u32 = 512u;
const NEG_INF: f32 = -1e30;

fn read_u8_weight(abs_byte: u32) -> u32 {
    let word = abs_byte >> 2u;
    let shift = (abs_byte & 3u) * 8u;
    return (weight_words[word] >> shift) & 0xFFu;
}

fn f16_to_f32(bits: u32) -> f32 {
    let s = (bits >> 15u) & 1u;
    var e = (bits >> 10u) & 0x1Fu;
    let f = bits & 0x3FFu;
    if e == 0u {
        if f == 0u { return select(0.0, -0.0, s == 1u); }
        e = 1u;
        var v = f32(f) / 1024.0;
        v *= exp2(-14.0);
        return select(v, -v, s == 1u);
    }
    if e == 31u { return select(1e30, -1e30, s == 1u); }
    var v = 1.0 + f32(f) / 1024.0;
    v *= exp2(f32(i32(e) - 15));
    return select(v, -v, s == 1u);
}

fn i8_from_u8(b: u32) -> i32 {
    if b > 127u { return i32(b) - 256; }
    return i32(b);
}

fn weight_row_bytes() -> u32 {
    if params.weight_ggml_type == GGML_TYPE_Q4_K {
        return (params.weight_row_elems / BLOCK_Q4K_ELEMS) * BLOCK_Q4K_BYTES;
    }
    return (params.weight_row_elems / BLOCK_Q6K_ELEMS) * BLOCK_Q6K_BYTES;
}

fn get_scale_min_k4(j: u32, scales_base: u32) -> vec2<u32> {
    if j < 4u {
        return vec2<u32>(read_u8_weight(scales_base + j) & 63u, read_u8_weight(scales_base + j + 4u) & 63u);
    }
    let sc = (read_u8_weight(scales_base + j + 4u) & 0xFu) | ((read_u8_weight(scales_base + j - 4u) >> 6u) << 4u);
    let m = (read_u8_weight(scales_base + j + 4u) >> 4u) | ((read_u8_weight(scales_base + j) >> 6u) << 4u);
    return vec2<u32>(sc, m);
}

fn dequant_q4_k_elem(block_base: u32, elem: u32) -> f32 {
    let d = f16_to_f32(read_u8_weight(block_base) | (read_u8_weight(block_base + 1u) << 8u));
    let dmin = f16_to_f32(read_u8_weight(block_base + 2u) | (read_u8_weight(block_base + 3u) << 8u));
    let scales_base = block_base + 4u;
    let qs_base = block_base + 16u;
    let group = elem / 64u;
    let is = group * 2u;
    let local = elem % 64u;
    let sm0 = get_scale_min_k4(is, scales_base);
    let sm1 = get_scale_min_k4(is + 1u, scales_base);
    let d1 = d * f32(sm0.x);
    let m1 = dmin * f32(sm0.y);
    let d2 = d * f32(sm1.x);
    let m2 = dmin * f32(sm1.y);
    let q_off = group * 32u;
    if local < 32u {
        let nib = read_u8_weight(qs_base + q_off + local) & 0xFu;
        return d1 * f32(nib) - m1;
    }
    let nib = read_u8_weight(qs_base + q_off + (local - 32u)) >> 4u;
    return d2 * f32(nib) - m2;
}

fn dequant_q4_k_weight(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q4K_ELEMS;
    let block_base = row_base + block_in_row * BLOCK_Q4K_BYTES;
    let elem = col % BLOCK_Q4K_ELEMS;
    return dequant_q4_k_elem(block_base, elem);
}

fn dequant_q6_k_weight(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q6K_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q6K_BYTES;
    let y_in_block = col % BLOCK_Q6K_ELEMS;
    let d_bits = read_u8_weight(base + 208u) | (read_u8_weight(base + 209u) << 8u);
    let d = f16_to_f32(d_bits);
    let chunk = y_in_block / 128u;
    let y_in = y_in_block % 128u;
    let group = y_in / 32u;
    let l = y_in % 32u;
    let ql_off = chunk * 64u;
    let qh_off = 128u + chunk * 32u;
    let sc_off = 192u + chunk * 8u;
    let is = l / 16u;
    var q: i32;
    var sc_idx: u32;
    if group == 0u {
        q = i32((read_u8_weight(base + ql_off + l) & 0xFu) | (((read_u8_weight(base + qh_off + l) >> 0u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is;
    } else if group == 1u {
        q = i32((read_u8_weight(base + ql_off + l + 32u) & 0xFu) | (((read_u8_weight(base + qh_off + l) >> 2u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 2u;
    } else if group == 2u {
        q = i32((read_u8_weight(base + ql_off + l) >> 4u) | (((read_u8_weight(base + qh_off + l) >> 4u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 4u;
    } else {
        q = i32((read_u8_weight(base + ql_off + l + 32u) >> 4u) | (((read_u8_weight(base + qh_off + l) >> 6u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 6u;
    }
    let sc = i8_from_u8(read_u8_weight(base + sc_idx));
    return d * f32(sc) * f32(q);
}

fn dequant_weight(row: u32, col: u32) -> f32 {
    if params.weight_ggml_type == GGML_TYPE_Q4_K {
        return dequant_q4_k_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q6_K {
        return dequant_q6_k_weight(row, col);
    }
    return 0.0;
}

fn gemm_row(row: u32, token_in_batch: u32) -> f32 {
    let h_base = token_in_batch * params.n_embd;
    var sum = 0.0;
    for (var j = 0u; j < params.n_embd; j = j + 1u) {
        sum = sum + dequant_weight(row, j) * hidden[h_base + j];
    }
    return sum;
}

fn rotate_rope_pair(x0: f32, x1: f32, pos: u32, pair_idx: u32) -> vec2<f32> {
    let theta = pow(params.rope_theta_base, -2.0 * f32(pair_idx) / f32(params.head_dim));
    let angle = f32(pos) * theta;
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(x0 * c - x1 * s, x0 * s + x1 * c);
}

// Layer-local indices: bind group maps one layer slice of the static arena.
fn k_cache_idx(slot: u32, kv_head: u32, dim: u32) -> u32 {
    let base = slot * params.slot_kv_elems * 2u;
    return base + kv_head * params.head_dim + dim;
}

fn v_cache_idx(slot: u32, kv_head: u32, dim: u32) -> u32 {
    let base = slot * params.slot_kv_elems * 2u;
    let v_base = base + params.n_kv_head * params.head_dim;
    return v_base + kv_head * params.head_dim + dim;
}

fn online_softmax_attention(qh: u32, kv_head: u32) {
    var q: array<f32, MAX_HEAD_DIM>;
    let row_base = qh * params.head_dim;
    let token_in_batch = 0u;
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        q[d] = gemm_row(row_base + d, token_in_batch);
    }
    let pairs = params.head_dim / 2u;
    for (var p = 0u; p < pairs; p = p + 1u) {
        let rot = rotate_rope_pair(q[p * 2u], q[p * 2u + 1u], params.token_idx, p);
        q[p * 2u] = rot.x;
        q[p * 2u + 1u] = rot.y;
    }

    var m_max = NEG_INF;
    var l_sum = 0.0;
    var out_acc: array<f32, MAX_HEAD_DIM>;
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        out_acc[d] = 0.0;
    }

    let seq_len = params.token_idx + 1u;
    let start = select(0u, seq_len - params.max_context, seq_len > params.max_context);
    let scale = 1.0 / sqrt(f32(params.head_dim));
    for (var logical = start; logical <= params.token_idx; logical = logical + 1u) {
        let slot = logical % params.max_context;
        var score = 0.0;
        for (var d = 0u; d < params.head_dim; d = d + 1u) {
            score = score + q[d] * kv_cache[k_cache_idx(slot, kv_head, d)];
        }
        score = score * scale;
        let m_new = max(m_max, score);
        let w = exp(score - m_new);
        let factor = exp(m_max - m_new);
        m_max = m_new;
        l_sum = l_sum * factor + w;
        for (var d = 0u; d < params.head_dim; d = d + 1u) {
            out_acc[d] = out_acc[d] * factor + w * kv_cache[v_cache_idx(slot, kv_head, d)];
        }
    }

    let inv = select(0.0, 1.0 / l_sum, l_sum > 0.0);
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        attn_out[row_base + d] = out_acc[d] * inv;
    }
}

fn write_kv_head(kv_head: u32, token_in_batch: u32, abs_pos: u32, apply_rope_k: bool) {
    var vec: array<f32, MAX_HEAD_DIM>;
    let row_base = kv_head * params.head_dim;
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        vec[d] = gemm_row(row_base + d, token_in_batch);
    }
    if apply_rope_k {
        let pairs = params.head_dim / 2u;
        for (var p = 0u; p < pairs; p = p + 1u) {
            let rot = rotate_rope_pair(vec[p * 2u], vec[p * 2u + 1u], abs_pos, p);
            vec[p * 2u] = rot.x;
            vec[p * 2u + 1u] = rot.y;
        }
    }
    let slot = abs_pos % params.max_context;
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        if params.proj_kind == 1u {
            kv_cache[k_cache_idx(slot, kv_head, d)] = vec[d];
        } else {
            kv_cache[v_cache_idx(slot, kv_head, d)] = vec[d];
        }
    }
}

// Decode: one workgroup per Q head (proj_kind=0) or KV head (proj_kind=1|2).
// Prefill: one workgroup per (token_in_batch, kv_head) for batched K/V writes.
@compute @workgroup_size(1)
fn main(@builtin(workgroup_id) wg_id: vec3<u32>) {
    if params.proj_kind == 1u || params.proj_kind == 2u {
        let pair = wg_id.x;
        let token_in_batch = pair / params.n_kv_head;
        let kv_head = pair % params.n_kv_head;
        if token_in_batch >= params.num_tokens_in_batch || kv_head >= params.n_kv_head {
            return;
        }
        let abs_pos = params.batch_start_token_idx + token_in_batch;
        write_kv_head(kv_head, token_in_batch, abs_pos, params.proj_kind == 1u);
        return;
    }
    if params.proj_kind == 0u {
        let qh = wg_id.x;
        if qh >= params.n_head {
            return;
        }
        let kv_head = qh / params.q_heads_per_kv;
        online_softmax_attention(qh, kv_head);
    }
}

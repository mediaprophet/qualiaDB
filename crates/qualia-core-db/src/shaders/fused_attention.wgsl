// Fused GQA attention: Q/K/V quantized GEMM, RoPE, ring-buffer KV write, scaled dot-product.
// Phase 1 scaffold — full GPU path pairs with CPU fallback in gguf_bridge.rs for Q4_K weights.

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
    proj_kind: u32, // 0=Q 1=K 2=V
}

@group(0) @binding(0) var<storage, read> hidden: array<f32>;
@group(0) @binding(1) var<storage, read> weight_words: array<u32>;
@group(0) @binding(2) var<uniform> params: AttentionParams;
@group(0) @binding(3) var<storage, read_write> kv_cache: array<f32>;
@group(0) @binding(4) var<storage, read_write> attn_out: array<f32>;

const BLOCK_Q6K_BYTES: u32 = 210u;
const BLOCK_Q6K_ELEMS: u32 = 256u;
const GGML_TYPE_Q6_K: u32 = 14u;
const PI: f32 = 3.141592653589793;

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
    return (params.weight_row_elems / BLOCK_Q6K_ELEMS) * BLOCK_Q6K_BYTES;
}

fn dequant_q6_k_weight(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let y = col;
    let block_in_row = y / BLOCK_Q6K_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q6K_BYTES;
    let y_in_block = y % BLOCK_Q6K_ELEMS;
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
    if params.weight_ggml_type == GGML_TYPE_Q6_K {
        return dequant_q6_k_weight(row, col);
    }
    return 0.0;
}

// RoPE + KV ring write + GQA softmax run on CPU today; GPU scaffold projects one weight row.
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_idx = global_id.x;
    if out_idx >= params.n_embd {
        return;
    }
    var sum = 0.0;
    for (var j = 0u; j < params.n_embd; j = j + 1u) {
        sum = sum + dequant_weight(out_idx, j) * hidden[j];
    }
    attn_out[out_idx] = sum;
}

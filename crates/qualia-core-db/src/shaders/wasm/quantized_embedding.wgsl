// Zero-copy quantized token embedding matmul.
// Raw mmap bytes are uploaded as u32-aligned words; dequant runs on-GPU.

struct EmbeddingParams {
    n_embd: u32,
    ggml_type: u32,
    n_output: u32,
    raw_byte_len: u32,
}

@group(0) @binding(0) var<storage, read> embd_words: array<u32>;
@group(0) @binding(1) var<uniform> params: EmbeddingParams;
@group(0) @binding(2) var<storage, read> weights: array<f32>;
@group(0) @binding(3) var<storage, read_write> output_logits: array<f32>;

const BLOCK_Q6K_BYTES: u32 = 210u;
const BLOCK_Q6K_ELEMS: u32 = 256u;
const BLOCK_Q4_0_BYTES: u32 = 18u;
const BLOCK_Q4_0_ELEMS: u32 = 32u;
const GGML_TYPE_Q4_0: u32 = 2u;
const GGML_TYPE_Q6_K: u32 = 14u;

fn read_u8(idx: u32) -> u32 {
    let word = idx >> 2u;
    let shift = (idx & 3u) * 8u;
    return (embd_words[word] >> shift) & 0xFFu;
}

fn f16_to_f32(bits: u32) -> f32 {
    let s = (bits >> 15u) & 1u;
    var e = (bits >> 10u) & 0x1Fu;
    let f = bits & 0x3FFu;
    if e == 0u {
        if f == 0u {
            return select(0.0, -0.0, s == 1u);
        }
        e = 1u;
        var v = f32(f) / 1024.0;
        v *= exp2(-14.0);
        return select(v, -v, s == 1u);
    }
    if e == 31u {
        return select(1e30, -1e30, s == 1u);
    }
    var v = 1.0 + f32(f) / 1024.0;
    v *= exp2(f32(i32(e) - 15));
    return select(v, -v, s == 1u);
}

fn i8_from_u8(b: u32) -> i32 {
    if b > 127u {
        return i32(b) - 256;
    }
    return i32(b);
}

fn dequantize_q6_k_elem(k: u32) -> f32 {
    let block_idx = k / BLOCK_Q6K_ELEMS;
    let base = block_idx * BLOCK_Q6K_BYTES;
    let y = k % BLOCK_Q6K_ELEMS;

    let d_bits = read_u8(base + 208u) | (read_u8(base + 209u) << 8u);
    let d = f16_to_f32(d_bits);

    let chunk = y / 128u;
    let y_in = y % 128u;
    let group = y_in / 32u;
    let l = y_in % 32u;

    let ql_off = chunk * 64u;
    let qh_off = 128u + chunk * 32u;
    let sc_off = 192u + chunk * 8u;
    let is = l / 16u;

    var q: i32;
    var sc_idx: u32;
    if group == 0u {
        q = i32((read_u8(base + ql_off + l) & 0xFu) | (((read_u8(base + qh_off + l) >> 0u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is;
    } else if group == 1u {
        q = i32((read_u8(base + ql_off + l + 32u) & 0xFu) | (((read_u8(base + qh_off + l) >> 2u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 2u;
    } else if group == 2u {
        q = i32((read_u8(base + ql_off + l) >> 4u) | (((read_u8(base + qh_off + l) >> 4u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 4u;
    } else {
        q = i32((read_u8(base + ql_off + l + 32u) >> 4u) | (((read_u8(base + qh_off + l) >> 6u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 6u;
    }

    let sc = i8_from_u8(read_u8(base + sc_idx));
    return d * f32(sc) * f32(q);
}

fn dequantize_q4_0_elem(k: u32) -> f32 {
    let block_idx = k / BLOCK_Q4_0_ELEMS;
    let base = block_idx * BLOCK_Q4_0_BYTES;
    let y = k % BLOCK_Q4_0_ELEMS;

    let d_bits = read_u8(base) | (read_u8(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);

    let half_idx = y % 16u;
    let byte_val = read_u8(base + 2u + half_idx);

    var nibble: u32;
    if y < 16u {
        nibble = byte_val & 0xFu;
    } else {
        nibble = byte_val >> 4u;
    }

    let q = i32(nibble) - 8;
    return d * f32(q);
}

fn dequantize_embd_elem(k: u32) -> f32 {
    if params.ggml_type == GGML_TYPE_Q4_0 {
        return dequantize_q4_0_elem(k);
    }
    if params.ggml_type == GGML_TYPE_Q6_K {
        return dequantize_q6_k_elem(k);
    }
    return 0.0;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    if i >= params.n_output {
        return;
    }

    var sum = 0.0;
    for (var k = 0u; k < params.n_embd; k = k + 1u) {
        let emb = dequantize_embd_elem(k);
        sum = sum + emb * weights[i * params.n_embd + k];
    }
    output_logits[i] = max(0.0, sum);
}

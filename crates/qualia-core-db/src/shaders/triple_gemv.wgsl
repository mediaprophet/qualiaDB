// Triple Q4_K_SOA GEMV: one workgroup per Q-row, shared activation tile,
// three weight matrices (Q+K+V) → three outputs. GQA-safe: K/V only when
// `row < n_kv` (n_kv packed in params.weight_byte_len).
// Cuts 3 dispatches → 1 in resident decode.
//
// Dispatch: (n_q, batch, 1) workgroups of 256.
// Bindings:
//   0 input, 1 W_q, 2 params, 3 out_q, 4 W_k, 5 out_k, 6 W_v, 7 out_v

struct GemmParams {
    n_in: u32,
    n_out: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32, // reused: n_kv rows
    n_batch: u32,
    in_row_stride: u32,
    out_row_stride: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight_q: array<u32>;
@group(0) @binding(2) var<uniform> params: GemmParams;
@group(0) @binding(3) var<storage, read_write> out_q: array<f32>;
@group(0) @binding(4) var<storage, read> weight_k: array<u32>;
@group(0) @binding(5) var<storage, read_write> out_k: array<f32>;
@group(0) @binding(6) var<storage, read> weight_v: array<u32>;
@group(0) @binding(7) var<storage, read_write> out_v: array<f32>;

const BLOCK_Q4K_SOA_BYTES: u32 = 160u;
const BLOCK_Q4K_ELEMS: u32 = 256u;
const COOP_WG: u32 = 256u;

var<workgroup> coop_act: array<f32, 256>;
var<workgroup> coop_act_b: array<f32, 256>;
var<workgroup> partial_q: array<f32, 256>;
var<workgroup> partial_k: array<f32, 256>;
var<workgroup> partial_v: array<f32, 256>;

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

fn weight_row_bytes() -> u32 {
    return (params.weight_row_elems / BLOCK_Q4K_ELEMS) * BLOCK_Q4K_SOA_BYTES;
}

fn dequant_fma_q(block_base: u32, t: u32, x: f32) -> f32 {
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;
    let scale_pair = sub >> 1u;
    let scale_hi = (sub & 1u) == 1u;
    let q_off = group * 32u;
    let d_word = weight_q[(block_base + 128u) / 4u + scale_pair];
    let m_word = weight_q[(block_base + 144u) / 4u + scale_pair];
    let dsub = f16_to_f32(select(d_word & 0xFFFFu, d_word >> 16u, scale_hi));
    let msub = f16_to_f32(select(m_word & 0xFFFFu, m_word >> 16u, scale_hi));
    var nib: u32;
    if local < 32u {
        let byte_i = block_base + q_off + local;
        nib = (weight_q[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFu;
    } else {
        let byte_i = block_base + q_off + (local - 32u);
        nib = ((weight_q[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFFu) >> 4u;
    }
    return (dsub * f32(nib) - msub) * x;
}

fn dequant_fma_k(block_base: u32, t: u32, x: f32) -> f32 {
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;
    let scale_pair = sub >> 1u;
    let scale_hi = (sub & 1u) == 1u;
    let q_off = group * 32u;
    let d_word = weight_k[(block_base + 128u) / 4u + scale_pair];
    let m_word = weight_k[(block_base + 144u) / 4u + scale_pair];
    let dsub = f16_to_f32(select(d_word & 0xFFFFu, d_word >> 16u, scale_hi));
    let msub = f16_to_f32(select(m_word & 0xFFFFu, m_word >> 16u, scale_hi));
    var nib: u32;
    if local < 32u {
        let byte_i = block_base + q_off + local;
        nib = (weight_k[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFu;
    } else {
        let byte_i = block_base + q_off + (local - 32u);
        nib = ((weight_k[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFFu) >> 4u;
    }
    return (dsub * f32(nib) - msub) * x;
}

fn dequant_fma_v(block_base: u32, t: u32, x: f32) -> f32 {
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;
    let scale_pair = sub >> 1u;
    let scale_hi = (sub & 1u) == 1u;
    let q_off = group * 32u;
    let d_word = weight_v[(block_base + 128u) / 4u + scale_pair];
    let m_word = weight_v[(block_base + 144u) / 4u + scale_pair];
    let dsub = f16_to_f32(select(d_word & 0xFFFFu, d_word >> 16u, scale_hi));
    let msub = f16_to_f32(select(m_word & 0xFFFFu, m_word >> 16u, scale_hi));
    var nib: u32;
    if local < 32u {
        let byte_i = block_base + q_off + local;
        nib = (weight_v[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFu;
    } else {
        let byte_i = block_base + q_off + (local - 32u);
        nib = ((weight_v[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFFu) >> 4u;
    }
    return (dsub * f32(nib) - msub) * x;
}

@compute @workgroup_size(256)
fn coop_gemv_triple(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let row = wg_id.x;
    let n_q = params.n_out;
    let n_kv = params.weight_byte_len;
    if row >= n_q { return; }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride_q = select(n_q, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base_q = m * out_stride_q;
    let do_kv = row < n_kv;

    var acc_q = 0.0;
    var acc_k = 0.0;
    var acc_v = 0.0;
    let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
    let row_base = row * weight_row_bytes();

    if n_blocks > 0u {
        coop_act[t] = input[in_base + t];
    }
    workgroupBarrier();
    for (var b = 0u; b < n_blocks; b = b + 1u) {
        let use_a = (b & 1u) == 0u;
        if b + 1u < n_blocks {
            let nxt = input[in_base + (b + 1u) * BLOCK_Q4K_ELEMS + t];
            if use_a {
                coop_act_b[t] = nxt;
            } else {
                coop_act[t] = nxt;
            }
        }
        let x = select(coop_act_b[t], coop_act[t], use_a);
        let block_base = row_base + b * BLOCK_Q4K_SOA_BYTES;
        acc_q = acc_q + dequant_fma_q(block_base, t, x);
        if do_kv {
            acc_k = acc_k + dequant_fma_k(block_base, t, x);
            acc_v = acc_v + dequant_fma_v(block_base, t, x);
        }
        workgroupBarrier();
    }

    partial_q[t] = acc_q;
    partial_k[t] = select(0.0, acc_k, do_kv);
    partial_v[t] = select(0.0, acc_v, do_kv);
    workgroupBarrier();
    var stride = COOP_WG >> 1u;
    loop {
        if stride == 0u { break; }
        if t < stride {
            partial_q[t] = partial_q[t] + partial_q[t + stride];
            if do_kv {
                partial_k[t] = partial_k[t] + partial_k[t + stride];
                partial_v[t] = partial_v[t] + partial_v[t + stride];
            }
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if t == 0u {
        out_q[out_base_q + row] = partial_q[0];
        if do_kv {
            out_k[row] = partial_k[0];
            out_v[row] = partial_v[0];
        }
    }
}

// Layer-by-layer quantized-weight GEMM: f32 activations × mmap Q4_K/Q6_K weights.
// Weight bytes are uploaded per-tensor via write_buffer (reused staging buffer).

struct GemmParams {
    n_in: u32,
    n_out: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    n_batch: u32,         // M in M×K×N; 1 = legacy vector×matrix
    in_row_stride: u32,   // floats between input rows; 0 → n_in
    out_row_stride: u32,  // floats between output rows; 0 → n_out
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight_words: array<u32>;
@group(0) @binding(2) var<uniform> params: GemmParams;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

const BLOCK_Q6K_BYTES: u32 = 210u;
const BLOCK_Q6K_ELEMS: u32 = 256u;
const BLOCK_Q4K_BYTES: u32 = 144u;
const BLOCK_Q4K_ELEMS: u32 = 256u;
const BLOCK_Q4_0_BYTES: u32 = 18u;
const BLOCK_Q4_0_ELEMS: u32 = 32u;
const BLOCK_Q5_0_BYTES: u32 = 22u;
const BLOCK_Q5_0_ELEMS: u32 = 32u;
const BLOCK_Q8_0_BYTES: u32 = 34u;
const BLOCK_Q8_0_ELEMS: u32 = 32u;
const GGML_TYPE_Q4_0: u32 = 2u;
const GGML_TYPE_Q5_0: u32 = 6u;
const GGML_TYPE_Q8_0: u32 = 8u;
const GGML_TYPE_Q4_K: u32 = 12u;
const GGML_TYPE_Q6_K: u32 = 14u;
const GGML_TYPE_F16: u32 = 1u;

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
    if params.weight_ggml_type == GGML_TYPE_Q4_0 {
        return (params.weight_row_elems / BLOCK_Q4_0_ELEMS) * BLOCK_Q4_0_BYTES;
    }
    if params.weight_ggml_type == GGML_TYPE_Q5_0 {
        return (params.weight_row_elems / BLOCK_Q5_0_ELEMS) * BLOCK_Q5_0_BYTES;
    }
    if params.weight_ggml_type == GGML_TYPE_Q8_0 {
        return (params.weight_row_elems / BLOCK_Q8_0_ELEMS) * BLOCK_Q8_0_BYTES;
    }
    if params.weight_ggml_type == GGML_TYPE_Q4_K {
        return (params.weight_row_elems / BLOCK_Q4K_ELEMS) * BLOCK_Q4K_BYTES;
    }
    if params.weight_ggml_type == GGML_TYPE_F16 {
        return params.weight_row_elems * 2u;
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

fn dequant_q4_0_weight(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q4_0_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q4_0_BYTES;
    let y = col % BLOCK_Q4_0_ELEMS;

    let d_bits = read_u8_weight(base) | (read_u8_weight(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);

    let half_idx = y % 16u;
    let byte_val = read_u8_weight(base + 2u + half_idx);

    var nibble: u32;
    if y < 16u {
        nibble = byte_val & 0xFu;
    } else {
        nibble = byte_val >> 4u;
    }

    let q = i32(nibble) - 8;
    return d * f32(q);
}

// block_q5_0: d(f16) + qh(u32) + qs[16] — matches ggml-quants.c / ggml_quants.rs
fn dequant_q5_0_weight(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q5_0_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q5_0_BYTES;
    let y = col % BLOCK_Q5_0_ELEMS;

    let d_bits = read_u8_weight(base) | (read_u8_weight(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);
    let qh = read_u8_weight(base + 2u)
        | (read_u8_weight(base + 3u) << 8u)
        | (read_u8_weight(base + 4u) << 16u)
        | (read_u8_weight(base + 5u) << 24u);

    let half = BLOCK_Q5_0_ELEMS / 2u;
    let j = y % half;
    let qs_byte = read_u8_weight(base + 6u + j);

    var q: i32;
    if y < half {
        let xh = ((qh >> j) << 4u) & 0x10u;
        q = i32((qs_byte & 0xFu) | xh) - 16;
    } else {
        let xh = (qh >> (j + 12u)) & 0x10u;
        q = i32((qs_byte >> 4u) | xh) - 16;
    }
    return d * f32(q);
}

// block_q8_0: d(f16) + qs[i8; 32]
fn dequant_q8_0_weight(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q8_0_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q8_0_BYTES;
    let y = col % BLOCK_Q8_0_ELEMS;

    let d_bits = read_u8_weight(base) | (read_u8_weight(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);
    let q = i8_from_u8(read_u8_weight(base + 2u + y));
    return d * f32(q);
}

// f16 weights: row-major IEEE half-floats, 2 bytes each. `unpack2x16float` is the core-WGSL
// hardware half->f32 path (one u32 read yields two weights); no SHADER_F16 device feature required.
fn dequant_f16_weight(row: u32, col: u32) -> f32 {
    let elem = row * params.weight_row_elems + col; // linear half index
    let pair = unpack2x16float(weight_words[elem >> 1u]);
    return select(pair.x, pair.y, (elem & 1u) == 1u);
}

fn dequant_weight(row: u32, col: u32) -> f32 {
    if params.weight_ggml_type == GGML_TYPE_F16 {
        return dequant_f16_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q4_0 {
        return dequant_q4_0_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q5_0 {
        return dequant_q5_0_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q8_0 {
        return dequant_q8_0_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q4_K {
        return dequant_q4_k_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q6_K {
        return dequant_q6_k_weight(row, col);
    }
    return 0.0;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let m = global_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch {
        return;
    }
    let i = global_id.x;
    if i >= params.n_out {
        return;
    }
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;
    var sum = 0.0;
    for (var j = 0u; j < params.n_in; j = j + 1u) {
        sum = sum + dequant_weight(i, j) * input[in_base + j];
    }
    output[out_base + i] = sum;
}

// ─────────────────────────────────────────────────────────────────────────────
// Cooperative GEMV (0.0.21) — the perf lever. One workgroup per output row; 256
// threads cooperatively reduce that row's dot-product, replacing `main`'s
// 1-thread/row serial loop (the measured decode bottleneck: see
// .dev-docs/SPARSE_FFN_ARCHITECTURE.md §6 and the kernel-bound diagnosis).
//
//   • Coalesced weight reads. At step s, threads t=0..255 read columns s*256+t of
//     the SAME row → consecutive elements → one coalesced memory transaction. The
//     naive kernel put adjacent threads on adjacent ROWS (n_in apart) → ~rows-wide
//     wasted bandwidth per transaction.
//   • Parallel dequant. Each thread dequantizes only its own columns, so the quant
//     ALU — the cost that made Q4_K *slower* than F16 on the naive kernel — is spread
//     across 256 threads instead of serialized on one.
//   • Portable reduction. Workgroup shared-memory tree reduction — no subgroup
//     feature required, so it runs on browser/WebGPU backends without subgroups. A
//     subgroup fast-path is a follow-up refinement, not a correctness dependency.
//
// `dequant_weight()` / `input` / `output` / `params` are shared verbatim with `main`,
// so the only numerical difference vs the proven kernel is FP reassociation of the
// sum (parity-gated by max_abs_err vs the CPU reference, not bit-equality).
//
// Dispatched as (n_out, 1, 1) workgroups; decode batch = 1 (m = 0). n_out ≤ 10240
// (MAX_STACK_GEMM_OUT) < 65535 → one workgroup per row is within dispatch limits.
const COOP_WG: u32 = 256u;
var<workgroup> coop_partial: array<f32, 256>;
// Q4_K cooperative block-header cache (0.0.21 dequant optimization). A Q4_K superblock is 256
// elements == COOP_WG, so one workgroup step processes exactly one superblock. The block header
// (super-scale `d`, super-min `dmin`, and the 8 6-bit sub-block scale/min pairs) is CONSTANT across
// all 256 elements — yet the generic `dequant_weight` path re-decodes it once *per element*, i.e.
// 256× per block per thread. Here 8 threads decode it once into shared memory and all 256 threads
// reuse it, collapsing the per-block header ALU ~32× (256→8 decodes) — the measured Q4_K GEMM
// bottleneck (F16 1264µs → Q4_K 2727µs/call; dequant ≈54% of the kernel).
var<workgroup> coop_q4k_dsub: array<f32, 8>; // d * sub_scale   per 32-element sub-block
var<workgroup> coop_q4k_msub: array<f32, 8>; // dmin * sub_min  per 32-element sub-block

@compute @workgroup_size(256)
fn coop_gemv(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    // row / m are uniform across the workgroup (from workgroup_id) → these early
    // returns and all barriers below are in uniform control flow.
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch {
        return;
    }
    let row = wg_id.x;
    if row >= params.n_out {
        return;
    }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    // Each thread accumulates a slice of the contraction dimension.
    var acc = 0.0;
    if params.weight_ggml_type == GGML_TYPE_Q4_K && (params.n_in % BLOCK_Q4K_ELEMS) == 0u {
        // Block-cooperative Q4_K path: workgroup step b == superblock b; thread t == element t.
        // Header decoded once (8 threads) into shared memory; reused by all 256 threads.
        let row_base = row * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let sub = t / 32u;     // which 32-element sub-block this thread's element belongs to
        let group = t / 64u;   // Q4_K nibble layout: 4 groups of 64, low/high nibble at ±32
        let local = t % 64u;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let block_base = row_base + b * BLOCK_Q4K_BYTES;
            // Cooperative header decode (8 threads) — d/dmin re-read per sub-thread is trivial vs
            // the 256× redundancy it replaces. Writes the 8 (d·scale, dmin·min) sub-block pairs.
            if t < 8u {
                let d = f16_to_f32(read_u8_weight(block_base) | (read_u8_weight(block_base + 1u) << 8u));
                let dmin = f16_to_f32(read_u8_weight(block_base + 2u) | (read_u8_weight(block_base + 3u) << 8u));
                let sm = get_scale_min_k4(t, block_base + 4u);
                coop_q4k_dsub[t] = d * f32(sm.x);
                coop_q4k_msub[t] = dmin * f32(sm.y);
            }
            workgroupBarrier();
            // Each thread dequantizes its own element t of this superblock from its nibble.
            let qs_base = block_base + 16u;
            let q_off = group * 32u;
            var nib: u32;
            if local < 32u {
                nib = read_u8_weight(qs_base + q_off + local) & 0xFu;
            } else {
                nib = read_u8_weight(qs_base + q_off + (local - 32u)) >> 4u;
            }
            let w = coop_q4k_dsub[sub] * f32(nib) - coop_q4k_msub[sub];
            acc = acc + w * input[in_base + b * BLOCK_Q4K_ELEMS + t];
            // Barrier before the next iteration's 8 threads overwrite the shared header.
            workgroupBarrier();
        }
    } else {
        // Generic strided path (other quant types / non-256-aligned K).
        var j = t;
        loop {
            if j >= params.n_in {
                break;
            }
            acc = acc + dequant_weight(row, j) * input[in_base + j];
            j = j + COOP_WG;
        }
    }
    coop_partial[t] = acc;
    workgroupBarrier();

    // Shared-memory tree reduction over the 256 partials.
    var stride = COOP_WG >> 1u;
    loop {
        if stride == 0u {
            break;
        }
        if t < stride {
            coop_partial[t] = coop_partial[t] + coop_partial[t + stride];
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if t == 0u {
        output[out_base + row] = coop_partial[0];
    }
}

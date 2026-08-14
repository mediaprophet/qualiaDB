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
// [WASM] residual binding (4) and all residual-dependent entry points stripped —
// native-only feature. This allows coop_gemv to compile with the 4-slot WASM bind layout.

const BLOCK_Q6K_BYTES: u32 = 210u;
const BLOCK_Q6K_ELEMS: u32 = 256u;
const BLOCK_Q4K_BYTES: u32 = 144u;
const BLOCK_Q4K_SOA_BYTES: u32 = 160u;
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
const GGML_TYPE_Q4_K_SOA: u32 = 112u;
const GGML_TYPE_Q6_K: u32 = 14u;
const GGML_TYPE_F16: u32 = 1u;
const GGML_TYPE_BF16: u32 = 30u;

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
    if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA {
        return (params.weight_row_elems / BLOCK_Q4K_ELEMS) * BLOCK_Q4K_SOA_BYTES;
    }
    if params.weight_ggml_type == GGML_TYPE_F16 || params.weight_ggml_type == GGML_TYPE_BF16 {
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

// bf16: 1 sign / 8 exp / 7 mantissa — promote by shifting into f32 high half.
fn dequant_bf16_weight(row: u32, col: u32) -> f32 {
    let elem = row * params.weight_row_elems + col;
    let word = weight_words[elem >> 1u];
    let bits16 = select(word & 0xFFFFu, word >> 16u, (elem & 1u) == 1u);
    return bitcast<f32>(bits16 << 16u);
}

fn dequant_weight(row: u32, col: u32) -> f32 {
    if params.weight_ggml_type == GGML_TYPE_F16 {
        return dequant_f16_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_BF16 {
        return dequant_bf16_weight(row, col);
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
    if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA {
        return dequant_q4_k_soa_weight(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q6_K {
        return dequant_q6_k_weight(row, col);
    }
    return 0.0;
}

// SoA Q4_K (convert-time): qs at block+0 (128 B), d_sub f16[8] at +128, m_sub f16[8] at +144.
// block_base is always 4-byte aligned (160 B blocks) → word loads for scales (A2000 INT4 path).
fn dequant_q4_k_soa_weight(row: u32, col: u32) -> f32 {
    let row_bytes = weight_row_bytes();
    let block = col / BLOCK_Q4K_ELEMS;
    let elem = col % BLOCK_Q4K_ELEMS;
    let block_base = row * row_bytes + block * BLOCK_Q4K_SOA_BYTES;
    let sub = elem / 32u;
    let group = elem / 64u;
    let local = elem % 64u;
    // Byte-addressed reads mirror the CPU oracle and fused-attention decoder.
    // This avoids backend-dependent packed-word behavior for the high half.
    let d_off = block_base + 128u + sub * 2u;
    let m_off = block_base + 144u + sub * 2u;
    let dsub = f16_to_f32(read_u8_weight(d_off) | (read_u8_weight(d_off + 1u) << 8u));
    let msub = f16_to_f32(read_u8_weight(m_off) | (read_u8_weight(m_off + 1u) << 8u));
    let qs_base = block_base;
    let q_off = group * 32u;
    var nib: u32;
    if local < 32u {
        let byte_i = qs_base + q_off + local;
        let word = weight_words[byte_i >> 2u];
        let shift = (byte_i & 3u) * 8u;
        nib = (word >> shift) & 0xFu;
    } else {
        let byte_i = qs_base + q_off + (local - 32u);
        let word = weight_words[byte_i >> 2u];
        let shift = (byte_i & 3u) * 8u;
        nib = ((word >> shift) & 0xFFu) >> 4u;
    }
    return dsub * f32(nib) - msub;
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
    // Q8_0 fast path: cache block scale, process 32 elements per block
    if params.weight_ggml_type == GGML_TYPE_Q8_0 {
        let row_base = i * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q8_0_ELEMS;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let base = row_base + b * BLOCK_Q8_0_BYTES;
            let d_bits = read_u8_weight(base) | (read_u8_weight(base + 1u) << 8u);
            let d = f16_to_f32(d_bits);
            for (var y = 0u; y < BLOCK_Q8_0_ELEMS; y = y + 1u) {
                let q = i8_from_u8(read_u8_weight(base + 2u + y));
                let j = b * BLOCK_Q8_0_ELEMS + y;
                sum = sum + d * f32(q) * input[in_base + j];
            }
        }
        // Handle remainder if n_in is not a multiple of 32
        let rem_start = n_blocks * BLOCK_Q8_0_ELEMS;
        for (var j = rem_start; j < params.n_in; j = j + 1u) {
            sum = sum + dequant_weight(i, j) * input[in_base + j];
        }
    } else {
        for (var j = 0u; j < params.n_in; j = j + 1u) {
            sum = sum + dequant_weight(i, j) * input[in_base + j];
        }
    }
    output[out_base + i] = sum;
}

// ─────────────────────────────────────────────────────────────────────────────
// Multi-row Q8_0 GEMV (llama.cpp-style): WG_SIZE=64 threads, OUTPUTS_PER_WG=4
// output rows per workgroup, THREADS_PER_BLOCK=4 threads per Q8_0 block.
//
// Key optimizations vs `main`:
//   • 4 output rows per workgroup → input vector read amortized 4×
//   • 4 threads per 32-elem block → each thread handles 8 elems (2 u32 reads)
//   • u32 packed quant reads → 4 bytes per load instead of 1
//   • Shared-memory tree reduction → 64→1 in 6 steps
//   • Block-strided iteration → 16 blocks per stride (64/4)
//
// Dispatched as (ceil(n_out/4), 1, 1) workgroups.
const MR_WG: u32 = 64u;
const MR_OUTPUTS_PER_WG: u32 = 4u;
const MR_TPB: u32 = 4u;   // threads per Q8_0 block
const MR_EPT: u32 = 8u;   // elems per thread (32/4)
var<workgroup> mr_partial: array<f32, MR_WG * MR_OUTPUTS_PER_WG>;

fn load_u32_weight(byte_offset: u32) -> u32 {
    let word_idx = byte_offset / 4u;
    let shift = (byte_offset & 3u) * 8u;
    let lo = weight_words[word_idx];
    let hi = weight_words[word_idx + 1u];
    let shifted = (lo >> shift) | (hi << (32u - shift));
    return select(shifted, lo, shift == 0u);
}

fn load_u16_weight(byte_offset: u32) -> u32 {
    let word = weight_words[byte_offset / 4u];
    let shift = (byte_offset & 2u) * 8u;
    return (word >> shift) & 0xFFFFu;
}

@compute @workgroup_size(64)
fn mul_mat_vec_q8_0(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>,
) {
    let tid = lid.x;
    let row_base = wg_id.x * MR_OUTPUTS_PER_WG;
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    let n_blocks = params.n_in / BLOCK_Q8_0_ELEMS;
    let twb = tid % MR_TPB;           // thread within block (0..3)
    let block_group = tid / MR_TPB;    // which block this thread starts at
    let n_block_groups = MR_WG / MR_TPB; // 16

    // Accumulate 4 output rows in registers
    var acc: array<f32, MR_OUTPUTS_PER_WG>;
    for (var r = 0u; r < MR_OUTPUTS_PER_WG; r = r + 1u) {
        acc[r] = 0.0;
    }

    for (var b = block_group; b < n_blocks; b = b + n_block_groups) {
        // Load 8 input elements for this thread's slice of the block
        let x_base = in_base + b * BLOCK_Q8_0_ELEMS + twb * MR_EPT;
        var x_block: array<f32, MR_EPT>;
        for (var i = 0u; i < MR_EPT; i = i + 1u) {
            x_block[i] = input[x_base + i];
        }

        for (var r = 0u; r < MR_OUTPUTS_PER_WG; r = r + 1u) {
            let output_row = row_base + r;
            if output_row >= params.n_out { continue; }
            let row_bytes = output_row * weight_row_bytes();
            let block_byte_base = row_bytes + b * BLOCK_Q8_0_BYTES;
            // f16 scale via u16 load
            let d = f16_to_f32(load_u16_weight(block_byte_base));
            // Read 8 quant bytes as 2 u32 loads (4 bytes each)
            let q0 = load_u32_weight(block_byte_base + 2u + twb * 8u);
            let q1 = load_u32_weight(block_byte_base + 2u + twb * 8u + 4u);
            var row_sum = 0.0;
            for (var bi = 0u; bi < 4u; bi = bi + 1u) {
                let qb0 = (q0 >> (bi * 8u)) & 0xFFu;
                let qb1 = (q1 >> (bi * 8u)) & 0xFFu;
                let qv0 = f32(i8_from_u8(qb0)) * d;
                let qv1 = f32(i8_from_u8(qb1)) * d;
                row_sum = row_sum + qv0 * x_block[bi];
                row_sum = row_sum + qv1 * x_block[bi + 4u];
            }
            acc[r] = acc[r] + row_sum;
        }
    }

    // Shared-memory tree reduction: 64 threads × 4 rows
    for (var r = 0u; r < MR_OUTPUTS_PER_WG; r = r + 1u) {
        mr_partial[r * MR_WG + tid] = acc[r];
    }
    workgroupBarrier();

    var stride = MR_WG / 2u;
    while (stride > 0u) {
        if tid < stride {
            for (var r = 0u; r < MR_OUTPUTS_PER_WG; r = r + 1u) {
                mr_partial[r * MR_WG + tid] = mr_partial[r * MR_WG + tid] + mr_partial[r * MR_WG + tid + stride];
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }

    if tid < MR_OUTPUTS_PER_WG {
        let output_row = row_base + tid;
        if output_row < params.n_out {
            output[out_base + output_row] = mr_partial[tid * MR_WG];
        }
    }
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
const COOP_FULL_ACT_MAX: u32 = 4096u;
var<workgroup> coop_partial: array<f32, 256>;
// Shared activation tile for one Q4 superblock (256 elems). Loaded once per block so dequant
// threads hit LDS instead of re-reading global `input` (A2000 INT4 bandwidth lever).
// Ping-pong pair: one barrier per superblock instead of two (load/compute overlap).
var<workgroup> coop_act: array<f32, 256>;
var<workgroup> coop_act_b: array<f32, 256>;
// Full activation for barrier-free Q4 paths (n_in ≤ 4096).
var<workgroup> coop_full_act: array<f32, 4096>;
// Q4_K cooperative block-header cache (0.0.21 dequant optimization). A Q4_K superblock is 256
// elements == COOP_WG, so one workgroup step processes exactly one superblock. The block header
// (super-scale `d`, super-min `dmin`, and the 8 6-bit sub-block scale/min pairs) is CONSTANT across
// all 256 elements — yet the generic `dequant_weight` path re-decodes it once *per element*, i.e.
// 256× per block per thread. Here 8 threads decode it once into shared memory and all 256 threads
// reuse it, collapsing the per-block header ALU ~32× (256→8 decodes) — the measured Q4_K GEMM
// bottleneck (F16 1264µs → Q4_K 2727µs/call; dequant ≈54% of the kernel).
// Ping-pong header slots (2 × 8 sub-block pairs). Even/odd superblocks write alternate slots so
// the trailing barrier that used to guard overwrite can be dropped — one barrier per block instead
// of two (measured Q4_K GEMV bottleneck: dequant + barriers).
var<workgroup> coop_q4k_dsub: array<f32, 16>; // d * sub_scale
var<workgroup> coop_q4k_msub: array<f32, 16>; // dmin * sub_min

// Shared accumulation: thread `t`'s partial dot-product of weight row `row` with the activation at
// `in_base`. Owns the block-cooperative Q4_K dequant (header decoded once per superblock into shared
// memory, reused by all 256 threads). Called from both the shared-memory `coop_gemv` and the
// subgroup `coop_gemv_sg` (coop_gemv_subgroup.wgsl) so the dequant logic lives in exactly one place.
// Contains `workgroupBarrier()` — must be called from uniform workgroup control flow (both callers do).
fn coop_row_dot(row: u32, t: u32, in_base: u32) -> f32 {
    var acc = 0.0;
    if params.weight_ggml_type == GGML_TYPE_F16 {
        // Conversion-time f16 pages (p64 --layout f16): one u32 load yields two weights via
        // unpack2x16float — far fewer memory ops than Q4_K nibble extract.
        let row_base = row * params.weight_row_elems; // linear half index of row start
        var j = t;
        loop {
            if j >= params.n_in {
                break;
            }
            let elem = row_base + j;
            let pair = unpack2x16float(weight_words[elem >> 1u]);
            let w = select(pair.x, pair.y, (elem & 1u) == 1u);
            acc = acc + w * input[in_base + j];
            j = j + COOP_WG;
        }
    } else if params.weight_ggml_type == GGML_TYPE_BF16 {
        let row_base = row * params.weight_row_elems;
        var j = t;
        loop {
            if j >= params.n_in {
                break;
            }
            let elem = row_base + j;
            let word = weight_words[elem >> 1u];
            let bits16 = select(word & 0xFFFFu, word >> 16u, (elem & 1u) == 1u);
            let w = bitcast<f32>(bits16 << 16u);
            acc = acc + w * input[in_base + j];
            j = j + COOP_WG;
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        // SoA Q4_K single-row: each thread owns lane `t` of every superblock.
        // Activation is one f32 per thread per block → load from global, **no LDS, no
        // barrier** in the FMA loop. Full-act LDS (16 KiB) was A/B'd and lost occupancy
        // on A2000 (~8.5 vs ~9.1 tok/s). Multi-row still uses shared act tiles separately.
        let row_base = row * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let sub = t / 32u;
        let group = t / 64u;
        let local = t % 64u;
        let scale_pair = sub >> 1u;
        let scale_hi = (sub & 1u) == 1u;
        let q_off = group * 32u;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let block_base = row_base + b * BLOCK_Q4K_SOA_BYTES;
            let d_word = weight_words[(block_base + 128u) / 4u + scale_pair];
            let m_word = weight_words[(block_base + 144u) / 4u + scale_pair];
            let dsub = f16_to_f32(select(d_word & 0xFFFFu, d_word >> 16u, scale_hi));
            let msub = f16_to_f32(select(m_word & 0xFFFFu, m_word >> 16u, scale_hi));
            var nib: u32;
            if local < 32u {
                let byte_i = block_base + q_off + local;
                nib = (weight_words[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFu;
            } else {
                let byte_i = block_base + q_off + (local - 32u);
                nib = ((weight_words[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            let x = input[in_base + b * BLOCK_Q4K_ELEMS + t];
            acc = acc + (dsub * f32(nib) - msub) * x;
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K && (params.n_in % BLOCK_Q4K_ELEMS) == 0u {
        // Block-cooperative Q4_K + ping-pong act; header still 8-thread decode.
        let row_base = row * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let sub = t / 32u;
        let group = t / 64u;
        let local = t % 64u;
        if n_blocks > 0u {
            coop_act[t] = input[in_base + t];
            if t < 8u {
                let block_base = row_base;
                let d_word = weight_words[block_base >> 2u];
                let d = f16_to_f32(d_word & 0xFFFFu);
                let dmin = f16_to_f32(d_word >> 16u);
                let sm = get_scale_min_k4(t, block_base + 4u);
                coop_q4k_dsub[t] = d * f32(sm.x);
                coop_q4k_msub[t] = dmin * f32(sm.y);
            }
        }
        workgroupBarrier();
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let use_a = (b & 1u) == 0u;
            let slot = (b & 1u) * 8u;
            let next_slot = ((b + 1u) & 1u) * 8u;
            if b + 1u < n_blocks {
                let nxt = input[in_base + (b + 1u) * BLOCK_Q4K_ELEMS + t];
                if use_a {
                    coop_act_b[t] = nxt;
                } else {
                    coop_act[t] = nxt;
                }
                if t < 8u {
                    let nb = row_base + (b + 1u) * BLOCK_Q4K_BYTES;
                    let d_word = weight_words[nb >> 2u];
                    let d = f16_to_f32(d_word & 0xFFFFu);
                    let dmin = f16_to_f32(d_word >> 16u);
                    let sm = get_scale_min_k4(t, nb + 4u);
                    coop_q4k_dsub[next_slot + t] = d * f32(sm.x);
                    coop_q4k_msub[next_slot + t] = dmin * f32(sm.y);
                }
            }
            let block_base = row_base + b * BLOCK_Q4K_BYTES;
            let qs_base = block_base + 16u;
            let q_off = group * 32u;
            var nib: u32;
            if local < 32u {
                let byte_i = qs_base + q_off + local;
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = (word >> shift) & 0xFu;
            } else {
                let byte_i = qs_base + q_off + (local - 32u);
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = ((word >> shift) & 0xFFu) >> 4u;
            }
            let x = select(coop_act_b[t], coop_act[t], use_a);
            let w = coop_q4k_dsub[slot + sub] * f32(nib) - coop_q4k_msub[slot + sub];
            acc = acc + w * x;
            workgroupBarrier();
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q6_K
        && (params.n_in % BLOCK_Q6K_ELEMS) == 0u
    {
        // Q6_K block-coop (logits): one element per thread per superblock + shared act.
        let row_base = row * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q6K_ELEMS;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            coop_act[t] = input[in_base + b * BLOCK_Q6K_ELEMS + t];
            workgroupBarrier();
            let col = b * BLOCK_Q6K_ELEMS + t;
            acc = acc + dequant_q6_k_weight(row, col) * coop_act[t];
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
    return acc;
}

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

    coop_partial[t] = coop_row_dot(row, t, in_base);
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

// Warp GEMV (32 threads/row): each lane owns 8 columns per 256-block → ~8× more
// FMA/thread than 256-wide coop, and reduce is only 5 steps (or subgroupAdd).
// Preferred for Q4_K_SOA decode on discrete GPUs (dispatch still n_out WGs).
const WARP_WG: u32 = 32u;

fn warp_q4soa_row_dot(row: u32, t: u32, in_base: u32) -> f32 {
    var acc = 0.0;
    let row_base = row * weight_row_bytes();
    let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
    for (var b = 0u; b < n_blocks; b = b + 1u) {
        let block_base = row_base + b * BLOCK_Q4K_SOA_BYTES;
        // 8 columns per thread within the 256-elem superblock.
        for (var k = 0u; k < 8u; k = k + 1u) {
            let local_col = t + k * WARP_WG;
            let sub = local_col / 32u; // = k
            let group = local_col / 64u;
            let local = local_col % 64u;
            let scale_pair = sub >> 1u;
            let scale_hi = (sub & 1u) == 1u;
            let d_word = weight_words[(block_base + 128u) / 4u + scale_pair];
            let m_word = weight_words[(block_base + 144u) / 4u + scale_pair];
            let d_bits = select(d_word & 0xFFFFu, d_word >> 16u, scale_hi);
            let m_bits = select(m_word & 0xFFFFu, m_word >> 16u, scale_hi);
            let dsub = f16_to_f32(d_bits);
            let msub = f16_to_f32(m_bits);
            let q_off = group * 32u;
            var nib: u32;
            if local < 32u {
                let byte_i = block_base + q_off + local;
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = (word >> shift) & 0xFu;
            } else {
                let byte_i = block_base + q_off + (local - 32u);
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = ((word >> shift) & 0xFFu) >> 4u;
            }
            let x = input[in_base + b * BLOCK_Q4K_ELEMS + local_col];
            acc = acc + (dsub * f32(nib) - msub) * x;
        }
    }
    return acc;
}

fn warp_tree_reduce_32(t: u32) -> f32 {
    // coop_partial[0..32) holds lane partials; returns sum (valid on all lanes after).
    workgroupBarrier();
    if t < 16u { coop_partial[t] = coop_partial[t] + coop_partial[t + 16u]; }
    workgroupBarrier();
    if t < 8u { coop_partial[t] = coop_partial[t] + coop_partial[t + 8u]; }
    workgroupBarrier();
    if t < 4u { coop_partial[t] = coop_partial[t] + coop_partial[t + 4u]; }
    workgroupBarrier();
    if t < 2u { coop_partial[t] = coop_partial[t] + coop_partial[t + 2u]; }
    workgroupBarrier();
    if t < 1u { coop_partial[t] = coop_partial[t] + coop_partial[t + 1u]; }
    workgroupBarrier();
    return coop_partial[0];
}

@compute @workgroup_size(32)
fn coop_gemv_warp(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let row = wg_id.x;
    if row >= params.n_out { return; }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    var acc = 0.0;
    if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        acc = warp_q4soa_row_dot(row, t, in_base);
    } else {
        // Generic: stride-32 over n_in.
        var j = t;
        loop {
            if j >= params.n_in { break; }
            acc = acc + dequant_weight(row, j) * input[in_base + j];
            j = j + WARP_WG;
        }
    }
    coop_partial[t] = acc;
    let total = warp_tree_reduce_32(t);
    if t == 0u {
        output[out_base + row] = total;
    }
}

// [WASM] coop_gemv_residual_warp and coop_gemv_residual stripped — require binding 4 (residual).

// ─────────────────────────────────────────────────────────────────────────────
// Multi-row cooperative GEMV (dramatic 3B lever).
//
// Single-row `coop_gemv` launches n_out workgroups; each reloads the full
// activation from global memory. For Llama-3.2-3B FFN (n_out=8192, n_in=3072)
// that is ~100 MB of *repeated* act traffic per gate/up alone — 8× the weight
// stream. Multi-row packs COOP_ROWS consecutive outputs into one WG:
//
//   1. Cooperative load of the full activation into LDS (once).
//   2. Barrier-free Q4_K(_SOA) per-block FMA (each thread only needs its lane).
//   3. Tree-reduce + write for each of the R rows, reusing the LDS act.
//
// Dispatch: (ceil(n_out / COOP_ROWS), batch, 1). Same group-0 bindings as coop_gemv.
// Falls back to sequential single-row `coop_row_dot` when n_in > COOP_FULL_ACT_MAX.
const COOP_ROWS: u32 = 8u;

// Dot product against LDS-resident activation (no global act reload, no per-block barrier).
// Assumes `coop_full_act` already holds the activation (loaded by caller).
fn coop_row_dot_lds(row: u32, t: u32) -> f32 {
    var acc = 0.0;
    if params.weight_ggml_type == GGML_TYPE_F16 {
        let row_base = row * params.weight_row_elems;
        var j = t;
        loop {
            if j >= params.n_in { break; }
            let elem = row_base + j;
            let pair = unpack2x16float(weight_words[elem >> 1u]);
            let w = select(pair.x, pair.y, (elem & 1u) == 1u);
            acc = acc + w * coop_full_act[j];
            j = j + COOP_WG;
        }
    } else if params.weight_ggml_type == GGML_TYPE_BF16 {
        let row_base = row * params.weight_row_elems;
        var j = t;
        loop {
            if j >= params.n_in { break; }
            let elem = row_base + j;
            let word = weight_words[elem >> 1u];
            let bits16 = select(word & 0xFFFFu, word >> 16u, (elem & 1u) == 1u);
            let w = bitcast<f32>(bits16 << 16u);
            acc = acc + w * coop_full_act[j];
            j = j + COOP_WG;
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        let row_base = row * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let sub = t / 32u;
        let group = t / 64u;
        let local = t % 64u;
        let scale_pair = sub >> 1u;
        let scale_hi = (sub & 1u) == 1u;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let block_base = row_base + b * BLOCK_Q4K_SOA_BYTES;
            let d_word = weight_words[(block_base + 128u) / 4u + scale_pair];
            let m_word = weight_words[(block_base + 144u) / 4u + scale_pair];
            let d_bits = select(d_word & 0xFFFFu, d_word >> 16u, scale_hi);
            let m_bits = select(m_word & 0xFFFFu, m_word >> 16u, scale_hi);
            let dsub = f16_to_f32(d_bits);
            let msub = f16_to_f32(m_bits);
            let qs_base = block_base;
            let q_off = group * 32u;
            var nib: u32;
            if local < 32u {
                let byte_i = qs_base + q_off + local;
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = (word >> shift) & 0xFu;
            } else {
                let byte_i = qs_base + q_off + (local - 32u);
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = ((word >> shift) & 0xFFu) >> 4u;
            }
            let x = coop_full_act[b * BLOCK_Q4K_ELEMS + t];
            acc = acc + (dsub * f32(nib) - msub) * x;
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        // Header decode per block into registers (thread-private); no shared header needed
        // because each thread's sub-scale is constant for its lane within a block.
        let row_base = row * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let sub = t / 32u;
        let group = t / 64u;
        let local = t % 64u;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let block_base = row_base + b * BLOCK_Q4K_BYTES;
            let d_word = weight_words[block_base >> 2u];
            let d = f16_to_f32(d_word & 0xFFFFu);
            let dmin = f16_to_f32(d_word >> 16u);
            let sm = get_scale_min_k4(sub, block_base + 4u);
            let dsub = d * f32(sm.x);
            let msub = dmin * f32(sm.y);
            let qs_base = block_base + 16u;
            let q_off = group * 32u;
            var nib: u32;
            if local < 32u {
                let byte_i = qs_base + q_off + local;
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = (word >> shift) & 0xFu;
            } else {
                let byte_i = qs_base + q_off + (local - 32u);
                let word = weight_words[byte_i >> 2u];
                let shift = (byte_i & 3u) * 8u;
                nib = ((word >> shift) & 0xFFu) >> 4u;
            }
            let x = coop_full_act[b * BLOCK_Q4K_ELEMS + t];
            acc = acc + (dsub * f32(nib) - msub) * x;
        }
    } else {
        var j = t;
        loop {
            if j >= params.n_in { break; }
            acc = acc + dequant_weight(row, j) * coop_full_act[j];
            j = j + COOP_WG;
        }
    }
    return acc;
}

fn coop_tree_reduce_write(t: u32, out_idx: u32, resid: f32) {
    // Assumes coop_partial[t] already holds the lane partial.
    workgroupBarrier();
    var stride = COOP_WG >> 1u;
    loop {
        if stride == 0u { break; }
        if t < stride {
            coop_partial[t] = coop_partial[t] + coop_partial[t + stride];
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if t == 0u {
        output[out_idx] = resid + coop_partial[0];
    }
}

// One K-sweep for COOP_ROWS rows (Q4_K_SOA, n_in multiple of 256, n_in ≤ 4096).
// Returns false if the fast path does not apply (caller uses serial fallback).
fn coop_mr_q4soa_fused_accum(
    row0: u32,
    t: u32,
    in_base: u32,
    acc: ptr<function, array<f32, 8>>,
) -> bool {
    // Multi-row with 256-elem act tile only (low LDS → high occupancy).
    // Each superblock: load act once, FMA into all COOP_ROWS (weight reuse).
    // Works for any n_in multiple of 256 (including down proj n_in=8192).
    if params.weight_ggml_type != GGML_TYPE_Q4_K_SOA {
        return false;
    }
    if (params.n_in % BLOCK_Q4K_ELEMS) != 0u || params.n_in == 0u {
        return false;
    }
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;
    let scale_pair = sub >> 1u;
    let scale_hi = (sub & 1u) == 1u;
    let q_off = group * 32u;
    let rb = weight_row_bytes();
    let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
    for (var r = 0u; r < COOP_ROWS; r = r + 1u) {
        (*acc)[r] = 0.0;
    }
    for (var b = 0u; b < n_blocks; b = b + 1u) {
        coop_act[t] = input[in_base + b * BLOCK_Q4K_ELEMS + t];
        workgroupBarrier();
        let x = coop_act[t];
        let col_base = b * BLOCK_Q4K_SOA_BYTES;
        for (var r = 0u; r < COOP_ROWS; r = r + 1u) {
            let row = row0 + r;
            if row >= params.n_out {
                continue;
            }
            let block_base = row * rb + col_base;
            let d_word = weight_words[(block_base + 128u) / 4u + scale_pair];
            let m_word = weight_words[(block_base + 144u) / 4u + scale_pair];
            let dsub = f16_to_f32(select(d_word & 0xFFFFu, d_word >> 16u, scale_hi));
            let msub = f16_to_f32(select(m_word & 0xFFFFu, m_word >> 16u, scale_hi));
            var nib: u32;
            if local < 32u {
                let byte_i = block_base + q_off + local;
                nib = (weight_words[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFu;
            } else {
                let byte_i = block_base + q_off + (local - 32u);
                nib = ((weight_words[byte_i >> 2u] >> ((byte_i & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            (*acc)[r] = (*acc)[r] + (dsub * f32(nib) - msub) * x;
        }
        workgroupBarrier();
    }
    return true;
}

fn coop_mr_reduce_write_rows(
    t: u32,
    row0: u32,
    out_base: u32,
    acc: ptr<function, array<f32, 8>>,
) {
    // Parallel multi-accumulator tree: one barrier ladder for all COOP_ROWS.
    for (var r = 0u; r < COOP_ROWS; r = r + 1u) {
        coop_full_act[r * COOP_WG + t] = (*acc)[r];
    }
    workgroupBarrier();
    var stride = COOP_WG >> 1u;
    loop {
        if stride == 0u {
            break;
        }
        if t < stride {
            for (var r = 0u; r < COOP_ROWS; r = r + 1u) {
                let base = r * COOP_WG;
                coop_full_act[base + t] =
                    coop_full_act[base + t] + coop_full_act[base + t + stride];
            }
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if t < COOP_ROWS {
        let row = row0 + t;
        if row < params.n_out {
            output[out_base + row] = coop_full_act[t * COOP_WG];
        }
    }
}

@compute @workgroup_size(256)
fn coop_gemv_mr(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let row0 = wg_id.x * COOP_ROWS;
    if row0 >= params.n_out { return; }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    var acc: array<f32, 8>;
    // Q4_K_SOA tiled multi-row (owns act loads; works for n_in > 4096).
    if coop_mr_q4soa_fused_accum(row0, t, in_base, &acc) {
        coop_mr_reduce_write_rows(t, row0, out_base, &acc);
        return;
    }
    // Other quants: load act once when it fits, else serial coop_row_dot.
    let use_full = params.n_in <= COOP_FULL_ACT_MAX && params.n_in > 0u;
    if use_full {
        var j = t;
        loop {
            if j >= params.n_in { break; }
            coop_full_act[j] = input[in_base + j];
            j = j + COOP_WG;
        }
        workgroupBarrier();
        for (var r = 0u; r < COOP_ROWS; r = r + 1u) {
            acc[r] = select(0.0, coop_row_dot_lds(row0 + r, t), row0 + r < params.n_out);
        }
        coop_mr_reduce_write_rows(t, row0, out_base, &acc);
    } else {
        for (var r = 0u; r < COOP_ROWS; r = r + 1u) {
            let row = row0 + r;
            acc[r] = select(0.0, coop_row_dot(row, t, in_base), row < params.n_out);
        }
        coop_mr_reduce_write_rows(t, row0, out_base, &acc);
    }
}

// [WASM] coop_gemv_residual_mr stripped — requires binding 4 (residual).

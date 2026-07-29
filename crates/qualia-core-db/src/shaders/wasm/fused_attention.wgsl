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
    rope_scale: f32,
    num_tokens_in_batch: u32, // 1 during decode; >1 during chunked prefill
    batch_start_token_idx: u32,
    mask_active: u32, // 0 = dense KV; 1 = graph-guided sparsity (U1 bitmask)
    mask_word_count: u32,
    out_stride_elems: u32, // batched Q row stride (floats); 0 = contiguous in binding
    // Phase 5.5: row stride (floats/token) of a PRE-COMPUTED Q/K/V projection in `hidden` (binding 0).
    // Non-zero → read the projection directly (the parallel GEMM already did the matmul); 0 → legacy
    // in-shader matmul via gemm_row. Decouples the heavy projection from this @workgroup_size(1) kernel.
    proj_row_stride: u32,
    kv_quant: u32,   // W5a: 1 ⇒ int8 KV cache (packed i8 + f32 scale); 0 ⇒ legacy f32
    // W5b Phase 4b: dict-coded KV. `dict_pack` = dict_k (low 16) | n_atoms (high 16); dict_k>0 ⇒ store
    // k-sparse codes (u16 atom-index | f16 coeff per word) reconstructed from `kv_atoms`. 0 ⇒ f32/int8.
    dict_pack: u32,
}

@group(0) @binding(0) var<storage, read> hidden: array<f32>;
@group(0) @binding(1) var<storage, read> weight_words: array<u32>;
@group(0) @binding(2) var<uniform> params: AttentionParams;
var<private> q_mask_token: u32;
@group(0) @binding(3) var<storage, read_write> kv_cache: array<f32>;
@group(0) @binding(4) var<storage, read_write> attn_out: array<f32>;
@group(0) @binding(5) var<storage, read> kv_mask_words: array<u32>;

// Dict-encode workgroup scratch (Phase 4b, write path). Sized for n_atoms ≤ 512, k ≤ 8.
const MAX_ATOMS: u32 = 512u;
const MAX_K: u32 = 8u;
var<workgroup> enc_residual: array<f32, MAX_HEAD_DIM>;
var<workgroup> enc_corr: array<f32, MAX_ATOMS>;
var<workgroup> enc_sel: array<u32, MAX_K>;
var<workgroup> enc_coeff: array<f32, MAX_K>;
var<workgroup> enc_gram: array<f32, 64>; // MAX_K × MAX_K
var<workgroup> enc_rhs: array<f32, MAX_K>;

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
const GGML_TYPE_F16: u32 = 1u;
const GGML_TYPE_BF16: u32 = 30u;
const GGML_TYPE_Q4_0: u32 = 2u;
const GGML_TYPE_Q5_0: u32 = 6u;
const GGML_TYPE_Q8_0: u32 = 8u;
const GGML_TYPE_Q4_K: u32 = 12u;
const GGML_TYPE_Q4_K_SOA: u32 = 112u;
const GGML_TYPE_Q6_K: u32 = 14u;
const MAX_HEAD_DIM: u32 = 512u;
const NEG_INF: f32 = -1e30;

// FlashAttention-style key-parallelism: one workgroup per Q head (decode) or per (token,kv_head)
// (K/V write), WG_SIZE threads cooperating over the context positions. 64 is warp/wavefront friendly
// (NV=32, AMD=64). The cross-thread online-softmax merge runs in MERGE_TILE-wide columns to bound
// barriers and shared memory (Limits::default() → 16 KiB workgroup storage).
const WG_SIZE: u32 = 64u;
const MERGE_TILE: u32 = 16u;
// WG_SIZE * MERGE_TILE — kept as a literal because this naga build rejects arithmetic in array sizes.
const MERGE_SCRATCH: u32 = 1024u;

// Q vector (RoPE-applied), shared across the workgroup so every thread reads the same projection.
var<workgroup> q_sh: array<f32, MAX_HEAD_DIM>;
// Scalar online-softmax reduction scratch (per-thread running max / normaliser).
var<workgroup> red_m: array<f32, WG_SIZE>;
var<workgroup> red_l: array<f32, WG_SIZE>;
// Weighted-V accumulator merge scratch: MERGE_TILE columns × WG_SIZE lanes (column-major).
var<workgroup> red_acc: array<f32, MERGE_SCRATCH>;

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
    if params.weight_ggml_type == GGML_TYPE_F16
        || params.weight_ggml_type == GGML_TYPE_BF16
    {
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
// Mirrors fused_transformer.wgsl::dequant_f16_weight (verified == CPU dequant_f16 in W3).
fn dequant_f16_weight(row: u32, col: u32) -> f32 {
    let elem = row * params.weight_row_elems + col; // linear half index
    let pair = unpack2x16float(weight_words[elem >> 1u]);
    return select(pair.x, pair.y, (elem & 1u) == 1u);
}

// bf16 (Gemma-4 norms / residual scales): promote by shifting into f32 high half.
// Mirrors fused_transformer.wgsl::dequant_bf16_weight.
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

// Convert-time SoA Q4_K: qs@0, d_sub f16[8]@128, m_sub f16[8]@144.
fn dequant_q4_k_soa_weight(row: u32, col: u32) -> f32 {
    let row_bytes = weight_row_bytes();
    let block = col / BLOCK_Q4K_ELEMS;
    let elem = col % BLOCK_Q4K_ELEMS;
    let block_base = row * row_bytes + block * BLOCK_Q4K_SOA_BYTES;
    let sub = elem / 32u;
    let group = elem / 64u;
    let local = elem % 64u;
    let d_off = block_base + 128u + sub * 2u;
    let m_off = block_base + 144u + sub * 2u;
    let dsub = f16_to_f32(read_u8_weight(d_off) | (read_u8_weight(d_off + 1u) << 8u));
    let msub = f16_to_f32(read_u8_weight(m_off) | (read_u8_weight(m_off + 1u) << 8u));
    let qs_base = block_base;
    let q_off = group * 32u;
    var nib: u32;
    if local < 32u {
        nib = read_u8_weight(qs_base + q_off + local) & 0xFu;
    } else {
        nib = (read_u8_weight(qs_base + q_off + (local - 32u)) >> 4u) & 0xFu;
    }
    return dsub * f32(nib) - msub;
}

fn gemm_row(row: u32, token_in_batch: u32) -> f32 {
    // Phase 5.5 — projection decoupling: when proj_row_stride != 0 the Q/K/V projection was already
    // computed by the parallel fused_transformer.wgsl GEMM (saturating the GPU) and bound here as
    // `hidden`. Read it directly instead of re-doing the matmul serially on this 1-thread workgroup.
    if params.proj_row_stride != 0u {
        return hidden[token_in_batch * params.proj_row_stride + row];
    }
    let h_base = token_in_batch * params.n_embd;
    var sum = 0.0;
    for (var j = 0u; j < params.n_embd; j = j + 1u) {
        sum = sum + dequant_weight(row, j) * hidden[h_base + j];
    }
    return sum;
}

// Interleaved ("normal"/llama) RoPE: rotate adjacent pairs (2i, 2i+1) — GGUF llama convention
// (is_neox=false), matches CPU rope_inplace / SmolLM2. (Name kept for history; this is NOT NEOX.)
fn apply_rope_neox(vec: ptr<function, array<f32, MAX_HEAD_DIM>>, abs_pos: u32) {
    let half_dim = params.head_dim / 2u;
    let scale = select(1.0, params.rope_scale, params.rope_scale > 0.0);
    let pos = f32(abs_pos) / scale;
    for (var i = 0u; i < half_dim; i = i + 1u) {
        let theta = pos * pow(params.rope_theta_base, -2.0 * f32(i) / f32(params.head_dim));
        let cos_t = cos(theta);
        let sin_t = sin(theta);
        let idx_0 = 2u * i;
        let idx_1 = 2u * i + 1u;
        let v0 = (*vec)[idx_0];
        let v1 = (*vec)[idx_1];
        (*vec)[idx_0] = v0 * cos_t - v1 * sin_t;
        (*vec)[idx_1] = v0 * sin_t + v1 * cos_t;
    }
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

// ── W5a int8 KV (kv_quant==1) ──────────────────────────────────────────────────────────────────
// Same binding-3 arena reinterpreted: per slot (4-byte elems) = [K scales: n_kv_head f32]
// [V scales: n_kv_head f32] [K data: n_kv_head·(head_dim/4) packed-i8 words] [V data: same]. Each
// element is quantized as round(x / scale) with scale = amax(head)/127; dequant = i8 · scale.
fn kv8_slot_stride() -> u32 {
    return 2u * params.n_kv_head * (1u + params.head_dim / 4u);
}
fn kv8_k_scale_idx(slot: u32, kv_head: u32) -> u32 {
    return slot * kv8_slot_stride() + kv_head;
}
fn kv8_v_scale_idx(slot: u32, kv_head: u32) -> u32 {
    return slot * kv8_slot_stride() + params.n_kv_head + kv_head;
}
fn kv8_k_data_idx(slot: u32, kv_head: u32, word: u32) -> u32 {
    let hd4 = params.head_dim / 4u;
    return slot * kv8_slot_stride() + 2u * params.n_kv_head + kv_head * hd4 + word;
}
fn kv8_v_data_idx(slot: u32, kv_head: u32, word: u32) -> u32 {
    let hd4 = params.head_dim / 4u;
    return slot * kv8_slot_stride() + 2u * params.n_kv_head + params.n_kv_head * hd4 + kv_head * hd4 + word;
}
// Sign-extend one i8 lane out of a packed u32 word.
fn i8_lane(packed: u32, lane: u32) -> f32 {
    let b = (packed >> (lane * 8u)) & 0xFFu;
    return f32(select(i32(b), i32(b) - 256, b >= 128u));
}

// ── W5b Phase 4b dict-coded KV ─────────────────────────────────────────────────────────────────
fn dict_k_val() -> u32 { return params.dict_pack & 0xFFFFu; }
fn dict_n_atoms() -> u32 { return params.dict_pack >> 16u; }

// Layer-local code-word offset (mirrors Rust `KvCacheLayout::code_index`; the binding is one layer
// slice, so no layer term). `is_v`: 0 = K region, 1 = V region.
fn code_idx(slot: u32, kv_head: u32, is_v: u32, i: u32) -> u32 {
    let dk = dict_k_val();
    let per_slot = 2u * params.n_kv_head * dk;
    let stream_off = select(0u, params.n_kv_head * dk, is_v == 1u);
    return slot * per_slot + stream_off + kv_head * dk + i;
}
// Atom element: the dictionary atoms live in the KV arena AFTER the per-layer code region (the
// binding is already this layer's slice), as `[K atoms n_atoms×hd][V atoms n_atoms×hd]`.
fn atom_elem(is_v: u32, a: u32, d: u32) -> f32 {
    let na = dict_n_atoms();
    let atoms_base = params.max_context * 2u * params.n_kv_head * dict_k_val(); // per-layer code region
    let stream_off = select(0u, na * params.head_dim, is_v == 1u);
    return kv_cache[atoms_base + stream_off + a * params.head_dim + d];
}
// Pack (atom_index, coeff) into a code word: u16 index (high) | f16 coeff (low).
fn pack_code(index: u32, coeff: f32) -> f32 {
    let f16bits = pack2x16float(vec2<f32>(coeff, 0.0)) & 0xFFFFu;
    return bitcast<f32>((index << 16u) | f16bits);
}
// Reconstruct element `d` of the K (is_v=0) or V (is_v=1) vector at (slot, kv_head) from its codes.
fn recon_elem(slot: u32, kv_head: u32, is_v: u32, d: u32) -> f32 {
    let dk = dict_k_val();
    var acc = 0.0;
    for (var i = 0u; i < dk; i = i + 1u) {
        let word = bitcast<u32>(kv_cache[code_idx(slot, kv_head, is_v, i)]);
        let coeff = unpack2x16float(word & 0xFFFFu).x;
        let idx = word >> 16u;
        if coeff != 0.0 {
            acc = acc + coeff * atom_elem(is_v, idx, d);
        }
    }
    return acc;
}

// Read one K element: dict-reconstruct, else int8-dequant, else raw f32 (f32 path bit-identical).
fn read_k(slot: u32, kv_head: u32, d: u32) -> f32 {
    if dict_k_val() > 0u {
        return recon_elem(slot, kv_head, 0u, d);
    }
    if params.kv_quant == 0u {
        return kv_cache[k_cache_idx(slot, kv_head, d)];
    }
    let scale = kv_cache[kv8_k_scale_idx(slot, kv_head)];
    let packed = bitcast<u32>(kv_cache[kv8_k_data_idx(slot, kv_head, d / 4u)]);
    return i8_lane(packed, d % 4u) * scale;
}
fn read_v(slot: u32, kv_head: u32, d: u32) -> f32 {
    if dict_k_val() > 0u {
        return recon_elem(slot, kv_head, 1u, d);
    }
    if params.kv_quant == 0u {
        return kv_cache[v_cache_idx(slot, kv_head, d)];
    }
    let scale = kv_cache[kv8_v_scale_idx(slot, kv_head)];
    let packed = bitcast<u32>(kv_cache[kv8_v_data_idx(slot, kv_head, d / 4u)]);
    return i8_lane(packed, d % 4u) * scale;
}

fn kv_slot_allowed(logical: u32) -> bool {
    if params.mask_active == 0u {
        return true;
    }
    let bit = logical;
    let word = bit / 32u;
    if word >= params.mask_word_count {
        return false;
    }
    let offset = bit % 32u;
    let mask_base = select(0u, q_mask_token * params.mask_word_count, params.num_tokens_in_batch > 1u);
    return (kv_mask_words[mask_base + word] & (1u << offset)) != 0u;
}

// Q+attn, key-parallel: WG_SIZE threads split the context positions, each keeping a PRIVATE
// online-softmax state (m_t, l_t, acc_t[]); a cross-thread merge combines them. abs position is per
// batch row (decode: batch=1 → batch_start + 0). All threads of the workgroup must reach every
// workgroupBarrier() — the early guards in `main` are uniform across the workgroup (derived from
// wg_id), and the per-thread position loop contains no barriers.
fn attention_parallel(qh: u32, kv_head: u32, token_in_batch: u32, lid: u32) {
    let row_base = qh * params.head_dim;
    let abs_pos = params.batch_start_token_idx + token_in_batch;

    // 1. Cooperatively project Q into shared, then apply interleaved (2i,2i+1) RoPE in shared.
    //    GGUF llama-arch (SmolLM2) uses the "normal"/interleaved rope (is_neox=false), NOT split-half.
    for (var d = lid; d < params.head_dim; d = d + WG_SIZE) {
        q_sh[d] = gemm_row(row_base + d, token_in_batch);
    }
    workgroupBarrier();
    let half_dim = params.head_dim / 2u;
    let rscale = select(1.0, params.rope_scale, params.rope_scale > 0.0);
    let rpos = f32(abs_pos) / rscale;
    for (var i = lid; i < half_dim; i = i + WG_SIZE) {
        let theta = rpos * pow(params.rope_theta_base, -2.0 * f32(i) / f32(params.head_dim));
        let cos_t = cos(theta);
        let sin_t = sin(theta);
        let v0 = q_sh[2u * i];
        let v1 = q_sh[2u * i + 1u];
        q_sh[2u * i] = v0 * cos_t - v1 * sin_t;
        q_sh[2u * i + 1u] = v0 * sin_t + v1 * cos_t;
    }
    workgroupBarrier();

    // 2. Per-thread partial online softmax over a strided slice of the context positions.
    var m_t = NEG_INF;
    var l_t = 0.0;
    var acc_t: array<f32, MAX_HEAD_DIM>;
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        acc_t[d] = 0.0;
    }
    let seq_len = abs_pos + 1u;
    let start = select(0u, seq_len - params.max_context, seq_len > params.max_context);
    let scale = 1.0 / sqrt(f32(params.head_dim));
    for (var logical = start + lid; logical <= abs_pos; logical = logical + WG_SIZE) {
        if !kv_slot_allowed(logical) {
            continue;
        }
        let slot = logical % params.max_context;
        var score = 0.0;
        if params.kv_quant == 1u {
            // int8 fast read: hoist the per-head scale and load each packed word ONCE (all 4 lanes),
            // instead of reloading the scale every element and each word 4× via `read_k`. Attention is
            // memory-bound, so this cuts the KV loads ~4× on the (default) int8 path. Bit-identical:
            // the same `q * (i8 * scale)` products are summed in the same d-order.
            let kscale = kv_cache[kv8_k_scale_idx(slot, kv_head)];
            let hd4 = params.head_dim / 4u;
            for (var wi = 0u; wi < hd4; wi = wi + 1u) {
                let packed = bitcast<u32>(kv_cache[kv8_k_data_idx(slot, kv_head, wi)]);
                let base = wi * 4u;
                score = score + q_sh[base] * (i8_lane(packed, 0u) * kscale);
                score = score + q_sh[base + 1u] * (i8_lane(packed, 1u) * kscale);
                score = score + q_sh[base + 2u] * (i8_lane(packed, 2u) * kscale);
                score = score + q_sh[base + 3u] * (i8_lane(packed, 3u) * kscale);
            }
        } else {
            for (var d = 0u; d < params.head_dim; d = d + 1u) {
                score = score + q_sh[d] * read_k(slot, kv_head, d);
            }
        }
        score = score * scale;
        let m_new = max(m_t, score);
        let w = exp(score - m_new);
        let factor = exp(m_t - m_new);
        m_t = m_new;
        l_t = l_t * factor + w;
        if params.kv_quant == 1u {
            // int8 fast read (mirror of the K path above): hoist the V scale + read each word once.
            let vscale = kv_cache[kv8_v_scale_idx(slot, kv_head)];
            let hd4 = params.head_dim / 4u;
            for (var wi = 0u; wi < hd4; wi = wi + 1u) {
                let packed = bitcast<u32>(kv_cache[kv8_v_data_idx(slot, kv_head, wi)]);
                let base = wi * 4u;
                acc_t[base] = acc_t[base] * factor + w * (i8_lane(packed, 0u) * vscale);
                acc_t[base + 1u] = acc_t[base + 1u] * factor + w * (i8_lane(packed, 1u) * vscale);
                acc_t[base + 2u] = acc_t[base + 2u] * factor + w * (i8_lane(packed, 2u) * vscale);
                acc_t[base + 3u] = acc_t[base + 3u] * factor + w * (i8_lane(packed, 3u) * vscale);
            }
        } else {
            for (var d = 0u; d < params.head_dim; d = d + 1u) {
                acc_t[d] = acc_t[d] * factor + w * read_v(slot, kv_head, d);
            }
        }
    }

    // 3a. Reduce the global max m across threads (tree reduction).
    red_m[lid] = m_t;
    workgroupBarrier();
    for (var s = WG_SIZE / 2u; s > 0u; s = s >> 1u) {
        if lid < s {
            red_m[lid] = max(red_m[lid], red_m[lid + s]);
        }
        workgroupBarrier();
    }
    let m = red_m[0];

    // 3b. Rescale each thread's normaliser to the global max, then reduce the global l.
    let factor_t = select(0.0, exp(m_t - m), m > NEG_INF);
    red_l[lid] = l_t * factor_t;
    workgroupBarrier();
    for (var s = WG_SIZE / 2u; s > 0u; s = s >> 1u) {
        if lid < s {
            red_l[lid] = red_l[lid] + red_l[lid + s];
        }
        workgroupBarrier();
    }
    let l = red_l[0];
    let inv = select(0.0, 1.0 / l, l > 0.0);

    // 3c. Merge the weighted-V accumulators in MERGE_TILE-wide columns: each thread contributes
    // factor_t * acc_t[d]; a per-column tree reduction sums them; thread d writes the normalised out.
    let out_base = params.out_stride_elems * token_in_batch;
    for (var d0 = 0u; d0 < params.head_dim; d0 = d0 + MERGE_TILE) {
        for (var c = 0u; c < MERGE_TILE; c = c + 1u) {
            let d = d0 + c;
            red_acc[c * WG_SIZE + lid] = select(0.0, acc_t[d] * factor_t, d < params.head_dim);
        }
        workgroupBarrier();
        for (var s = WG_SIZE / 2u; s > 0u; s = s >> 1u) {
            if lid < s {
                for (var c = 0u; c < MERGE_TILE; c = c + 1u) {
                    red_acc[c * WG_SIZE + lid] = red_acc[c * WG_SIZE + lid] + red_acc[c * WG_SIZE + lid + s];
                }
            }
            workgroupBarrier();
        }
        if lid < MERGE_TILE {
            let d = d0 + lid;
            if d < params.head_dim {
                attn_out[out_base + row_base + d] = red_acc[lid * WG_SIZE] * inv;
            }
        }
        workgroupBarrier();
    }
}

// K/V projection write, key-parallel: the WG_SIZE threads split the head_dim outputs, each computing
// its strided gemm_row (a full dequant-dot over n_embd when proj_row_stride==0 — the in-shader
// projection that dominated decode when run on a single lane). Reuses q_sh as the shared K/V vector.
// Barriers are uniform: proj_kind / apply_rope_k come from the uniform buffer, so the whole workgroup
// takes the same branch.
fn write_kv_head(kv_head: u32, token_in_batch: u32, abs_pos: u32, apply_rope_k: bool, lid: u32) {
    let row_base = kv_head * params.head_dim;
    for (var d = lid; d < params.head_dim; d = d + WG_SIZE) {
        q_sh[d] = gemm_row(row_base + d, token_in_batch);
    }
    workgroupBarrier();
    if apply_rope_k {
        let half_dim = params.head_dim / 2u;
        let rscale = select(1.0, params.rope_scale, params.rope_scale > 0.0);
        let rpos = f32(abs_pos) / rscale;
        for (var i = lid; i < half_dim; i = i + WG_SIZE) {
            let theta = rpos * pow(params.rope_theta_base, -2.0 * f32(i) / f32(params.head_dim));
            let cos_t = cos(theta);
            let sin_t = sin(theta);
            let v0 = q_sh[2u * i];
            let v1 = q_sh[2u * i + 1u];
            q_sh[2u * i] = v0 * cos_t - v1 * sin_t;
            q_sh[2u * i + 1u] = v0 * sin_t + v1 * cos_t;
        }
        workgroupBarrier();
    }
    let slot = abs_pos % params.max_context;
    let enc_dk = dict_k_val();
    if enc_dk > 0u {
        // ── Phase 4b: OMP-encode the head vector (q_sh) to `enc_dk` code words ──
        let na = dict_n_atoms();
        let is_v = select(0u, 1u, params.proj_kind == 2u);
        for (var d = lid; d < params.head_dim; d = d + WG_SIZE) {
            enc_residual[d] = q_sh[d];
        }
        workgroupBarrier();
        // `n_sel` after pass p is `p + 1` (one atom per pass) — a uniform quantity, so no shared
        // counter is needed (a per-invocation counter would only advance on lid 0 → divergent loops).
        for (var omp_pass = 0u; omp_pass < enc_dk; omp_pass = omp_pass + 1u) {
            let n_sel = omp_pass + 1u;
            for (var a = lid; a < na; a = a + WG_SIZE) {
                var s = 0.0;
                for (var d = 0u; d < params.head_dim; d = d + 1u) {
                    s = s + enc_residual[d] * atom_elem(is_v, a, d);
                }
                enc_corr[a] = s;
            }
            workgroupBarrier();
            if lid == 0u {
                // Greedy pick: atom of max |corr| not already selected (the `omp_pass` picked so far).
                var best = 0u;
                var bestabs = -1.0;
                for (var a = 0u; a < na; a = a + 1u) {
                    var used = false;
                    for (var s2 = 0u; s2 < omp_pass; s2 = s2 + 1u) {
                        if enc_sel[s2] == a { used = true; }
                    }
                    let ab = abs(enc_corr[a]);
                    if !used && ab > bestabs { bestabs = ab; best = a; }
                }
                enc_sel[omp_pass] = best;
                // Least-squares re-solve for the n_sel selected atoms (the "orthogonal" in OMP): build
                // the Gram matrix + rhs against the ORIGINAL vector, solve by Gaussian elimination.
                for (var i = 0u; i < n_sel; i = i + 1u) {
                    var r = 0.0;
                    for (var d = 0u; d < params.head_dim; d = d + 1u) {
                        r = r + atom_elem(is_v, enc_sel[i], d) * q_sh[d];
                    }
                    enc_rhs[i] = r;
                    for (var j = 0u; j < n_sel; j = j + 1u) {
                        var g = 0.0;
                        for (var d = 0u; d < params.head_dim; d = d + 1u) {
                            g = g + atom_elem(is_v, enc_sel[i], d) * atom_elem(is_v, enc_sel[j], d);
                        }
                        enc_gram[i * MAX_K + j] = g;
                    }
                    enc_gram[i * MAX_K + i] = enc_gram[i * MAX_K + i] + 1e-6;
                }
                for (var col = 0u; col < n_sel; col = col + 1u) {
                    let piv = enc_gram[col * MAX_K + col];
                    if abs(piv) > 1e-12 {
                        for (var row = 0u; row < n_sel; row = row + 1u) {
                            if row != col {
                                let f = enc_gram[row * MAX_K + col] / piv;
                                for (var c = col; c < n_sel; c = c + 1u) {
                                    enc_gram[row * MAX_K + c] = enc_gram[row * MAX_K + c] - f * enc_gram[col * MAX_K + c];
                                }
                                enc_rhs[row] = enc_rhs[row] - f * enc_rhs[col];
                            }
                        }
                    }
                }
                for (var i = 0u; i < n_sel; i = i + 1u) {
                    let dgn = enc_gram[i * MAX_K + i];
                    enc_coeff[i] = select(0.0, enc_rhs[i] / dgn, abs(dgn) > 1e-12);
                }
            }
            workgroupBarrier();
            for (var d = lid; d < params.head_dim; d = d + WG_SIZE) {
                var r = q_sh[d];
                for (var i = 0u; i < n_sel; i = i + 1u) {
                    r = r - enc_coeff[i] * atom_elem(is_v, enc_sel[i], d);
                }
                enc_residual[d] = r;
            }
            workgroupBarrier();
        }
        if lid == 0u {
            for (var i = 0u; i < enc_dk; i = i + 1u) {
                kv_cache[code_idx(slot, kv_head, is_v, i)] = pack_code(enc_sel[i], enc_coeff[i]);
            }
        }
    } else if params.kv_quant == 0u {
        for (var d = lid; d < params.head_dim; d = d + WG_SIZE) {
            if params.proj_kind == 1u {
                kv_cache[k_cache_idx(slot, kv_head, d)] = q_sh[d];
            } else {
                kv_cache[v_cache_idx(slot, kv_head, d)] = q_sh[d];
            }
        }
    } else {
        // int8: per-head amax → scale (redundant per-thread scan; head_dim is small), then pack 4
        // i8 lanes per word. The whole workgroup shares q_sh, so every thread sees the same amax.
        var amax = 0.0;
        for (var d = 0u; d < params.head_dim; d = d + 1u) {
            amax = max(amax, abs(q_sh[d]));
        }
        let scale = select(amax / 127.0, 1.0, amax == 0.0);
        let inv = select(1.0 / scale, 0.0, amax == 0.0);
        if lid == 0u {
            if params.proj_kind == 1u {
                kv_cache[kv8_k_scale_idx(slot, kv_head)] = scale;
            } else {
                kv_cache[kv8_v_scale_idx(slot, kv_head)] = scale;
            }
        }
        let hd4 = params.head_dim / 4u;
        for (var word = lid; word < hd4; word = word + WG_SIZE) {
            var packed = 0u;
            for (var lane = 0u; lane < 4u; lane = lane + 1u) {
                let q = clamp(round(q_sh[word * 4u + lane] * inv), -127.0, 127.0);
                let bits = u32(i32(q) & 0xFF);
                packed = packed | (bits << (lane * 8u));
            }
            if params.proj_kind == 1u {
                kv_cache[kv8_k_data_idx(slot, kv_head, word)] = bitcast<f32>(packed);
            } else {
                kv_cache[kv8_v_data_idx(slot, kv_head, word)] = bitcast<f32>(packed);
            }
        }
    }
}

// Decode: one workgroup per Q head (proj_kind=0) or KV head (proj_kind=1|2).
// Prefill: one workgroup per (token_in_batch, kv_head) for batched K/V writes.
// The grid (workgroup count) is unchanged from the @workgroup_size(1) era; WG_SIZE threads now
// cooperate WITHIN each workgroup (key-parallelism). The K/V projection write is cheap and stays on
// a single lane; only the Q-attention path (the bottleneck) is parallelised.
@compute @workgroup_size(64)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let lid = local_id.x;
    q_mask_token = wg_id.y;
    // NOTE: the bounds guards below are WORKGROUP-UNIFORM (derived from `wg_id` + uniform `params`,
    // identical for all threads in the group), and the grid is exactly sized so they never actually
    // exclude a launched workgroup. They are written as `if`-guards, NOT early `return`s, so that the
    // workgroup-collective `workgroupBarrier()`s inside the called functions are reached inside a
    // uniform branch. DX12's FXC (D3DCompile) rejects a barrier that follows an early `return` in a
    // group-varying-looking path (error X3663); a barrier inside an `if` on `wg_id`/params is uniform
    // and compiles on FXC, DXC, SPIR-V and Metal alike. Do NOT reintroduce the early returns.
    if params.proj_kind == 1u || params.proj_kind == 2u {
        let pair = wg_id.x;
        let token_in_batch = pair / params.n_kv_head;
        let kv_head = pair % params.n_kv_head;
        if token_in_batch < params.num_tokens_in_batch && kv_head < params.n_kv_head {
            let abs_pos = params.batch_start_token_idx + token_in_batch;
            write_kv_head(kv_head, token_in_batch, abs_pos, params.proj_kind == 1u, lid);
        }
    } else if params.proj_kind == 0u {
        let qh = wg_id.x;
        let token_ix = select(0u, wg_id.y, params.num_tokens_in_batch > 1u);
        if qh < params.n_head && token_ix < params.num_tokens_in_batch {
            let kv_head = qh / params.q_heads_per_kv;
            attention_parallel(qh, kv_head, token_ix, lid);
        }
    }
}

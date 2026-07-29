// ─────────────────────────────────────────────────────────────────────────────
// QualiaDB math_core — per-weight-role dequant TEMPLATE (Phase 5 dispatch fusion).
//
// This file is NOT valid WGSL on its own. It is a modular fragment instantiated by
// Rust (`gguf_bridge.rs::try_new`) once per FFN weight role, via string substitution:
//   `$W`  → the storage-array binding name  (e.g. `gate_words`, `up_words`)
//   `$S`  → the function-name suffix         (e.g. `_gate`,      `_up`)
// The instances are concatenated after `fused_ffn.wgsl` (which declares the shared
// `params` uniform, the `$W` bindings, and the shared helpers `f16_to_f32` /
// `i8_from_u8` / `weight_row_bytes` referenced below). This keeps the dequant math
// in one authored source while letting a single compute pass read two quantized
// weight matrices — without `ptr<storage,…>` function params (Tint-portable).
// ─────────────────────────────────────────────────────────────────────────────

fn read_u8$S(abs_byte: u32) -> u32 {
    let word = abs_byte >> 2u;
    let shift = (abs_byte & 3u) * 8u;
    return ($W[word] >> shift) & 0xFFu;
}

fn get_scale_min_k4$S(j: u32, scales_base: u32) -> vec2<u32> {
    if j < 4u {
        return vec2<u32>(read_u8$S(scales_base + j) & 63u, read_u8$S(scales_base + j + 4u) & 63u);
    }
    let sc = (read_u8$S(scales_base + j + 4u) & 0xFu) | ((read_u8$S(scales_base + j - 4u) >> 6u) << 4u);
    let m = (read_u8$S(scales_base + j + 4u) >> 4u) | ((read_u8$S(scales_base + j) >> 6u) << 4u);
    return vec2<u32>(sc, m);
}

fn dequant_q4_k_elem$S(block_base: u32, elem: u32) -> f32 {
    let d = f16_to_f32(read_u8$S(block_base) | (read_u8$S(block_base + 1u) << 8u));
    let dmin = f16_to_f32(read_u8$S(block_base + 2u) | (read_u8$S(block_base + 3u) << 8u));
    let scales_base = block_base + 4u;
    let qs_base = block_base + 16u;
    let group = elem / 64u;
    let is = group * 2u;
    let local = elem % 64u;
    let sm0 = get_scale_min_k4$S(is, scales_base);
    let sm1 = get_scale_min_k4$S(is + 1u, scales_base);
    let d1 = d * f32(sm0.x);
    let m1 = dmin * f32(sm0.y);
    let d2 = d * f32(sm1.x);
    let m2 = dmin * f32(sm1.y);
    let q_off = group * 32u;
    if local < 32u {
        let nib = read_u8$S(qs_base + q_off + local) & 0xFu;
        return d1 * f32(nib) - m1;
    }
    let nib = read_u8$S(qs_base + q_off + (local - 32u)) >> 4u;
    return d2 * f32(nib) - m2;
}

fn dequant_q4_k_weight$S(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q4K_ELEMS;
    let block_base = row_base + block_in_row * BLOCK_Q4K_BYTES;
    let elem = col % BLOCK_Q4K_ELEMS;
    return dequant_q4_k_elem$S(block_base, elem);
}

fn dequant_q6_k_weight$S(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let y = col;
    let block_in_row = y / BLOCK_Q6K_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q6K_BYTES;
    let y_in_block = y % BLOCK_Q6K_ELEMS;

    let d_bits = read_u8$S(base + 208u) | (read_u8$S(base + 209u) << 8u);
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
        q = i32((read_u8$S(base + ql_off + l) & 0xFu) | (((read_u8$S(base + qh_off + l) >> 0u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is;
    } else if group == 1u {
        q = i32((read_u8$S(base + ql_off + l + 32u) & 0xFu) | (((read_u8$S(base + qh_off + l) >> 2u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 2u;
    } else if group == 2u {
        q = i32((read_u8$S(base + ql_off + l) >> 4u) | (((read_u8$S(base + qh_off + l) >> 4u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 4u;
    } else {
        q = i32((read_u8$S(base + ql_off + l + 32u) >> 4u) | (((read_u8$S(base + qh_off + l) >> 6u) & 3u) << 4u)) - 32;
        sc_idx = sc_off + is + 6u;
    }
    let sc = i8_from_u8(read_u8$S(base + sc_idx));
    return d * f32(sc) * f32(q);
}

fn dequant_q4_0_weight$S(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q4_0_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q4_0_BYTES;
    let y = col % BLOCK_Q4_0_ELEMS;

    let d_bits = read_u8$S(base) | (read_u8$S(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);

    let half_idx = y % 16u;
    let byte_val = read_u8$S(base + 2u + half_idx);

    var nibble: u32;
    if y < 16u {
        nibble = byte_val & 0xFu;
    } else {
        nibble = byte_val >> 4u;
    }

    let q = i32(nibble) - 8;
    return d * f32(q);
}

// block_q5_0: d(f16) + qh(u32) + qs[16]
fn dequant_q5_0_weight$S(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q5_0_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q5_0_BYTES;
    let y = col % BLOCK_Q5_0_ELEMS;

    let d_bits = read_u8$S(base) | (read_u8$S(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);
    let qh = read_u8$S(base + 2u)
        | (read_u8$S(base + 3u) << 8u)
        | (read_u8$S(base + 4u) << 16u)
        | (read_u8$S(base + 5u) << 24u);

    let half = BLOCK_Q5_0_ELEMS / 2u;
    let j = y % half;
    let qs_byte = read_u8$S(base + 6u + j);

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
fn dequant_q8_0_weight$S(row: u32, col: u32) -> f32 {
    let row_base = row * weight_row_bytes();
    let block_in_row = col / BLOCK_Q8_0_ELEMS;
    let base = row_base + block_in_row * BLOCK_Q8_0_BYTES;
    let y = col % BLOCK_Q8_0_ELEMS;

    let d_bits = read_u8$S(base) | (read_u8$S(base + 1u) << 8u);
    let d = f16_to_f32(d_bits);
    let q = i8_from_u8(read_u8$S(base + 2u + y));
    return d * f32(q);
}

fn dequant_weight$S(row: u32, col: u32) -> f32 {
    if params.weight_ggml_type == GGML_TYPE_Q4_0 {
        return dequant_q4_0_weight$S(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q5_0 {
        return dequant_q5_0_weight$S(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q8_0 {
        return dequant_q8_0_weight$S(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q4_K {
        return dequant_q4_k_weight$S(row, col);
    }
    if params.weight_ggml_type == GGML_TYPE_Q6_K {
        return dequant_q6_k_weight$S(row, col);
    }
    return 0.0;
}

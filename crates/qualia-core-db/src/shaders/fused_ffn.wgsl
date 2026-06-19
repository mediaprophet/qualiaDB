// ─────────────────────────────────────────────────────────────────────────────
// QualiaDB — Fused FFN Expansion (Phase 5 dispatch fusion).
//
// Collapses the SwiGLU expansion `silu(gate·x) * (up·x)` into ONE compute pass,
// eliminating 2 dispatches per layer (the separate gate GEMM, up GEMM and the
// SiLU×mul elementwise) and the round-trip of two n_ffn intermediates through VRAM.
// The result lands in the same slot the elementwise SiLU previously wrote, so the
// downstream `down` projection is unchanged.
//
// MODULAR COMPOSITION: this file declares the shared scaffold (uniform, bindings,
// f16/i8 helpers, weight_row_bytes, silu). The per-weight-role dequant math is
// injected after it by Rust (see `dequant_template.wgsl`) as `dequant_weight_gate`
// / `dequant_weight_up`. WGSL allows out-of-order module-scope declarations, so the
// entry point below may reference those injected functions.
//
// gate and up are required (Rust-gated) to share ggml_type + dims, so a single
// `params` (the gate GEMM's staged GemmParams) describes both dequant streams.
// ─────────────────────────────────────────────────────────────────────────────

struct GemmParams {
    n_in: u32,
    n_out: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    n_batch: u32,
    in_row_stride: u32,
    out_row_stride: u32,
}

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

@group(0) @binding(0) var<storage, read> ffn_input: array<f32>;
@group(0) @binding(1) var<storage, read> gate_words: array<u32>;
@group(0) @binding(2) var<storage, read> up_words: array<u32>;
@group(0) @binding(3) var<uniform> params: GemmParams;
@group(0) @binding(4) var<storage, read_write> ffn_output: array<f32>;

// Phase 6 neuro-symbolic seam. A module `const` (NOT an `@id` override — wgpu 0.19.0
// has no `compilation_options` to set overrides at pipeline creation), so `false`
// const-folds the branch away entirely → the text-only hot path pays ZERO cost.
// Phase 6 flips this (Rust string-injects `true`, or promotes to `@id` once the
// toolchain supports it) and adds a storage binding for the .q42 NQuin.metadata
// bitfield so a tensor with Q42_META_DEONTIC_TAINT set is driven to zero in-silicon.
const ENABLE_DEONTIC_TAINT: bool = false;

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
    return (params.weight_row_elems / BLOCK_Q6K_ELEMS) * BLOCK_Q6K_BYTES;
}

fn silu(x: f32) -> f32 {
    return x / (1.0 + exp(-x));
}

// Rust injects the per-role dequant functions (dequant_weight_gate / dequant_weight_up
// from dequant_template.wgsl) HERE — after the shared helpers they depend on
// (weight_row_bytes / f16_to_f32 / i8_from_u8) and before the entry point that calls
// them — so the source is valid even under declare-before-use shader front-ends.
// @@DEQUANT_FUNCTIONS@@

@compute @workgroup_size(64)
fn fused_ffn_expansion(@builtin(global_invocation_id) global_id: vec3<u32>) {
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

    // One fused sweep over the input: both gate and up dot-products share each
    // loaded activation `x`, then SwiGLU combines them in registers.
    var gate_sum = 0.0;
    var up_sum = 0.0;

    if params.weight_ggml_type == GGML_TYPE_Q5_0 {
        // Phase 5.2 — block-amortized Q5_0. The per-element path re-decodes each 32-elem
        // block's `d` (f16) and 32-bit `qh` for EVERY element (32× redundant ALU — the true
        // anchor on decode tok/s). Here we decode `d`+`qh` for gate AND up ONCE per block,
        // hold them in registers, and do the 32 multiply-accumulates with cheap nibble
        // extraction only. Mathematically identical to `dequant_q5_0_weight`.
        let g_row = i * weight_row_bytes();
        let u_row = i * weight_row_bytes();
        let n_blocks = params.n_in / BLOCK_Q5_0_ELEMS;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let col0 = b * BLOCK_Q5_0_ELEMS;
            let gb = g_row + b * BLOCK_Q5_0_BYTES;
            let ub = u_row + b * BLOCK_Q5_0_BYTES;
            let gd = f16_to_f32(read_u8_gate(gb) | (read_u8_gate(gb + 1u) << 8u));
            let gqh = read_u8_gate(gb + 2u)
                | (read_u8_gate(gb + 3u) << 8u)
                | (read_u8_gate(gb + 4u) << 16u)
                | (read_u8_gate(gb + 5u) << 24u);
            let ud = f16_to_f32(read_u8_up(ub) | (read_u8_up(ub + 1u) << 8u));
            let uqh = read_u8_up(ub + 2u)
                | (read_u8_up(ub + 3u) << 8u)
                | (read_u8_up(ub + 4u) << 16u)
                | (read_u8_up(ub + 5u) << 24u);
            for (var l = 0u; l < 16u; l = l + 1u) {
                let xl = ffn_input[in_base + col0 + l];
                let xh = ffn_input[in_base + col0 + 16u + l];
                let gqs = read_u8_gate(gb + 6u + l);
                let glo = f32(i32((gqs & 0xFu) | (((gqh >> l) << 4u) & 0x10u)) - 16);
                let ghi = f32(i32((gqs >> 4u) | ((gqh >> (l + 12u)) & 0x10u)) - 16);
                gate_sum = gate_sum + gd * (glo * xl + ghi * xh);
                let uqs = read_u8_up(ub + 6u + l);
                let ulo = f32(i32((uqs & 0xFu) | (((uqh >> l) << 4u) & 0x10u)) - 16);
                let uhi = f32(i32((uqs >> 4u) | ((uqh >> (l + 12u)) & 0x10u)) - 16);
                up_sum = up_sum + ud * (ulo * xl + uhi * xh);
            }
        }
    } else {
        // Fallback (non-Q5_0 quant): per-element dequant via the templated math core.
        for (var j = 0u; j < params.n_in; j = j + 1u) {
            let x = ffn_input[in_base + j];
            gate_sum = gate_sum + dequant_weight_gate(i, j) * x;
            up_sum = up_sum + dequant_weight_up(i, j) * x;
        }
    }

    var result = silu(gate_sum) * up_sum;

    if ENABLE_DEONTIC_TAINT {
        // Phase 6 seam (const-folded out today):
        // result = result * f32((nquin_metadata[i] & Q42_META_DEONTIC_TAINT) == 0u);
    }

    ffn_output[out_base + i] = result;
}

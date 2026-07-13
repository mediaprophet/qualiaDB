// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// QualiaDB â€” Fused FFN Expansion (Phase 5 dispatch fusion).
//
// Collapses the SwiGLU expansion `silu(gateÂ·x) * (upÂ·x)` into ONE compute pass,
// eliminating 2 dispatches per layer (the separate gate GEMM, up GEMM and the
// SiLUÃ—mul elementwise) and the round-trip of two n_ffn intermediates through VRAM.
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
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
const GGML_TYPE_Q6_K: u32 = 14u;
const GGML_TYPE_Q4_K_SOA: u32 = 112u;

@group(0) @binding(0) var<storage, read> ffn_input: array<f32>;
@group(0) @binding(1) var<storage, read> gate_words: array<u32>;
@group(0) @binding(2) var<storage, read> up_words: array<u32>;
@group(0) @binding(3) var<uniform> params: GemmParams;
@group(0) @binding(4) var<storage, read_write> ffn_output: array<f32>;

// Phase 6 neuro-symbolic seam. A module `const` (NOT an `@id` override â€” wgpu 0.19.0
// has no `compilation_options` to set overrides at pipeline creation), so `false`
// const-folds the branch away entirely â†’ the text-only hot path pays ZERO cost.
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
    if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA {
        return (params.weight_row_elems / BLOCK_Q4K_ELEMS) * BLOCK_Q4K_SOA_BYTES;
    }
    return (params.weight_row_elems / BLOCK_Q6K_ELEMS) * BLOCK_Q6K_BYTES;
}

fn silu(x: f32) -> f32 {
    return x / (1.0 + exp(-x));
}

// Rust injects the per-role dequant functions (dequant_weight_gate / dequant_weight_up
// from dequant_template.wgsl) HERE â€” after the shared helpers they depend on
// (weight_row_bytes / f16_to_f32 / i8_from_u8) and before the entry point that calls
// them â€” so the source is valid even under declare-before-use shader front-ends.
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
        // Phase 5.2 â€” block-amortized Q5_0. The per-element path re-decodes each 32-elem
        // block's `d` (f16) and 32-bit `qh` for EVERY element (32Ã— redundant ALU â€” the true
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Cooperative fused FFN expansion (T-A1b) â€” one workgroup per output row.
//
// The naive `fused_ffn_expansion` is 1-thread/row serial dequant (measured ~7%
// slower than two coop GEMVs on Q8 smol). This path matches coop_gemv:
//   â€¢ 256 threads stride columns (coalesced weight reads, parallel dequant)
//   â€¢ shared activation tile per 256-wide slab (one global load of x / tile)
//   â€¢ dual partials reduced in LDS â†’ silu(gate)Â·up written once
// Dispatch: (n_out, batch, 1) workgroups of size 256 â€” same as coop_gemv.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const COOP_WG: u32 = 256u;
var<workgroup> coop_act: array<f32, 256>;
// Q4_K_SOA single-row uses global act (no full-act LDS — occupancy win on A2000).
var<workgroup> coop_g: array<f32, 256>;
var<workgroup> coop_u: array<f32, 256>;
// Q4_K dual-matrix header cache (ping-pong 8+8 per role) â€” T-A1c.
var<workgroup> coop_q4k_dsub_g: array<f32, 16>;
var<workgroup> coop_q4k_msub_g: array<f32, 16>;
var<workgroup> coop_q4k_dsub_u: array<f32, 16>;
var<workgroup> coop_q4k_msub_u: array<f32, 16>;

@compute @workgroup_size(256)
fn coop_fused_ffn_expansion(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
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

    var gate_acc = 0.0;
    var up_acc = 0.0;
    let g_row = row * weight_row_bytes();
    let u_row = row * weight_row_bytes();
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;

    // Tile activations + dual dequant/FMA (gate + up share each loaded x).
    if params.weight_ggml_type == GGML_TYPE_Q8_0
        && (params.n_in % BLOCK_Q8_0_ELEMS) == 0u
    {
        let n_tiles = (params.n_in + COOP_WG - 1u) / COOP_WG;
        for (var tile = 0u; tile < n_tiles; tile = tile + 1u) {
            let j = tile * COOP_WG + t;
            if j < params.n_in {
                coop_act[t] = ffn_input[in_base + j];
            } else {
                coop_act[t] = 0.0;
            }
            workgroupBarrier();
            if j < params.n_in {
                let x = coop_act[t];
                let block = j / BLOCK_Q8_0_ELEMS;
                let y = j % BLOCK_Q8_0_ELEMS;
                let gb = g_row + block * BLOCK_Q8_0_BYTES;
                let ub = u_row + block * BLOCK_Q8_0_BYTES;
                let gd = f16_to_f32(read_u8_gate(gb) | (read_u8_gate(gb + 1u) << 8u));
                let ud = f16_to_f32(read_u8_up(ub) | (read_u8_up(ub + 1u) << 8u));
                let gq = f32(i8_from_u8(read_u8_gate(gb + 2u + y)));
                let uq = f32(i8_from_u8(read_u8_up(ub + 2u + y)));
                gate_acc = gate_acc + gd * gq * x;
                up_acc = up_acc + ud * uq * x;
            }
            workgroupBarrier();
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        // Barrier-free Q4 SoA: each lane needs only ffn_input[block*256+t].
        // Global act beats full-act LDS (occupancy) and per-block tiles (barriers).
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let scale_pair = sub >> 1u;
        let scale_hi = (sub & 1u) == 1u;
        let q_off = group * 32u;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let x = ffn_input[in_base + b * BLOCK_Q4K_ELEMS + t];
            let g_blk = g_row + b * BLOCK_Q4K_SOA_BYTES;
            let u_blk = u_row + b * BLOCK_Q4K_SOA_BYTES;
            let gd_word = gate_words[(g_blk + 128u) / 4u + scale_pair];
            let gm_word = gate_words[(g_blk + 144u) / 4u + scale_pair];
            let ud_word = up_words[(u_blk + 128u) / 4u + scale_pair];
            let um_word = up_words[(u_blk + 144u) / 4u + scale_pair];
            let gd = f16_to_f32(select(gd_word & 0xFFFFu, gd_word >> 16u, scale_hi));
            let gm = f16_to_f32(select(gm_word & 0xFFFFu, gm_word >> 16u, scale_hi));
            let ud = f16_to_f32(select(ud_word & 0xFFFFu, ud_word >> 16u, scale_hi));
            let um = f16_to_f32(select(um_word & 0xFFFFu, um_word >> 16u, scale_hi));
            var gnib: u32;
            var unib: u32;
            if local < 32u {
                let gbi = g_blk + q_off + local;
                let ubi = u_blk + q_off + local;
                gnib = (gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFu;
                unib = (up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFu;
            } else {
                let gbi = g_blk + q_off + (local - 32u);
                let ubi = u_blk + q_off + (local - 32u);
                gnib = ((gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFFu) >> 4u;
                unib = ((up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            gate_acc = gate_acc + (gd * f32(gnib) - gm) * x;
            up_acc = up_acc + (ud * f32(unib) - um) * x;
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        // Interleaved Q4_K + dual header cache (8 threads decode scales once / block / role).
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let g_base = g_row + b * BLOCK_Q4K_BYTES;
            let u_base = u_row + b * BLOCK_Q4K_BYTES;
            let slot = (b & 1u) * 8u;
            if t < 8u {
                let gd_word = gate_words[g_base >> 2u];
                let ud_word = up_words[u_base >> 2u];
                let gd = f16_to_f32(gd_word & 0xFFFFu);
                let gdmin = f16_to_f32(gd_word >> 16u);
                let ud = f16_to_f32(ud_word & 0xFFFFu);
                let udmin = f16_to_f32(ud_word >> 16u);
                let gsm = get_scale_min_k4_gate(t, g_base + 4u);
                let usm = get_scale_min_k4_up(t, u_base + 4u);
                coop_q4k_dsub_g[slot + t] = gd * f32(gsm.x);
                coop_q4k_msub_g[slot + t] = gdmin * f32(gsm.y);
                coop_q4k_dsub_u[slot + t] = ud * f32(usm.x);
                coop_q4k_msub_u[slot + t] = udmin * f32(usm.y);
            }
            coop_act[t] = ffn_input[in_base + b * BLOCK_Q4K_ELEMS + t];
            workgroupBarrier();
            let x = coop_act[t];
            let g_qs = g_base + 16u;
            let u_qs = u_base + 16u;
            let q_off = group * 32u;
            var gnib: u32;
            var unib: u32;
            if local < 32u {
                let gbi = g_qs + q_off + local;
                let ubi = u_qs + q_off + local;
                gnib = (gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFu;
                unib = (up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFu;
            } else {
                let gbi = g_qs + q_off + (local - 32u);
                let ubi = u_qs + q_off + (local - 32u);
                gnib = ((gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFFu) >> 4u;
                unib = ((up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            gate_acc = gate_acc
                + (coop_q4k_dsub_g[slot + sub] * f32(gnib) - coop_q4k_msub_g[slot + sub]) * x;
            up_acc = up_acc
                + (coop_q4k_dsub_u[slot + sub] * f32(unib) - coop_q4k_msub_u[slot + sub]) * x;
            workgroupBarrier();
        }
    } else {
        // Generic: strided columns + shared act tile (Q4_0 / Q5_0 / Q6_K / misaligned).
        let n_tiles = (params.n_in + COOP_WG - 1u) / COOP_WG;
        for (var tile = 0u; tile < n_tiles; tile = tile + 1u) {
            let j = tile * COOP_WG + t;
            if j < params.n_in {
                coop_act[t] = ffn_input[in_base + j];
            } else {
                coop_act[t] = 0.0;
            }
            workgroupBarrier();
            if j < params.n_in {
                let x = coop_act[t];
                gate_acc = gate_acc + dequant_weight_gate(row, j) * x;
                up_acc = up_acc + dequant_weight_up(row, j) * x;
            }
            workgroupBarrier();
        }
    }

    coop_g[t] = gate_acc;
    coop_u[t] = up_acc;
    workgroupBarrier();

    // Tree reduce both partials (power-of-two WG size).
    var stride = COOP_WG / 2u;
    loop {
        if t < stride {
            coop_g[t] = coop_g[t] + coop_g[t + stride];
            coop_u[t] = coop_u[t] + coop_u[t + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if stride == 0u {
            break;
        }
    }

    if t == 0u {
        var result = silu(coop_g[0]) * coop_u[0];
        if ENABLE_DEONTIC_TAINT {
            // const-folded out today
        }
        ffn_output[out_base + row] = result;
    }
}

// Subgroup-reduction twin of `coop_fused_ffn_expansion` (when adapter has SUBGROUP).
// Accumulation identical; final dual reduce uses subgroupAdd + tiny cross-wave combine.
@compute @workgroup_size(256)
fn coop_fused_ffn_sg(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
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

    var gate_acc = 0.0;
    var up_acc = 0.0;
    let g_row = row * weight_row_bytes();
    let u_row = row * weight_row_bytes();
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;

    if params.weight_ggml_type == GGML_TYPE_Q8_0
        && (params.n_in % BLOCK_Q8_0_ELEMS) == 0u
    {
        let n_tiles = (params.n_in + COOP_WG - 1u) / COOP_WG;
        for (var tile = 0u; tile < n_tiles; tile = tile + 1u) {
            let j = tile * COOP_WG + t;
            if j < params.n_in {
                coop_act[t] = ffn_input[in_base + j];
            } else {
                coop_act[t] = 0.0;
            }
            workgroupBarrier();
            if j < params.n_in {
                let x = coop_act[t];
                let block = j / BLOCK_Q8_0_ELEMS;
                let y = j % BLOCK_Q8_0_ELEMS;
                let gb = g_row + block * BLOCK_Q8_0_BYTES;
                let ub = u_row + block * BLOCK_Q8_0_BYTES;
                let gd = f16_to_f32(read_u8_gate(gb) | (read_u8_gate(gb + 1u) << 8u));
                let ud = f16_to_f32(read_u8_up(ub) | (read_u8_up(ub + 1u) << 8u));
                let gq = f32(i8_from_u8(read_u8_gate(gb + 2u + y)));
                let uq = f32(i8_from_u8(read_u8_up(ub + 2u + y)));
                gate_acc = gate_acc + gd * gq * x;
                up_acc = up_acc + ud * uq * x;
            }
            workgroupBarrier();
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        // SG path: barrier-free global act (same as coop_fused_ffn_expansion).
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let scale_pair = sub >> 1u;
        let scale_hi = (sub & 1u) == 1u;
        let q_off = group * 32u;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let x = ffn_input[in_base + b * BLOCK_Q4K_ELEMS + t];
            let g_blk = g_row + b * BLOCK_Q4K_SOA_BYTES;
            let u_blk = u_row + b * BLOCK_Q4K_SOA_BYTES;
            let gd_word = gate_words[(g_blk + 128u) / 4u + scale_pair];
            let gm_word = gate_words[(g_blk + 144u) / 4u + scale_pair];
            let ud_word = up_words[(u_blk + 128u) / 4u + scale_pair];
            let um_word = up_words[(u_blk + 144u) / 4u + scale_pair];
            let gd = f16_to_f32(select(gd_word & 0xFFFFu, gd_word >> 16u, scale_hi));
            let gm = f16_to_f32(select(gm_word & 0xFFFFu, gm_word >> 16u, scale_hi));
            let ud = f16_to_f32(select(ud_word & 0xFFFFu, ud_word >> 16u, scale_hi));
            let um = f16_to_f32(select(um_word & 0xFFFFu, um_word >> 16u, scale_hi));
            var gnib: u32;
            var unib: u32;
            if local < 32u {
                let gbi = g_blk + q_off + local;
                let ubi = u_blk + q_off + local;
                gnib = (gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFu;
                unib = (up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFu;
            } else {
                let gbi = g_blk + q_off + (local - 32u);
                let ubi = u_blk + q_off + (local - 32u);
                gnib = ((gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFFu) >> 4u;
                unib = ((up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            gate_acc = gate_acc + (gd * f32(gnib) - gm) * x;
            up_acc = up_acc + (ud * f32(unib) - um) * x;
        }
    } else if params.weight_ggml_type == GGML_TYPE_Q4_K
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let g_base = g_row + b * BLOCK_Q4K_BYTES;
            let u_base = u_row + b * BLOCK_Q4K_BYTES;
            let slot = (b & 1u) * 8u;
            if t < 8u {
                let gd_word = gate_words[g_base >> 2u];
                let ud_word = up_words[u_base >> 2u];
                let gd = f16_to_f32(gd_word & 0xFFFFu);
                let gdmin = f16_to_f32(gd_word >> 16u);
                let ud = f16_to_f32(ud_word & 0xFFFFu);
                let udmin = f16_to_f32(ud_word >> 16u);
                let gsm = get_scale_min_k4_gate(t, g_base + 4u);
                let usm = get_scale_min_k4_up(t, u_base + 4u);
                coop_q4k_dsub_g[slot + t] = gd * f32(gsm.x);
                coop_q4k_msub_g[slot + t] = gdmin * f32(gsm.y);
                coop_q4k_dsub_u[slot + t] = ud * f32(usm.x);
                coop_q4k_msub_u[slot + t] = udmin * f32(usm.y);
            }
            coop_act[t] = ffn_input[in_base + b * BLOCK_Q4K_ELEMS + t];
            workgroupBarrier();
            let x = coop_act[t];
            let g_qs = g_base + 16u;
            let u_qs = u_base + 16u;
            let q_off = group * 32u;
            var gnib: u32;
            var unib: u32;
            if local < 32u {
                let gbi = g_qs + q_off + local;
                let ubi = u_qs + q_off + local;
                gnib = (gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFu;
                unib = (up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFu;
            } else {
                let gbi = g_qs + q_off + (local - 32u);
                let ubi = u_qs + q_off + (local - 32u);
                gnib = ((gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFFu) >> 4u;
                unib = ((up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            gate_acc = gate_acc
                + (coop_q4k_dsub_g[slot + sub] * f32(gnib) - coop_q4k_msub_g[slot + sub]) * x;
            up_acc = up_acc
                + (coop_q4k_dsub_u[slot + sub] * f32(unib) - coop_q4k_msub_u[slot + sub]) * x;
            workgroupBarrier();
        }
    } else {
        let n_tiles = (params.n_in + COOP_WG - 1u) / COOP_WG;
        for (var tile = 0u; tile < n_tiles; tile = tile + 1u) {
            let j = tile * COOP_WG + t;
            if j < params.n_in {
                coop_act[t] = ffn_input[in_base + j];
            } else {
                coop_act[t] = 0.0;
            }
            workgroupBarrier();
            if j < params.n_in {
                let x = coop_act[t];
                gate_acc = gate_acc + dequant_weight_gate(row, j) * x;
                up_acc = up_acc + dequant_weight_up(row, j) * x;
            }
            workgroupBarrier();
        }
    }

    // Wave reduce both partials; lane 0 publishes to LDS; thread 0 finishes.
    let g_sum = subgroupAdd(gate_acc);
    let u_sum = subgroupAdd(up_acc);
    if sg_lane == 0u {
        coop_g[t / sg_size] = g_sum;
        coop_u[t / sg_size] = u_sum;
    }
    workgroupBarrier();
    if t == 0u {
        let n_sg = (COOP_WG + sg_size - 1u) / sg_size;
        var tg = 0.0;
        var tu = 0.0;
        for (var s = 0u; s < n_sg; s = s + 1u) {
            tg = tg + coop_g[s];
            tu = tu + coop_u[s];
        }
        ffn_output[out_base + row] = silu(tg) * tu;
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Multi-row fused FFN (3B lever): one K-sweep for FFN_MR_ROWS consecutive rows.
// Load full activation once; accumulate gate/up for R rows in registers; reduce.
// Dispatch: ceil(n_out / FFN_MR_ROWS). Same bindings as coop_fused_ffn_expansion.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const FFN_MR_ROWS: u32 = 4u;

@compute @workgroup_size(256)
fn coop_fused_ffn_mr(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let row0 = wg_id.x * FFN_MR_ROWS;
    if row0 >= params.n_out { return; }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    // Q4_K_SOA multi-row: barrier-free global act (lane t shared across R rows).
    let soa_ok = params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
        && params.n_in > 0u;

    var g0 = 0.0; var g1 = 0.0; var g2 = 0.0; var g3 = 0.0;
    var u0 = 0.0; var u1 = 0.0; var u2 = 0.0; var u3 = 0.0;

    if soa_ok {
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        let sub = t / 32u;
        let group = t / 64u;
        let local = t % 64u;
        let scale_pair = sub >> 1u;
        let scale_hi = (sub & 1u) == 1u;
        let rb = weight_row_bytes();

        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let x = ffn_input[in_base + b * BLOCK_Q4K_ELEMS + t];
            let col_base = b * BLOCK_Q4K_SOA_BYTES;
            // Unrolled R=4 so register pressure stays predictable.
            for (var r = 0u; r < FFN_MR_ROWS; r = r + 1u) {
                let row = row0 + r;
                if row >= params.n_out { continue; }
                let g_blk2 = row * rb + col_base;
                let u_blk2 = row * rb + col_base;
                let gd_word = gate_words[(g_blk2 + 128u) / 4u + scale_pair];
                let gm_word = gate_words[(g_blk2 + 144u) / 4u + scale_pair];
                let ud_word = up_words[(u_blk2 + 128u) / 4u + scale_pair];
                let um_word = up_words[(u_blk2 + 144u) / 4u + scale_pair];
                let gd = f16_to_f32(select(gd_word & 0xFFFFu, gd_word >> 16u, scale_hi));
                let gm = f16_to_f32(select(gm_word & 0xFFFFu, gm_word >> 16u, scale_hi));
                let ud = f16_to_f32(select(ud_word & 0xFFFFu, ud_word >> 16u, scale_hi));
                let um = f16_to_f32(select(um_word & 0xFFFFu, um_word >> 16u, scale_hi));
                let q_off = group * 32u;
                var gnib: u32;
                var unib: u32;
                if local < 32u {
                    let gbi = g_blk2 + q_off + local;
                    let ubi = u_blk2 + q_off + local;
                    gnib = (gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFu;
                    unib = (up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFu;
                } else {
                    let gbi = g_blk2 + q_off + (local - 32u);
                    let ubi = u_blk2 + q_off + (local - 32u);
                    gnib = ((gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFFu) >> 4u;
                    unib = ((up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFFu) >> 4u;
                }
                let gv = (gd * f32(gnib) - gm) * x;
                let uv = (ud * f32(unib) - um) * x;
                if r == 0u { g0 = g0 + gv; u0 = u0 + uv; }
                else if r == 1u { g1 = g1 + gv; u1 = u1 + uv; }
                else if r == 2u { g2 = g2 + gv; u2 = u2 + uv; }
                else { g3 = g3 + gv; u3 = u3 + uv; }
            }
        }
    } else {
        // Fallback: serial single-row dots via full path for each live row.
        for (var r = 0u; r < FFN_MR_ROWS; r = r + 1u) {
            let row = row0 + r;
            if row >= params.n_out { continue; }
            var gate_acc = 0.0;
            var up_acc = 0.0;
            let g_row = row * weight_row_bytes();
            let u_row = row * weight_row_bytes();
            var j = t;
            loop {
                if j >= params.n_in { break; }
                let x = ffn_input[in_base + j];
                gate_acc = gate_acc + dequant_weight_gate(row, j) * x;
                up_acc = up_acc + dequant_weight_up(row, j) * x;
                j = j + COOP_WG;
            }
            if r == 0u { g0 = gate_acc; u0 = up_acc; }
            else if r == 1u { g1 = gate_acc; u1 = up_acc; }
            else if r == 2u { g2 = gate_acc; u2 = up_acc; }
            else { g3 = gate_acc; u3 = up_acc; }
        }
    }

    // Reduce + write each of the R rows via subgroupAdd (same as single-row FFN).
    var accs_g: array<f32, 4>;
    var accs_u: array<f32, 4>;
    accs_g[0] = g0; accs_g[1] = g1; accs_g[2] = g2; accs_g[3] = g3;
    accs_u[0] = u0; accs_u[1] = u1; accs_u[2] = u2; accs_u[3] = u3;
    for (var r = 0u; r < FFN_MR_ROWS; r = r + 1u) {
        let row = row0 + r;
        let row_ok = row < params.n_out;
        let g_sum = subgroupAdd(accs_g[r]);
        let u_sum = subgroupAdd(accs_u[r]);
        if sg_lane == 0u {
            coop_g[t / sg_size] = g_sum;
            coop_u[t / sg_size] = u_sum;
        }
        workgroupBarrier();
        if t == 0u && row_ok {
            let n_sg = (COOP_WG + sg_size - 1u) / sg_size;
            var tg = 0.0;
            var tu = 0.0;
            for (var s = 0u; s < n_sg; s = s + 1u) {
                tg = tg + coop_g[s];
                tu = tu + coop_u[s];
            }
            ffn_output[out_base + row] = silu(tg) * tu;
        }
        workgroupBarrier();
    }
}

// Warp fused FFN: 32 threads/row, 8 columns/lane per Q4 superblock â€” denser FMA, smaller reduce.
const FFN_WARP: u32 = 32u;

@compute @workgroup_size(32)
fn coop_fused_ffn_warp(
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

    var gate_acc = 0.0;
    var up_acc = 0.0;
    let g_row = row * weight_row_bytes();
    let u_row = row * weight_row_bytes();

    if params.weight_ggml_type == GGML_TYPE_Q4_K_SOA
        && (params.n_in % BLOCK_Q4K_ELEMS) == 0u
    {
        let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
        for (var b = 0u; b < n_blocks; b = b + 1u) {
            let g_blk = g_row + b * BLOCK_Q4K_SOA_BYTES;
            let u_blk = u_row + b * BLOCK_Q4K_SOA_BYTES;
            for (var k = 0u; k < 8u; k = k + 1u) {
                let local_col = t + k * FFN_WARP;
                let sub = local_col / 32u;
                let group = local_col / 64u;
                let local = local_col % 64u;
                let scale_pair = sub >> 1u;
                let scale_hi = (sub & 1u) == 1u;
                let gd_word = gate_words[(g_blk + 128u) / 4u + scale_pair];
                let gm_word = gate_words[(g_blk + 144u) / 4u + scale_pair];
                let ud_word = up_words[(u_blk + 128u) / 4u + scale_pair];
                let um_word = up_words[(u_blk + 144u) / 4u + scale_pair];
                let gd = f16_to_f32(select(gd_word & 0xFFFFu, gd_word >> 16u, scale_hi));
                let gm = f16_to_f32(select(gm_word & 0xFFFFu, gm_word >> 16u, scale_hi));
                let ud = f16_to_f32(select(ud_word & 0xFFFFu, ud_word >> 16u, scale_hi));
                let um = f16_to_f32(select(um_word & 0xFFFFu, um_word >> 16u, scale_hi));
                let q_off = group * 32u;
                var gnib: u32;
                var unib: u32;
                if local < 32u {
                    let gbi = g_blk + q_off + local;
                    let ubi = u_blk + q_off + local;
                    gnib = (gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFu;
                    unib = (up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFu;
                } else {
                    let gbi = g_blk + q_off + (local - 32u);
                    let ubi = u_blk + q_off + (local - 32u);
                    gnib = ((gate_words[gbi >> 2u] >> ((gbi & 3u) * 8u)) & 0xFFu) >> 4u;
                    unib = ((up_words[ubi >> 2u] >> ((ubi & 3u) * 8u)) & 0xFFu) >> 4u;
                }
                let x = ffn_input[in_base + b * BLOCK_Q4K_ELEMS + local_col];
                gate_acc = gate_acc + (gd * f32(gnib) - gm) * x;
                up_acc = up_acc + (ud * f32(unib) - um) * x;
            }
        }
    } else {
        var j = t;
        loop {
            if j >= params.n_in { break; }
            let x = ffn_input[in_base + j];
            gate_acc = gate_acc + dequant_weight_gate(row, j) * x;
            up_acc = up_acc + dequant_weight_up(row, j) * x;
            j = j + FFN_WARP;
        }
    }

    coop_g[t] = gate_acc;
    coop_u[t] = up_acc;
    workgroupBarrier();
    if t < 16u {
        coop_g[t] = coop_g[t] + coop_g[t + 16u];
        coop_u[t] = coop_u[t] + coop_u[t + 16u];
    }
    workgroupBarrier();
    if t < 8u {
        coop_g[t] = coop_g[t] + coop_g[t + 8u];
        coop_u[t] = coop_u[t] + coop_u[t + 8u];
    }
    workgroupBarrier();
    if t < 4u {
        coop_g[t] = coop_g[t] + coop_g[t + 4u];
        coop_u[t] = coop_u[t] + coop_u[t + 4u];
    }
    workgroupBarrier();
    if t < 2u {
        coop_g[t] = coop_g[t] + coop_g[t + 2u];
        coop_u[t] = coop_u[t] + coop_u[t + 2u];
    }
    workgroupBarrier();
    if t < 1u {
        coop_g[t] = coop_g[t] + coop_g[t + 1u];
        coop_u[t] = coop_u[t] + coop_u[t + 1u];
    }
    workgroupBarrier();
    if t == 0u {
        ffn_output[out_base + row] = silu(coop_g[0]) * coop_u[0];
    }
}

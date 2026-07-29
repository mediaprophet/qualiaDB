// ─────────────────────────────────────────────────────────────────────────────
// QualiaDB — BitNet b1.58 ternary GEMM (STELLAR §A, task #12).
//
//   out[m][i] = scale · Σ_j  trit(W[i][j]) · act[m][j]      W ∈ {-1, 0, +1}
//
// The BitNet win: a ternary weight contributes by **add / subtract only** — there is
// no per-weight multiply in the inner loop. The single per-tensor `scale` multiply
// happens once per output element, at the end. (Contrast `fused_ffn.wgsl`, which does
// a dequant-multiply-accumulate per weight.)
//
// Weights: the row-major trits of an (n_out × n_in) matrix, packed 5-per-byte in
// base-3 (see `ternary.rs::pack_trits`). The 4-byte f32 scale that prefixes a ternary
// blob is passed in `params.scale` — it is NOT in `trit_words`.
//
// CPU oracle / parity reference: `ternary::ternary_gemm_cpu` mirrors this kernel
// exactly (same trit extraction, same add/subtract, same end-scale).
// ─────────────────────────────────────────────────────────────────────────────

struct TernaryParams {
    n_in: u32,
    n_out: u32,
    n_batch: u32,
    in_row_stride: u32,
    out_row_stride: u32,
    scale: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> activations: array<f32>;
@group(0) @binding(1) var<storage, read> trit_words: array<u32>;
@group(0) @binding(2) var<uniform> params: TernaryParams;
@group(0) @binding(3) var<storage, read_write> ternary_output: array<f32>;

// Read one packed byte from the u32 word array (little-endian within the word).
fn read_byte(idx: u32) -> u32 {
    let w = trit_words[idx >> 2u];
    return (w >> ((idx & 3u) * 8u)) & 0xFFu;
}

// Ternary value {-1, 0, +1} at linear weight index `k` (5 trits/byte, base-3).
fn trit_at(k: u32) -> i32 {
    let pos = k % 5u;
    var b = read_byte(k / 5u);
    for (var p = 0u; p < pos; p = p + 1u) {
        b = b / 3u;
    }
    return i32(b % 3u) - 1;
}

@compute @workgroup_size(64)
fn ternary_gemm(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x; // output feature (column of W)
    let m = gid.y; // batch row
    let batch = max(params.n_batch, 1u);
    if (m >= batch || i >= params.n_out) {
        return;
    }

    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let row0 = i * params.n_in; // linear trit base for weight row i

    var acc = 0.0;
    for (var j = 0u; j < params.n_in; j = j + 1u) {
        let t = trit_at(row0 + j);
        let x = activations[in_base + j];
        // add/subtract — no multiply by the weight (the §A win)
        if (t > 0) {
            acc = acc + x;
        } else if (t < 0) {
            acc = acc - x;
        }
    }

    ternary_output[m * out_stride + i] = params.scale * acc;
}

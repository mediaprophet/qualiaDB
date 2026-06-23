// ─────────────────────────────────────────────────────────────────────────────
// QualiaDB — BitNet b1.58 ternary GEMM, **2-bit branchless** variant (STELLAR §A).
//
//   out[m][i] = scale · Σ_j  trit(W[i][j]) · act[m][j]      W ∈ {-1, 0, +1}
//
// Optimised for GPU execution (external-review-driven):
//   • 2-bit packing (4 trits/byte: 00=0, 01=+1, 10=-1) → unpack with SHIFT + MASK only,
//     no integer `/` or `%` (base-3 needs both — dozens of cycles on Ampere).
//   • BRANCHLESS accumulation: the trit becomes a float multiplier `f32(c==1) - f32(c==2)`
//     and the body is a single FMA. Every thread in the warp runs identical instructions —
//     no divergence (vs the `if trit>0 … else if trit<0` form in `ternary_gemm.wgsl`).
// On a GPU the multiply is free (FMA); the ternary win here is bandwidth + occupancy.
//
// Bindings + params are identical to `ternary_gemm.wgsl`; CPU oracle: `ternary::ternary_gemm_cpu_2bit`.
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

// One packed byte from the u32 word array (little-endian within the word).
fn read_byte(idx: u32) -> u32 {
    let w = trit_words[idx >> 2u];
    return (w >> ((idx & 3u) * 8u)) & 0xFFu;
}

// 2-bit code {0,1,2} at linear weight index k (4 codes/byte) → trit {0,+1,-1} as f32, branchless.
fn trit_f32(k: u32) -> f32 {
    let byte = read_byte(k >> 2u);              // k / 4
    let code = (byte >> ((k & 3u) * 2u)) & 3u;  // (k % 4) * 2 bits
    return f32(code == 1u) - f32(code == 2u);   // +1, -1, or 0 — no branch
}

@compute @workgroup_size(64)
fn ternary_gemm(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x; // output feature
    let m = gid.y; // batch row
    let batch = max(params.n_batch, 1u);
    if (m >= batch || i >= params.n_out) {
        return;
    }

    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let row0 = i * params.n_in;

    var acc = 0.0;
    for (var j = 0u; j < params.n_in; j = j + 1u) {
        // single FMA, identical across the warp
        acc = acc + trit_f32(row0 + j) * activations[in_base + j];
    }

    ternary_output[m * out_stride + i] = params.scale * acc;
}

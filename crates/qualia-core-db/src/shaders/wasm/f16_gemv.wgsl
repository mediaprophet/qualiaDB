// ─────────────────────────────────────────────────────────────────────────────
// QualiaDB — F16 GEMV baseline (benchmark reference for the ternary kernels).
//
//   out[i] = Σ_j  f16(W[i][j]) · act[j]      (batch = 1, decode shape)
//
// Same binding layout + `TernaryParams` uniform as `ternary_gemm{,_2bit}.wgsl` so a single bench
// harness can A/B all three. Reads 16-bit weights (2 per u32 word) → ~8× the weight traffic of the
// 2-bit ternary kernel; the comparison isolates the *bandwidth* win (decode is memory-bound).
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
@group(0) @binding(1) var<storage, read> weights: array<u32>; // 2 × f16 per word
@group(0) @binding(2) var<uniform> params: TernaryParams;
@group(0) @binding(3) var<storage, read_write> gemv_output: array<f32>;

fn f16_to_f32(bits: u32) -> f32 {
    let s = (bits >> 15u) & 1u;
    var e = (bits >> 10u) & 0x1Fu;
    let f = bits & 0x3FFu;
    if (e == 0u) {
        if (f == 0u) { return select(0.0, -0.0, s == 1u); }
        var v = (f32(f) / 1024.0) * exp2(-14.0);
        return select(v, -v, s == 1u);
    }
    if (e == 31u) { return select(1e30, -1e30, s == 1u); }
    var v = (1.0 + f32(f) / 1024.0) * exp2(f32(i32(e) - 15));
    return select(v, -v, s == 1u);
}

// f16 weight at linear index k (2 per u32 word).
fn weight_f32(k: u32) -> f32 {
    let w = weights[k >> 1u];
    let bits = select(w & 0xFFFFu, w >> 16u, (k & 1u) == 1u);
    return f16_to_f32(bits);
}

@compute @workgroup_size(64)
fn f16_gemv(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n_out) {
        return;
    }
    let row0 = i * params.n_in;
    var acc = 0.0;
    for (var j = 0u; j < params.n_in; j = j + 1u) {
        acc = acc + weight_f32(row0 + j) * activations[j];
    }
    gemv_output[i] = acc;
}

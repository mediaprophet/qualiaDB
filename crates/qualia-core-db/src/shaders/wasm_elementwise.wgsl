// MC8: RMSNorm, SiLU×mul, residual-add — keeps hidden state on GPU between layers.

struct ElemParams {
    n: u32,
    batch: u32,
    op: u32, // 0=rms_norm 1=silu_mul 2=add_residual
    eps: f32,
}

@group(0) @binding(0) var<storage, read> buf_a: array<f32>;
@group(0) @binding(1) var<storage, read> buf_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> buf_out: array<f32>;
@group(0) @binding(3) var<uniform> params: ElemParams;

// RMSNorm: one workgroup per batch row (n ≤ ~4k; serial reduction is fine for MC8).
@compute @workgroup_size(1)
fn rms_norm_batch(@builtin(workgroup_id) wg_id: vec3<u32>) {
    let n = params.n;
    let b = wg_id.x;
    if b >= params.batch || n == 0u {
        return;
    }
    let base = b * n;
    var ss = 0.0;
    for (var j = 0u; j < n; j = j + 1u) {
        let v = buf_a[base + j];
        ss = ss + v * v;
    }
    ss = ss / f32(n);
    let inv_rms = 1.0 / sqrt(ss + params.eps);
    for (var i = 0u; i < n; i = i + 1u) {
        buf_out[base + i] = buf_a[base + i] * inv_rms * buf_b[i];
    }
}

@compute @workgroup_size(64)
fn silu_mul_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = params.n;
    let b = gid.y;
    if b >= params.batch || n == 0u {
        return;
    }
    let i = gid.x;
    if i >= n {
        return;
    }
    let idx = b * n + i;
    let g = buf_a[idx];
    let silu = g / (1.0 + exp(-g));
    buf_out[idx] = silu * buf_b[idx];
}

@compute @workgroup_size(64)
fn add_residual_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = params.n;
    let b = gid.y;
    if b >= params.batch || n == 0u {
        return;
    }
    let i = gid.x;
    if i >= n {
        return;
    }
    let idx = b * n + i;
    buf_out[idx] = buf_a[idx] + buf_b[idx];
}
// MC8: RMSNorm, SiLU×mul, residual-add — keeps hidden state on GPU between layers.

struct ElemParams {
    n: u32,
    batch: u32,
    op: u32, // 0=rms_norm 1=silu_mul 2=add_residual
    eps: f32,
    a_row_stride: u32,   // floats between rows; 0 → n
    b_row_stride: u32,
    out_row_stride: u32,
    a_slot: u32,         // float offset within row
    b_slot: u32,
    out_slot: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> buf_a: array<f32>;
@group(0) @binding(1) var<storage, read> buf_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> buf_out: array<f32>;
@group(0) @binding(3) var<uniform> params: ElemParams;

fn row_stride(stride: u32) -> u32 {
    return select(params.n, stride, stride > 0u);
}

fn a_idx(m: u32, i: u32) -> u32 {
    return m * row_stride(params.a_row_stride) + params.a_slot + i;
}

fn b_idx(m: u32, i: u32) -> u32 {
    return m * row_stride(params.b_row_stride) + params.b_slot + i;
}

fn out_idx(m: u32, i: u32) -> u32 {
    return m * row_stride(params.out_row_stride) + params.out_slot + i;
}

// RMSNorm: one workgroup per batch row — variance reduction is row-local only (no cross-token barrier).
@compute @workgroup_size(1)
fn rms_norm_batch(@builtin(workgroup_id) wg_id: vec3<u32>) {
    let n = params.n;
    let m = wg_id.y;
    if m >= params.batch || n == 0u {
        return;
    }
    let a_base = a_idx(m, 0u);
    let o_base = out_idx(m, 0u);
    var ss = 0.0;
    for (var j = 0u; j < n; j = j + 1u) {
        let v = buf_a[a_base + j];
        ss = ss + v * v;
    }
    ss = ss / f32(n);
    let inv_rms = 1.0 / sqrt(ss + params.eps);
    for (var i = 0u; i < n; i = i + 1u) {
        buf_out[o_base + i] = buf_a[a_base + i] * inv_rms * buf_b[i];
    }
}

@compute @workgroup_size(64)
fn silu_mul_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = params.n;
    let m = gid.y;
    if m >= params.batch || n == 0u {
        return;
    }
    let i = gid.x;
    if i >= n {
        return;
    }
    let ia = a_idx(m, i);
    let ib = b_idx(m, i);
    let io = out_idx(m, i);
    let g = buf_a[ia];
    let silu = g / (1.0 + exp(-g));
    buf_out[io] = silu * buf_b[ib];
}

@compute @workgroup_size(64)
fn add_residual_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = params.n;
    let m = gid.y;
    if m >= params.batch || n == 0u {
        return;
    }
    let i = gid.x;
    if i >= n {
        return;
    }
    buf_out[out_idx(m, i)] = buf_a[a_idx(m, i)] + buf_b[b_idx(m, i)];
}
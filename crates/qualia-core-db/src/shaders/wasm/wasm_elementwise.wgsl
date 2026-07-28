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

// RMSNorm: one workgroup per batch row, **256-wide parallel reduce**.
// Previous path used @workgroup_size(1) and a scalar loop over n (3072–4096) —
// ~57 single-thread RMS passes/token on 3B. Parallel partials + tree reduce keeps
// the same mean-square formula (ss/n + eps) while using the SM.
const RMS_WG: u32 = 256u;
var<workgroup> rms_partial: array<f32, 256>;
var<workgroup> rms_inv: f32;

@compute @workgroup_size(256)
fn rms_norm_batch(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let n = params.n;
    let m = wg_id.y;
    let t = lid.x;
    if m >= params.batch || n == 0u {
        return;
    }
    let a_base = a_idx(m, 0u);
    let o_base = out_idx(m, 0u);

    // Phase 1: each lane accumulates squares over a strided slice of the row.
    var local_ss = 0.0;
    var j = t;
    loop {
        if j >= n { break; }
        let v = buf_a[a_base + j];
        local_ss = local_ss + v * v;
        j = j + RMS_WG;
    }
    rms_partial[t] = local_ss;
    workgroupBarrier();

    // Phase 2: tree reduce → inv_rms in shared.
    var stride = RMS_WG >> 1u;
    loop {
        if stride == 0u { break; }
        if t < stride {
            rms_partial[t] = rms_partial[t] + rms_partial[t + stride];
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if t == 0u {
        let mean_sq = rms_partial[0] / f32(n);
        rms_inv = 1.0 / sqrt(mean_sq + params.eps);
    }
    workgroupBarrier();
    let inv_rms = rms_inv;

    // Phase 3: write normed · weight (weight is length-n in buf_b[0..n]).
    var i = t;
    loop {
        if i >= n { break; }
        buf_out[o_base + i] = buf_a[a_base + i] * inv_rms * buf_b[i];
        i = i + RMS_WG;
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
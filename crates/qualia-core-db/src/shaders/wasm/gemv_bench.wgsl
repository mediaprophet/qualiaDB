// gemv_bench.wgsl — minimal f32 GEMV for the cross-circuit capability probe (H1(a) / D30).
// One thread per output row computes dot(input, weight_row). Deliberately simple + identical
// across devices so the per-circuit timing is a fair relative signal (not a tuned kernel).

struct Params {
    n_in: u32,
    n_out: u32,
    _p0: u32,
    _p1: u32,
};

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn gemv(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.x;
    if (row >= params.n_out) { return; }
    let base = row * params.n_in;
    var acc = 0.0;
    for (var j = 0u; j < params.n_in; j = j + 1u) {
        acc = acc + input[j] * weight[base + j];
    }
    output[row] = acc;
}

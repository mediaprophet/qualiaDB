// Substrate GEMM — C = A · B, all row-major f32.
// A is m×k, B is k×n, C is m×n. One invocation computes one output element.
// This is the portable wgpu compute kernel the STEM substrate dispatches to when the
// measured capability matrix says the GPU wins (correctness-gated against the CPU
// reference). Deliberately simple (one thread per output, no tiling) — a correct first
// kernel; shared-memory tiling is a later optimization, not a correctness concern.

struct Dims {
    m: u32,
    k: u32,
    n: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read>       a: array<f32>;
@group(0) @binding(1) var<storage, read>       b: array<f32>;
@group(0) @binding(2) var<uniform>             d: Dims;
@group(0) @binding(3) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn gemm(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.x;
    let col = gid.y;
    if (row >= d.m || col >= d.n) {
        return;
    }
    var acc: f32 = 0.0;
    for (var l: u32 = 0u; l < d.k; l = l + 1u) {
        acc = acc + a[row * d.k + l] * b[l * d.n + col];
    }
    c[row * d.n + col] = acc;
}

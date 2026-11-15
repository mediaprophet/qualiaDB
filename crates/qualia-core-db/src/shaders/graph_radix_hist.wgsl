// Target: c:\github\qualiaDB\crates\qualia-core-db\src\shaders\graph_radix_hist.wgsl
// Stable LSD radix histogram: workgroup-local histogram to eliminate global atomic contention.

struct Params {
    n: u32,
    shift: u32,
    num_wgs: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> keys_in: array<u32>;
@group(0) @binding(1) var<storage, read_write> wg_hist: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

var<workgroup> local_hist: array<atomic<u32>, 256>;

fn digit_at(i: u32, shift: u32) -> u32 {
    let lo = keys_in[i * 2u];
    let hi = keys_in[i * 2u + 1u];
    if (shift < 32u) {
        return (lo >> shift) & 255u;
    }
    return (hi >> (shift - 32u)) & 255u;
}

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let t = lid.x;
    atomicStore(&local_hist[t], 0u);
    workgroupBarrier();

    let i = gid.x;
    if (i < params.n) {
        let d = digit_at(i, params.shift);
        atomicAdd(&local_hist[d], 1u);
    }
    workgroupBarrier();

    // Column-major by digit d so device prefix sum is contiguous:
    wg_hist[t * params.num_wgs + wid.x] = atomicLoad(&local_hist[t]);
}

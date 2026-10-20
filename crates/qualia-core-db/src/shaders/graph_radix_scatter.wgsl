// Target: c:\github\qualiaDB\crates\qualia-core-db\src\shaders\graph_radix_scatter.wgsl
// Stable intra-workgroup rank scatter for LSD radix sort.

struct Params {
    n: u32,
    shift: u32,
    num_wgs: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> keys_in: array<u32>;
@group(0) @binding(1) var<storage, read> idx_in: array<u32>;
@group(0) @binding(2) var<storage, read_write> keys_out: array<u32>;
@group(0) @binding(3) var<storage, read_write> idx_out: array<u32>;
@group(0) @binding(4) var<storage, read> wg_offsets: array<u32>;
@group(0) @binding(5) var<uniform> params: Params;

var<workgroup> wg_digits: array<u32, 256>;

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
    let i = gid.x;
    let t = lid.x;
    let valid = i < params.n;
    let d = select(0xFFFFFFFFu, digit_at(i, params.shift), valid);
    wg_digits[t] = d;
    workgroupBarrier();

    if (!valid) {
        return;
    }

    // Deterministic intra-workgroup rank: count threads u < t with identical digit d.
    var local_rank: u32 = 0u;
    for (var u: u32 = 0u; u < t; u = u + 1u) {
        local_rank = local_rank + select(0u, 1u, wg_digits[u] == d);
    }

    let base_offset = wg_offsets[d * params.num_wgs + wid.x];
    let pos = base_offset + local_rank;
    keys_out[pos * 2u] = keys_in[i * 2u];
    keys_out[pos * 2u + 1u] = keys_in[i * 2u + 1u];
    idx_out[pos] = idx_in[i];
}

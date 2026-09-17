// LSD radix scatter: place each key/index at the next slot for its digit.
// Unstable (atomics). Matches ingest's previous sort_unstable contract.

struct Params {
    n: u32,
    shift: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> keys_in: array<u32>;
@group(0) @binding(1) var<storage, read> idx_in: array<u32>;
@group(0) @binding(2) var<storage, read_write> keys_out: array<u32>;
@group(0) @binding(3) var<storage, read_write> idx_out: array<u32>;
@group(0) @binding(4) var<storage, read_write> offsets: array<atomic<u32>>;
@group(0) @binding(5) var<uniform> params: Params;

fn digit_at(i: u32, shift: u32) -> u32 {
    let lo = keys_in[i * 2u];
    let hi = keys_in[i * 2u + 1u];
    if (shift < 32u) {
        return (lo >> shift) & 255u;
    }
    return (hi >> (shift - 32u)) & 255u;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n) {
        return;
    }
    let d = digit_at(i, params.shift);
    let pos = atomicAdd(&offsets[d], 1u);
    keys_out[pos * 2u] = keys_in[i * 2u];
    keys_out[pos * 2u + 1u] = keys_in[i * 2u + 1u];
    idx_out[pos] = idx_in[i];
}

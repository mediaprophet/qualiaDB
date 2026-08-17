// LSD radix histogram: 256 bins for one 8-bit digit of a u64 key (lo/hi u32).
// Integer-only. No shader-int64 required.

struct Params {
    n: u32,
    shift: u32,
    _pad0: u32,
    _pad1: u32,
}

// Packed as [lo0, hi0, lo1, hi1, ...] — avoid vec2 array-stride surprises.
@group(0) @binding(0) var<storage, read> keys: array<u32>;
@group(0) @binding(1) var<storage, read_write> hist: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: Params;

fn digit_at(i: u32, shift: u32) -> u32 {
    let lo = keys[i * 2u];
    let hi = keys[i * 2u + 1u];
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
    atomicAdd(&hist[digit_at(i, params.shift)], 1u);
}

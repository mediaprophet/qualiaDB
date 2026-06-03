// GPU/NPU Sieve — portable bitmask-output version.
// Splits each 64-bit Quin field into lo/hi u32 pairs (no shader-int64 required,
// works on Vulkan, Metal, and DX12 without the shader-int64 capability).
// Output: dense 27-u32 bitmask — one bit per Quin, 850 Quins max per block.
// Core 2 decodes the bitmask on the CPU via trailing-zero bit-scan.

struct Quin {
    subject_lo:   u32, subject_hi:   u32,
    predicate_lo: u32, predicate_hi: u32,
    object_lo:    u32, object_hi:    u32,
    context_lo:   u32, context_hi:   u32,
    meta_lo:      u32, meta_hi:      u32,
    parity_lo:    u32, parity_hi:    u32,
}

struct FilterMask {
    lo: u32,
    hi: u32,
}

// Binding 0: flat Quin array (variable length — one full block = 850 Quins)
@group(0) @binding(0) var<storage, read>       quins:       array<Quin>;
// Binding 1: output bitmask — ceil(850 / 32) = 27 u32 words, atomically written
@group(0) @binding(1) var<storage, read_write> out_bitmask: array<atomic<u32>, 27>;
// Binding 2: 64-bit filter mask split into lo/hi u32 (matches Rust FilterMask64)
@group(0) @binding(2) var<uniform>             filter:      FilterMask;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index >= arrayLength(&quins) { return; }

    let meta_lo = quins[index].meta_lo;
    let meta_hi = quins[index].meta_hi;

    // Both halves of the 64-bit metadata field must satisfy the mask
    if (meta_lo & filter.lo) == filter.lo && (meta_hi & filter.hi) == filter.hi {
        let bucket    = index / 32u;
        let bit_shift = index % 32u;
        atomicOr(&out_bitmask[bucket], 1u << bit_shift);
    }
}

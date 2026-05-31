// The GPU/NPU Sieve Compute Shader
// This executes across the GPU cores to filter Quins by their 5th Vector Metadata bitmasks.

struct Quin {
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
    parity: u64,
};

@group(0) @binding(0) var<storage, read> quins: array<Quin>;
@group(0) @binding(1) var<storage, read_write> results: array<u32>; // 1 = match, 0 = no match

// The bitmask filter passed in via uniform
struct FilterMask {
    target_mask: u32,
};
@group(0) @binding(2) var<uniform> filter: FilterMask;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&quins)) {
        return;
    }
    
    // Extract the lower 16 bits of the 5th Vector (the validation bitmask)
    let quin_metadata = quins[index].metadata;
    let validation_mask = u32(quin_metadata & 0xFFFFu);
    
    // Perform Parallel Bit Extract / Bitwise AND
    if ((validation_mask & filter.target_mask) == filter.target_mask) {
        results[index] = 1u;
    } else {
        results[index] = 0u;
    }
}

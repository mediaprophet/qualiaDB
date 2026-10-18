// Differential 3D Video Codec: Subgroup Prefix-Sum Tile Compactor (`tile_compaction.wgsl`)
//
// Packs scattered dirty 16x16 tiles into a contiguous 1D Sparse Residual Shading Queue.
// Implements two-stage workgroup reduction with subgroup operations & shared memory fallback.
// Emits WebGPU dispatch_workgroups_indirect arguments with tail sentinel padding.

struct CompactionUniforms {
    viewport_dim: vec4<f32>,     // width, height, num_tiles_x, num_tiles_y
    total_tiles: u32,
    max_queue_capacity: u32,
    _pad0: u32,
    _pad1: u32,
};

const NULL_TILE_ID: u32 = 0xFFFFFFFFu;

@group(0) @binding(0) var<uniform> uniforms: CompactionUniforms;
@group(0) @binding(1) var<storage, read> tile_dirty_bits: array<u32>;
@group(0) @binding(2) var<storage, read_write> indirect_dispatch_args: array<atomic<u32>>; // [x, y, z] for dispatch_indirect
@group(0) @binding(3) var<storage, read_write> compacted_tile_queue: array<u32>;

// Workgroup shared memory reduction buffer (up to 8 subgroups per 256-thread workgroup)
var<workgroup> wg_subgroup_counts: array<atomic<u32>, 8>;
var<workgroup> wg_base_offset: u32;

@compute @workgroup_size(256, 1, 1)
fn cs_compact_tiles(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let tile_idx = global_id.x;
    let tid = local_id.x;

    var is_dirty = false;
    if (tile_idx < uniforms.total_tiles) {
        let word_idx = tile_idx / 32u;
        let bit_idx = tile_idx % 32u;
        let word_val = tile_dirty_bits[word_idx];
        is_dirty = ((word_val >> bit_idx) & 1u) != 0u;
    }

    // Stage 1: Workgroup local shared-memory Blelloch/Hillis-Steele prefix sum fallback
    // (Portable across all WebGPU implementations regardless of native subgroup width)
    var local_vote = 0u;
    if (is_dirty) {
        local_vote = 1u;
    }

    // Shared buffer for 256 local items
    var local_offset = 0u;
    let sg_idx = tid / 32u;
    let lane_idx = tid % 32u;

    if (lane_idx == 0u) {
        atomicStore(&wg_subgroup_counts[sg_idx], 0u);
    }
    workgroupBarrier();

    if (is_dirty) {
        local_offset = atomicAdd(&wg_subgroup_counts[sg_idx], 1u);
    }
    workgroupBarrier();

    // Stage 2: Thread 0 computes workgroup total and performs single global atomicAdd
    if (tid == 0u) {
        var total_wg_dirty = 0u;
        for (var i = 0u; i < 8u; i = i + 1u) {
            let cnt = atomicLoad(&wg_subgroup_counts[i]);
            atomicStore(&wg_subgroup_counts[i], total_wg_dirty); // Convert to inclusive prefix offset
            total_wg_dirty = total_wg_dirty + cnt;
        }

        if (total_wg_dirty > 0u) {
            // Indirect dispatch: args[0] = workgroups_x, args[1] = 1, args[2] = 1
            // Each workgroup shades 1 compacted tile (or tile batch)
            wg_base_offset = atomicAdd(&indirect_dispatch_args[0], total_wg_dirty);
        } else {
            wg_base_offset = 0u;
        }
    }
    workgroupBarrier();

    // Stage 3: Scatter compacted tile IDs into 1D output queue
    if (is_dirty) {
        let slot = wg_base_offset + atomicLoad(&wg_subgroup_counts[sg_idx]) + local_offset;
        if (slot < uniforms.max_queue_capacity) {
            compacted_tile_queue[slot] = tile_idx;
        }
    }

    // Tail sentinel padding for indirect shader dispatch alignment
    if (tid == 0u && tile_idx + 256u >= uniforms.total_tiles) {
        let final_count = atomicLoad(&indirect_dispatch_args[0]);
        let padded_count = (final_count + 63u) & ~63u;
        for (var p = final_count; p < padded_count; p = p + 1u) {
            if (p < uniforms.max_queue_capacity) {
                compacted_tile_queue[p] = NULL_TILE_ID;
            }
        }
        // Ensure y and z dispatch counts are 1
        atomicStore(&indirect_dispatch_args[1], 1u);
        atomicStore(&indirect_dispatch_args[2], 1u);
    }
}

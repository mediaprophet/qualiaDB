// ============================================================================
// QUALIA-CORE-DB — GPU-Driven Indirect Culling, 2-Phase HZB Occlusion & 4-Tier LOD
//
// Evaluates:
//   1. PGA Motor Transform of instance bounding sphere: p' = R p ~R + 2 (d ~R)_vec
//   2. 6-Plane Frustum Culling against world-space sphere (center, scaled_radius)
//   3. 2-Phase Hierarchical Z-Buffer (HZB) Depth-Pyramid Occlusion Culling
//   4. Screen-Space Projected Radius LOD Selection (LOD0..LOD3) modulated by
//      epistemic salience `alpha_salience` and spectral variance `sigma_spectral`
//      with 10% hysteresis deadband to prevent LOD popping
//   5. Multi-Modal Stream Compaction:
//      - Stream 0: Mesh Indirect Stream (LOD0..LOD2, q == 0)
//      - Stream 1: Holographic SDF Proxy Stream (q > 0 or rep_policy == 2)
//      - Stream 2: Surfel / Octahedral Impostor Stream (LOD3 or rep_policy == 3/4)
// ============================================================================

struct CullUniforms {
    view_proj: mat4x4<f32>,
    view_matrix: mat4x4<f32>,
    frustum_planes: array<vec4<f32>, 6>,
    camera_pos_and_focal_px: vec4<f32>,     // xyz = camera world pos, w = cot(fov_y * 0.5) * 0.5 * viewport_height
    viewport_and_hzb_dims: vec4<f32>,       // xy = viewport (w, h), zw = hzb mip0 (w, h)
    lod_thresholds_px: vec4<f32>,           // x = LOD0 (128), y = LOD1 (48), z = LOD2 (16), w = min_cull_px (1.5)
    counts_and_flags: vec4<u32>,            // x = instance_count, y = hzb_mip_count, z = enable_hzb (0/1), w = hysteresis_bits
};

// 80-byte GPU-aligned InstanceCullDescriptor (5 x 16-byte words)
struct InstanceCullDescriptorGpu {
    // PGA unit motor real rotor R = (e023, e031, e012, scalar)
    r: vec4<f32>,
    // PGA unit motor dual part D = (e01, e02, e03, e0123)
    d: vec4<f32>,
    // xyz = non-uniform scale, w = local bounding sphere radius
    scale_radius: vec4<f32>,
    // x = asset_id, y = (material_id | (rep_policy << 16)),
    // z = rig_offset, w = (proc_header_idx | (tensor_q_v << 16))
    packed_ids: vec4<u32>,
    // x = alpha_salience, y = sigma_spectral, z = prev_lod_tier_f32, w = reserved
    epistemic_and_lod: vec4<f32>,
};

struct SubmeshSliceGpu {
    index_count: u32,
    first_index: u32,
    base_vertex: i32,
    material_slot: u32,
    aabb_min: vec4<f32>,
    aabb_max: vec4<f32>,
};

struct ResidentAssetEntryGpu {
    lods: array<SubmeshSliceGpu, 4>,
    impostor_atlas_uv: vec4<f32>,
    surfel_slice_offset: u32,
    surfel_count: u32,
    sdf_brick_offset: u32,
    flags: u32,
};

struct DrawIndexedIndirectCommandGpu {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

struct StreamCounters {
    mesh_draw_count: atomic<u32>,
    holo_proxy_count: atomic<u32>,
    surfel_impostor_count: atomic<u32>,
    occluded_count: atomic<u32>,
};

const REP_AUTO: u32 = 0u;
const REP_EXPLICIT_MESH_LOD: u32 = 1u;
const REP_HOLOGRAPHIC_SDF_PROXY: u32 = 2u;
const REP_GAUSSIAN_SURFEL_CLOUD: u32 = 3u;
const REP_OCTAHEDRAL_IMPOSTOR: u32 = 4u;

@group(0) @binding(0) var<uniform> u_cull: CullUniforms;
@group(0) @binding(1) var<storage, read> instances: array<InstanceCullDescriptorGpu>;
@group(0) @binding(2) var<storage, read> assets: array<ResidentAssetEntryGpu>;
@group(0) @binding(3) var<storage, read_write> draw_commands: array<DrawIndexedIndirectCommandGpu>;
@group(0) @binding(4) var<storage, read_write> visible_mesh_instances: array<u32>;
@group(0) @binding(5) var<storage, read_write> visible_holo_instances: array<u32>;
@group(0) @binding(6) var<storage, read_write> visible_surfel_instances: array<u32>;
@group(0) @binding(7) var<storage, read_write> counters: StreamCounters;
@group(0) @binding(8) var hzb_pyramid: texture_2d<f32>;

fn pga_transform_origin(r: vec4<f32>, d: vec4<f32>) -> vec3<f32> {
    let rv = r.xyz;
    let rw = r.w;
    let dv = d.xyz;
    let dw = d.w;
    return 2.0 * (rw * dv - dw * rv + cross(dv, rv));
}

fn pga_transform_point_local(r: vec4<f32>, d: vec4<f32>, p: vec3<f32>) -> vec3<f32> {
    let rv = r.xyz;
    let rw = r.w;
    let dv = d.xyz;
    let dw = d.w;
    let t_rot = 2.0 * cross(rv, p);
    let p_rot = p + rw * t_rot + cross(rv, t_rot);
    let translation = 2.0 * (rw * dv - dw * rv + cross(dv, rv));
    return p_rot + translation;
}

fn sphere_passes_frustum(center: vec3<f32>, radius: f32) -> bool {
    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        let plane = u_cull.frustum_planes[i];
        let signed_dist = dot(plane.xyz, center) + plane.w;
        if (signed_dist < -radius) {
            return false;
        }
    }
    return true;
}

fn sphere_passes_hzb(world_center: vec3<f32>, radius: f32) -> bool {
    if (u_cull.counts_and_flags.z == 0u) {
        return true;
    }
    let view_pos = (u_cull.view_matrix * vec4<f32>(world_center, 1.0)).xyz;
    // Right-handed view space: camera looks down -Z, or +Z depending on convention.
    let dist_z = length(world_center - u_cull.camera_pos_and_focal_px.xyz);
    if (dist_z <= radius + 0.25) {
        // Camera is inside or near the bounding sphere — never occlude.
        return true;
    }
    let clip = u_cull.view_proj * vec4<f32>(world_center, 1.0);
    if (clip.w <= 1e-4) {
        return true;
    }
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return true;
    }

    let proj_radius_px = (radius * u_cull.camera_pos_and_focal_px.w) / max(dist_z, 0.1);
    let footprint_px = max(proj_radius_px * 2.0, 1.0);
    let max_mip = f32(max(u_cull.counts_and_flags.y, 1u) - 1u);
    let mip_level = clamp(ceil(log2(footprint_px)), 0.0, max_mip);
    let mip_i32 = i32(mip_level);
    let mip_dims = vec2<f32>(textureDimensions(hzb_pyramid, mip_i32));
    let texel = clamp(vec2<i32>(uv * mip_dims), vec2<i32>(0), vec2<i32>(mip_dims) - vec2<i32>(1));
    let hzb_farthest_depth = textureLoad(hzb_pyramid, texel, mip_i32).r;

    // Nearest point of sphere toward camera in clip space
    let cam_dir = normalize(u_cull.camera_pos_and_focal_px.xyz - world_center);
    let nearest_world = world_center + cam_dir * radius;
    let nearest_clip = u_cull.view_proj * vec4<f32>(nearest_world, 1.0);
    let nearest_depth = clamp(nearest_clip.z / max(nearest_clip.w, 1e-5), 0.0, 1.0);

    // Standard [0, 1] depth where smaller is closer: occluded only if nearest_depth > hzb_farthest_depth
    return nearest_depth <= (hzb_farthest_depth + 1e-4);
}

fn select_lod_tier(proj_radius_px: f32, prev_lod: u32) -> u32 {
    let t0 = u_cull.lod_thresholds_px.x; // 128.0
    let t1 = u_cull.lod_thresholds_px.y; // 48.0
    let t2 = u_cull.lod_thresholds_px.z; // 16.0
    let hyst = 0.10; // 10% hysteresis deadband

    var tier: u32 = 3u;
    if (proj_radius_px >= t0) {
        tier = 0u;
    } else if (proj_radius_px >= t1) {
        tier = 1u;
    } else if (proj_radius_px >= t2) {
        tier = 2u;
    } else {
        tier = 3u;
    }

    // Apply hysteresis around threshold boundaries to prevent frame-to-frame oscillation
    if (prev_lod == 0u && tier == 1u && proj_radius_px >= t0 * (1.0 - hyst)) {
        return 0u;
    }
    if (prev_lod == 1u && tier == 0u && proj_radius_px < t0 * (1.0 + hyst)) {
        return 1u;
    }
    if (prev_lod == 1u && tier == 2u && proj_radius_px >= t1 * (1.0 - hyst)) {
        return 1u;
    }
    if (prev_lod == 2u && tier == 1u && proj_radius_px < t1 * (1.0 + hyst)) {
        return 2u;
    }
    if (prev_lod == 2u && tier == 3u && proj_radius_px >= t2 * (1.0 - hyst)) {
        return 2u;
    }
    if (prev_lod == 3u && tier == 2u && proj_radius_px < t2 * (1.0 + hyst)) {
        return 3u;
    }
    return tier;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let instance_count = u_cull.counts_and_flags.x;
    if (idx >= instance_count) {
        return;
    }

    let inst = instances[idx];
    let max_scale = max(inst.scale_radius.x, max(inst.scale_radius.y, inst.scale_radius.z));
    let world_radius = max(inst.scale_radius.w * max_scale, 0.001);
    let world_center = pga_transform_origin(inst.r, inst.d);

    // 1. 6-Plane Frustum Culling
    if (!sphere_passes_frustum(world_center, world_radius)) {
        return;
    }

    // 2. Screen-space projected radius modulated by epistemic salience & spectral uncertainty
    let dist = max(length(world_center - u_cull.camera_pos_and_focal_px.xyz), 0.1);
    let raw_proj_px = (world_radius * u_cull.camera_pos_and_focal_px.w) / dist;
    let alpha_salience = clamp(inst.epistemic_and_lod.x, 0.05, 4.0);
    let sigma_spectral = max(inst.epistemic_and_lod.y, 0.0);
    let epistemic_weight = sqrt(alpha_salience) / max(1.0 + 0.35 * sigma_spectral, 0.5);
    let effective_proj_px = raw_proj_px * epistemic_weight;

    if (effective_proj_px < u_cull.lod_thresholds_px.w) {
        return;
    }

    // 3. 2-Phase HZB Depth-Pyramid Occlusion Culling
    if (!sphere_passes_hzb(world_center, world_radius)) {
        atomicAdd(&counters.occluded_count, 1u);
        return;
    }

    // 4. Unpack representation policy & tensor valence (q, v)
    let asset_id = inst.packed_ids.x;
    let rep_policy = (inst.packed_ids.y >> 16u) & 0xFFFFu;
    let tensor_q_v = (inst.packed_ids.w >> 16u) & 0xFFFFu;
    let tensor_q = tensor_q_v & 0xFFu;
    let prev_lod = min(u32(max(inst.epistemic_and_lod.z, 0.0)), 3u);
    let lod_tier = select_lod_tier(effective_proj_px, prev_lod);

    // 5. Route to Holographic SDF Proxy Stream if q > 0 or policy == HolographicSdfProxy
    if (tensor_q > 0u || rep_policy == REP_HOLOGRAPHIC_SDF_PROXY) {
        let slot = atomicAdd(&counters.holo_proxy_count, 1u);
        visible_holo_instances[slot] = idx;
        return;
    }

    // 6. Route to Surfel / Octahedral Impostor Stream if LOD3 or policy == Surfel/Impostor
    if (lod_tier >= 3u || rep_policy == REP_GAUSSIAN_SURFEL_CLOUD || rep_policy == REP_OCTAHEDRAL_IMPOSTOR) {
        let slot = atomicAdd(&counters.surfel_impostor_count, 1u);
        visible_surfel_instances[slot] = idx;
        return;
    }

    // 7. Route to Mesh Indirect Stream (LOD0..LOD2)
    let asset = assets[asset_id];
    var slice = asset.lods[lod_tier];
    if (slice.index_count == 0u) {
        slice = asset.lods[0u];
    }
    if (slice.index_count == 0u) {
        return;
    }

    let cmd_slot = atomicAdd(&counters.mesh_draw_count, 1u);
    visible_mesh_instances[cmd_slot] = idx;
    draw_commands[cmd_slot] = DrawIndexedIndirectCommandGpu(
        slice.index_count,
        1u,
        slice.first_index,
        slice.base_vertex,
        idx,
    );
}

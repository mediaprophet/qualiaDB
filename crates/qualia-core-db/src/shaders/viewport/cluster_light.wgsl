// Clustered Forward / Deferred Light Culling (`cluster_light.wgsl`)
//
// Subdivides the view frustum into a 3D grid of clusters (16 x 9 x 24):
// - Pass 1 (Grid Generation): Evaluates view-space AABBs for each cluster tile.
// - Pass 2 (Light Culling): Intersects point and spot lights with cluster AABBs.
// - Outputs light offset and count per cluster to drive high-performance zero-divergence lighting.

struct ClusterAabb {
    min_point: vec4<f32>,
    max_point: vec4<f32>,
};

struct PointLight {
    position_view: vec4<f32>,   // xyz in view space, w: radius in meters
    color_intensity: vec4<f32>, // rgb color, w: luminous intensity in lumens/cd
};

struct ClusterGridParams {
    grid_dim: vec4<u32>,        // x: 16, y: 9, z: 24, w: total clusters (3456)
    view_size: vec4<f32>,       // screen width, screen height, near plane, far plane
    inverse_projection: mat4x4<f32>,
    light_count: u32,
    max_lights_per_cluster: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<uniform> params: ClusterGridParams;
@group(0) @binding(1) var<storage, read_write> cluster_aabbs: array<ClusterAabb>;
@group(0) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(0) @binding(3) var<storage, read_write> cluster_light_indices: array<u32>;
// Cluster grid record format: vec2<u32> -> (offset_into_index_buffer, light_count)
@group(0) @binding(4) var<storage, read_write> cluster_grid: array<vec2<u32>>;

// Converts screen-space normalized coordinates to view-space position on near plane
fn screen_to_view(screen_coord: vec2<f32>) -> vec3<f32> {
    // Convert [0, 1] UV to NDC [-1, 1]
    let ndc = vec4<f32>(screen_coord.x * 2.0 - 1.0, (1.0 - screen_coord.y) * 2.0 - 1.0, 1.0, 1.0);
    let view = params.inverse_projection * ndc;
    return view.xyz / view.w;
}

// ── Pass 1: Build Cluster AABBs ───────────────────────────────────────────────

@compute @workgroup_size(16, 9, 1)
fn cluster_build_aabbs(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tile_x = global_id.x;
    let tile_y = global_id.y;
    let tile_z = global_id.z;

    if (tile_x >= params.grid_dim.x || tile_y >= params.grid_dim.y || tile_z >= params.grid_dim.z) {
        return;
    }

    let cluster_idx = tile_x + tile_y * params.grid_dim.x + tile_z * (params.grid_dim.x * params.grid_dim.y);

    let tile_dim = vec2<f32>(f32(params.grid_dim.x), f32(params.grid_dim.y));
    let min_uv = vec2<f32>(f32(tile_x), f32(tile_y)) / tile_dim;
    let max_uv = vec2<f32>(f32(tile_x + 1u), f32(tile_y + 1u)) / tile_dim;

    // View-space ray directions for four corners of the cluster tile
    let p_min_near = screen_to_view(min_uv);
    let p_max_near = screen_to_view(max_uv);

    // Exponential Z slicing: z_near * (z_far / z_near)^(slice / num_slices)
    let near = params.view_size.z;
    let far = params.view_size.w;
    let slice_ratio_near = f32(tile_z) / f32(params.grid_dim.z);
    let slice_ratio_far = f32(tile_z + 1u) / f32(params.grid_dim.z);

    let z_cluster_near = near * pow(far / near, slice_ratio_near);
    let z_cluster_far = near * pow(far / near, slice_ratio_far);

    // Scale corner rays by ratio of target Z to near Z
    let min_p_near_scaled = p_min_near * (z_cluster_near / p_min_near.z);
    let min_p_far_scaled = p_min_near * (z_cluster_far / p_min_near.z);
    let max_p_near_scaled = p_max_near * (z_cluster_near / p_max_near.z);
    let max_p_far_scaled = p_max_near * (z_cluster_far / p_max_near.z);

    let aabb_min = min(min(min_p_near_scaled, min_p_far_scaled), min(max_p_near_scaled, max_p_far_scaled));
    let aabb_max = max(max(min_p_near_scaled, min_p_far_scaled), max(max_p_near_scaled, max_p_far_scaled));

    cluster_aabbs[cluster_idx].min_point = vec4<f32>(aabb_min, 0.0);
    cluster_aabbs[cluster_idx].max_point = vec4<f32>(aabb_max, 0.0);
}

// ── Pass 2: Cull Point Lights into Clusters ──────────────────────────────────

// Sphere-AABB distance test
fn sphere_intersects_aabb(center: vec3<f32>, radius: f32, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> bool {
    let closest = clamp(center, aabb_min, aabb_max);
    let delta = center - closest;
    return dot(delta, delta) <= (radius * radius);
}

@compute @workgroup_size(64, 1, 1)
fn cluster_cull_lights(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cluster_idx = global_id.x;
    if (cluster_idx >= params.grid_dim.w) {
        return;
    }

    let aabb = cluster_aabbs[cluster_idx];
    let aabb_min = aabb.min_point.xyz;
    let aabb_max = aabb.max_point.xyz;

    var visible_light_count = 0u;
    let offset = cluster_idx * params.max_lights_per_cluster;

    for (var i = 0u; i < params.light_count; i = i + 1u) {
        if (visible_light_count >= params.max_lights_per_cluster) {
            break;
        }

        let light = point_lights[i];
        let light_pos = light.position_view.xyz;
        let light_radius = light.position_view.w;

        if (sphere_intersects_aabb(light_pos, light_radius, aabb_min, aabb_max)) {
            cluster_light_indices[offset + visible_light_count] = i;
            visible_light_count = visible_light_count + 1u;
        }
    }

    cluster_grid[cluster_idx] = vec2<u32>(offset, visible_light_count);
}

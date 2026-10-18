// Differential 3D Video Codec: Predictive 2.5D Temporal Warp Pass (`temporal_warp.wgsl`)
//
// Pillars:
// - Reprojects history radiance along analytical motion vectors (P-Frame)
// - Evaluates depth consistency & disocclusion holes
// - Applies closed-form Beer-Lambert participating media transmittance (D_10)
// - Flags dirty/high-entropy pixels into tile entropy bitmask

struct WarpUniforms {
    current_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    inv_current_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,       // xyz: world pos, w: frame_index
    viewport_dim: vec4<f32>,     // width, height, 1/width, 1/height
    codec_params: vec4<f32>,     // x: depth_threshold, y: normal_threshold, z: intra_refresh_rate, w: lamport_epoch
};

// 48-byte DeltaNQuin layout (3 x vec4<u32> = 12 x u32 = 48 bytes)
struct DeltaNQuinGpu {
    words0_3: vec4<u32>, // [0]: subject_id, [1]: op(u16)|qp(u8)|branch_med(u8), [2]: pos.x, [3]: pos.y
    words4_7: vec4<u32>, // [4]: pos.z, [5]: vel.xy(f16x2), [6]: vel.z|lod(f16x2), [7]: rot_or_sh
    words8_11: vec4<u32>, // [8]: cell_id, [9]: epoch, [10]: dirty_tile_mask, [11]: parity
};

@group(0) @binding(0) var<uniform> uniforms: WarpUniforms;
@group(0) @binding(1) var history_color_tex: texture_2d<f32>;
@group(0) @binding(2) var history_depth_tex: texture_depth_2d;
@group(0) @binding(3) var current_depth_tex: texture_depth_2d;
@group(0) @binding(4) var motion_vector_tex: texture_2d<f32>;
@group(0) @binding(5) var<storage, read> delta_quins: array<DeltaNQuinGpu>;
@group(0) @binding(6) var<storage, read_write> tile_dirty_bits: array<atomic<u32>>;
@group(0) @binding(7) var warped_color_out: texture_storage_2d<rgba16float, write>;

// Unpacks 48-byte DeltaNQuin fields
fn unpack_delta_pos(quin: DeltaNQuinGpu) -> vec3<f32> {
    return vec3<f32>(
        bitcast<f32>(quin.words0_3.z),
        bitcast<f32>(quin.words0_3.w),
        bitcast<f32>(quin.words4_7.x)
    );
}

fn unpack_salience_qp(quin: DeltaNQuinGpu) -> u32 {
    return (quin.words0_3.y >> 16u) & 0xFFu;
}

fn unpack_medium_id(quin: DeltaNQuinGpu) -> u32 {
    return (quin.words0_3.y >> 28u) & 0x0Fu;
}

// Reconstructs world-space position from UV and depth
fn reconstruct_world_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let clip = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth, 1.0);
    let world_w = uniforms.inv_current_view_proj * clip;
    return world_w.xyz / max(world_w.w, 1e-6);
}

// Analytical Beer-Lambert transmittance for D_10 medium
fn evaluate_medium_transmittance(medium_id: u32, distance_m: f32) -> vec3<f32> {
    var sigma_t = vec3<f32>(0.0);
    if (medium_id == 1u) {
        // Clear Water
        sigma_t = vec3<f32>(0.035, 0.015, 0.005);
    } else if (medium_id == 2u) {
        // Turbid Blackwater / Humic
        sigma_t = vec3<f32>(0.85, 0.45, 0.22);
    } else if (medium_id == 3u) {
        // Mist / Fog
        sigma_t = vec3<f32>(0.08, 0.08, 0.08);
    }
    return exp(-sigma_t * distance_m);
}

@compute @workgroup_size(16, 16, 1)
fn cs_temporal_warp(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = vec2<u32>(u32(uniforms.viewport_dim.x), u32(uniforms.viewport_dim.y));
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let pixel_coords = vec2<i32>(global_id.xy);
    let uv = (vec2<f32>(global_id.xy) + 0.5) * uniforms.viewport_dim.zw;

    // Sample current depth
    let curr_depth = textureLoad(current_depth_tex, pixel_coords, 0);
    if (curr_depth >= 1.0) {
        // Skybox background: mark clean, write transparent
        textureStore(warped_color_out, pixel_coords, vec4<f32>(0.0));
        return;
    }

    // Sample motion vector
    let mv = textureLoad(motion_vector_tex, pixel_coords, 0).xy;
    let prev_uv = uv - mv;

    var is_disoccluded = false;
    var prev_color = vec4<f32>(0.0);

    // Boundary check
    if (prev_uv.x < 0.0 || prev_uv.x > 1.0 || prev_uv.y < 0.0 || prev_uv.y > 1.0) {
        is_disoccluded = true;
    } else {
        let prev_coords = vec2<i32>(prev_uv * uniforms.viewport_dim.xy);
        let hist_depth = textureLoad(history_depth_tex, prev_coords, 0);

        // Depth consistency threshold
        let depth_delta = abs(curr_depth - hist_depth);
        if (depth_delta > uniforms.codec_params.x) {
            is_disoccluded = true;
        } else {
            prev_color = textureLoad(history_color_tex, prev_coords, 0);
        }
    }

    // Reconstruct world position and apply participating medium attenuation
    let world_pos = reconstruct_world_pos(uv, curr_depth);
    let view_dist = length(world_pos - uniforms.camera_pos.xyz);
    let default_medium = 0u; // Homogeneous atmosphere
    let transmittance = evaluate_medium_transmittance(default_medium, view_dist);
    prev_color = vec4<f32>(prev_color.rgb * transmittance, prev_color.a);

    // Rolling intra-refresh: force 2-4% slice to be intra-coded
    let tile_x = global_id.x / 16u;
    let tile_y = global_id.y / 16u;
    let tile_idx = tile_y * ((dims.x + 15u) / 16u) + tile_x;

    let refresh_interval = u32(1.0 / max(uniforms.codec_params.z, 0.01));
    let frame_mod = u32(uniforms.camera_pos.w) % refresh_interval;
    let is_rolling_refresh = ((tile_x + tile_y) % refresh_interval) == frame_mod;

    if (is_disoccluded || is_rolling_refresh) {
        // Mark tile dirty in atomic bitmask
        let word_idx = tile_idx / 32u;
        let bit_mask = 1u << (tile_idx % 32u);
        atomicOr(&tile_dirty_bits[word_idx], bit_mask);
    }

    textureStore(warped_color_out, pixel_coords, prev_color);
}

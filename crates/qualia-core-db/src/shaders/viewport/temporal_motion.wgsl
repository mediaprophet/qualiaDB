// Screen-space camera-motion producer for temporal accumulation.
//
// Reconstructs current world position from depth buffer and unprojects using
// inv_current_view_proj, then reprojects into previous frame clip-space using
// previous_view_proj to calculate screen-space UV delta: motion = prev_uv - curr_uv.

struct CameraMotionParams {
    inv_current_view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    viewport: vec4<f32>, // x = width, y = height, z = 1/width, w = 1/height
};

@group(0) @binding(0) var scene_depth: texture_depth_2d;
@group(0) @binding(1) var<uniform> motion_params: CameraMotionParams;
@group(0) @binding(2) var depth_sampler: sampler;

fn fullscreen_position(vertex_index: u32) -> vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@vertex
fn temporal_motion_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_position(vertex_index);
}

@fragment
fn temporal_motion_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let uv = (floor(position.xy) + vec2<f32>(0.5, 0.5)) * motion_params.viewport.zw;
    let depth = textureSampleLevel(scene_depth, depth_sampler, uv, 0);

    // NDC: x in [-1, 1], y in [-1, 1] (y up, uv y down: ndc_y = 1.0 - 2.0 * uv.y)
    let ndc = vec4<f32>(
        uv.x * 2.0 - 1.0,
        1.0 - uv.y * 2.0,
        depth,
        1.0
    );

    let world_h = motion_params.inv_current_view_proj * ndc;
    if (abs(world_h.w) <= 1e-6) {
        return vec2<f32>(0.0, 0.0);
    }
    let world_pos = world_h.xyz / world_h.w;

    let prev_clip = motion_params.previous_view_proj * vec4<f32>(world_pos, 1.0);
    if (prev_clip.w <= 1e-6) {
        return vec2<f32>(0.0, 0.0);
    }

    let prev_ndc_x = prev_clip.x / prev_clip.w;
    let prev_ndc_y = prev_clip.y / prev_clip.w;

    let prev_u = (prev_ndc_x + 1.0) * 0.5;
    let prev_v = (1.0 - prev_ndc_y) * 0.5;

    return vec2<f32>(prev_u - uv.x, prev_v - uv.y);
}

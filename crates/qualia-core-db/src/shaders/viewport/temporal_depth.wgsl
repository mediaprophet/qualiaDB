// Renderer-owned linear-depth producer for temporal resolve.
//
// The authoritative scene depth attachment is a depth texture, not a colour
// producer. This pass samples that attachment and writes a portable float
// target in view-space units. It therefore supplies genuine depth while
// leaving motion vectors and reactive coverage as explicit host contracts.

struct DepthProducerParams {
    near_plane: f32,
    far_plane: f32,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var scene_depth: texture_depth_2d;
@group(0) @binding(1) var<uniform> depth_params: DepthProducerParams;

fn fullscreen_position(vertex_index: u32) -> vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@vertex
fn temporal_depth_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_position(vertex_index);
}

@fragment
fn temporal_depth_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(vec2<f32>(textureDimensions(scene_depth)));
    let pixel = clamp(
        vec2<i32>(floor(position.xy)),
        vec2<i32>(0),
        dimensions - vec2<i32>(1)
    );
    let depth = textureLoad(scene_depth, pixel, 0);
    let range = max(
        depth_params.far_plane
            - depth * (depth_params.far_plane - depth_params.near_plane),
        0.000001
    );
    let linear_depth = depth_params.near_plane * depth_params.far_plane / range;
    return vec4<f32>(linear_depth, 0.0, 0.0, 1.0);
}

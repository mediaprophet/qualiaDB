// Renderer-owned temporal resolve. The producer contract is deliberately explicit:
// current scene colour, current linear depth, motion vectors, and reactive mask are
// all supplied by the caller. There is no fallback producer in this shader.

struct TemporalParams {
    // xy = dimensions, zw = reciprocal dimensions.
    viewport: vec4<f32>,
    // x = depth threshold, y = maximum accepted motion, z = maximum history weight,
    // w = history-valid bit + 2 * reset-history bit.
    controls: vec4<f32>,
};

@group(0) @binding(0) var current_scene: texture_2d<f32>;
@group(0) @binding(1) var history_scene: texture_2d<f32>;
@group(0) @binding(2) var current_linear_depth: texture_2d<f32>;
@group(0) @binding(3) var history_linear_depth: texture_2d<f32>;
@group(0) @binding(4) var motion_vectors: texture_2d<f32>;
@group(0) @binding(5) var reactive_mask: texture_2d<f32>;
@group(0) @binding(6) var<uniform> temporal: TemporalParams;

fn fullscreen_position(vertex_index: u32) -> vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

fn clamp_pixel(pixel: vec2<i32>, dimensions: vec2<i32>) -> vec2<i32> {
    return clamp(pixel, vec2<i32>(0), dimensions - vec2<i32>(1));
}

fn finite(value: f32) -> bool {
    // NaN is unequal to itself; the upper bound rejects infinities without relying
    // on optional floating-point classification builtins.
    return value == value && abs(value) < 3.402823e+37;
}

fn finite2(value: vec2<f32>) -> bool {
    return finite(value.x) && finite(value.y);
}

@vertex
fn temporal_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_position(vertex_index);
}

@fragment
fn temporal_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(vec2<f32>(textureDimensions(current_scene)));
    let current_pixel = clamp_pixel(vec2<i32>(floor(position.xy)), dimensions);
    let current = textureLoad(current_scene, current_pixel, 0);

    let current_depth = textureLoad(current_linear_depth, current_pixel, 0).r;
    let motion = textureLoad(motion_vectors, current_pixel, 0).xy;
    let reactive = textureLoad(reactive_mask, current_pixel, 0).r;

    let history_bit = temporal.controls.w - floor(temporal.controls.w * 0.5) * 2.0;
    let history_valid = history_bit >= 0.5;
    let reset_history = floor(temporal.controls.w * 0.5) >= 0.5;
    let motion_valid = finite2(motion);
    let safe_motion = select(vec2<f32>(0.0), motion, motion_valid);
    let history_pixel_float = vec2<f32>(current_pixel) + safe_motion * temporal.viewport.xy;
    let history_in_bounds = finite2(history_pixel_float)
        && all(history_pixel_float >= vec2<f32>(0.0))
        && all(history_pixel_float < temporal.viewport.xy);
    let history_pixel = clamp_pixel(vec2<i32>(floor(history_pixel_float)), dimensions);
    let history = textureLoad(history_scene, history_pixel, 0);
    let history_depth = textureLoad(history_linear_depth, history_pixel, 0).r;
    let depth_valid = finite(current_depth)
        && finite(history_depth)
        && abs(current_depth - history_depth) <= temporal.controls.x;
    let reactive_valid = finite(reactive);
    let safe_reactive = select(1.0, reactive, reactive_valid);
    let motion_length = length(safe_motion);
    let motion_confidence = 1.0 - clamp(
        motion_length / max(temporal.controls.y, 0.0001),
        0.0,
        1.0
    );
    let history_weight = clamp(
        (1.0 - clamp(safe_reactive, 0.0, 1.0))
            * motion_confidence
            * temporal.controls.z,
        0.0,
        temporal.controls.z
    );

    let accept_history = history_valid
        && !reset_history
        && history_in_bounds
        && depth_valid
        && reactive_valid
        && motion_valid;
    return select(
        current,
        mix(current, history, history_weight),
        accept_history
    );
}

// Publication is a separate pass so the current linear-depth producer can use
// any float colour format supported by its owner. The helper stores a portable
// RGBA16F red-channel history target.
@group(0) @binding(7) var publish_linear_depth: texture_2d<f32>;

@fragment
fn publish_depth_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(vec2<f32>(textureDimensions(publish_linear_depth)));
    let pixel = clamp_pixel(vec2<i32>(floor(position.xy)), dimensions);
    let depth = textureLoad(publish_linear_depth, pixel, 0).r;
    return vec4<f32>(depth, 0.0, 0.0, 1.0);
}

struct TemporalOutputParams {
    exposure: f32,
    surface_is_srgb: f32,
    _padding0: f32,
    _padding1: f32,
    white_balance_gains: vec3<f32>,
    _padding2: f32,
};

@group(0) @binding(0) var resolved_scene: texture_2d<f32>;
@group(0) @binding(1) var<uniform> output_params: TemporalOutputParams;

@vertex
fn temporal_output_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@fragment
fn temporal_output_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(vec2<f32>(textureDimensions(resolved_scene)));
    let pixel = clamp(
        vec2<i32>(floor(position.xy)),
        vec2<i32>(0),
        dimensions - vec2<i32>(1)
    );
    let scene_linear = textureLoad(resolved_scene, pixel, 0).rgb;
    let output = qualia_sdr_output(
        scene_linear,
        output_params.exposure,
        output_params.white_balance_gains,
        output_params.surface_is_srgb
    );
    return vec4<f32>(output, 1.0);
}

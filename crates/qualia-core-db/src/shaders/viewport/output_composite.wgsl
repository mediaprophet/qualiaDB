struct OutputParams {
    exposure: f32,
    surface_is_srgb: f32,
    _padding0: f32,
    _padding1: f32,
    white_balance_gains: vec3<f32>,
    _padding2: f32,
};

@group(0) @binding(0) var output_sampler: sampler;
@group(0) @binding(1) var scene_texture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> output_params: OutputParams;

fn output_fullscreen_pos(vertex_index: u32) -> vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@vertex
fn output_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return output_fullscreen_pos(vertex_index);
}

@fragment
fn output_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(scene_texture));
    let uv = position.xy / dimensions;
    let scene_linear = textureSample(scene_texture, output_sampler, uv).rgb;
    let output = qualia_sdr_output(
        scene_linear,
        output_params.exposure,
        output_params.white_balance_gains,
        output_params.surface_is_srgb
    );
    return vec4<f32>(output, 1.0);
}

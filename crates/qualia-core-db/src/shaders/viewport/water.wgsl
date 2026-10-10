// HMC water surface — a bounded transparent surface that consumes the shared
// scene depth contract without writing depth. Terrain and entities therefore
// retain ownership of authoritative visibility and picking.

struct Camera {
    view_projection: mat4x4<f32>,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    tensor_mode: u32,
    _padding0: vec4<f32>,
    _padding1: vec4<f32>,
    _padding2: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;

struct WaterUniform {
    colour_alpha: vec4<f32>,
    wave: vec4<f32>, // time, amplitude, frequency, shore-width
    viewport: vec4<f32>, // width, height, depth epsilon, reserved
};
@group(2) @binding(0) var<uniform> water: WaterUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    let local = position + vec3<f32>(
        0.0,
        sin((position.x + water.wave.x) * water.wave.z)
            * cos((position.z - water.wave.x * 0.73) * water.wave.z * 0.83)
            * water.wave.y,
        0.0,
    );
    output.clip_position = camera.view_projection * model * vec4<f32>(local, 1.0);
    output.uv = uv;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // The render pass uses the same scene depth attachment with LessEqual and
    // depth writes disabled. This second guard makes the ownership explicit in
    // the shader contract and leaves a small tolerance for raster precision.
    let water_depth = input.clip_position.z / input.clip_position.w;
    if (water_depth > 1.0 + water.viewport.z || water_depth < 0.0) {
        discard;
    }

    let ripple = 0.5 + 0.5 * sin((input.uv.x + input.uv.y) * 18.0 + water.wave.x * 1.7);
    let tint = mix(water.colour_alpha.rgb * 0.82, water.colour_alpha.rgb * 1.12, ripple);
    return vec4<f32>(tint, water.colour_alpha.a);
}

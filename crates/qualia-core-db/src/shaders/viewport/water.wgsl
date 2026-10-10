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
    quality: vec4<f32>, // tier, reflection, foam, shore response
    viewport: vec4<f32>, // width, height, depth epsilon, reserved
};
@group(2) @binding(0) var<uniform> water: WaterUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) stable_normal: vec3<f32>,
};

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    let phase_x = (position.x + water.wave.x) * water.wave.z;
    let phase_z = (position.z - water.wave.x * 0.73) * water.wave.z * 0.83;
    let sin_x = sin(phase_x);
    let cos_x = cos(phase_x);
    let sin_z = sin(phase_z);
    let cos_z = cos(phase_z);
    let height = sin_x * cos_z * water.wave.y;
    // Analytic derivatives keep normals stable across tessellation and avoid a
    // second geometry upload just to carry a normal stream.
    let d_height_x = cos_x * cos_z * water.wave.y * water.wave.z;
    let d_height_z = -sin_x * sin_z * water.wave.y * water.wave.z * 0.83;
    let local = position + vec3<f32>(
        0.0,
        height,
        0.0,
    );
    output.clip_position = camera.view_projection * model * vec4<f32>(local, 1.0);
    output.uv = uv;
    output.stable_normal = normalize(vec3<f32>(-d_height_x, 1.0, -d_height_z));
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
    let normal = normalize(input.stable_normal);
    let fresnel = pow(1.0 - clamp(normal.y, 0.0, 1.0), 5.0);
    let reflection = fresnel * water.quality.y;

    // The read-only LessEqual depth test above is the authoritative scene-depth
    // intersection. Its raster-space depth gradient supplies a bounded shore
    // cue without a frame-local depth texture or a second depth owner.
    let depth_gradient = length(vec2<f32>(dpdx(water_depth), dpdy(water_depth)));
    let shore_width = max(water.wave.w, water.viewport.z);
    let shore_intersection = 1.0 - smoothstep(0.0, shore_width, depth_gradient);
    let foam = shore_intersection * water.quality.z * water.quality.w;
    let base_tint = mix(water.colour_alpha.rgb * 0.82, water.colour_alpha.rgb * 1.12, ripple);
    let reflected_tint = vec3<f32>(0.58, 0.78, 0.96);
    let foam_tint = vec3<f32>(0.82, 0.94, 1.0);
    let tier_scale = select(1.0, 0.92, water.quality.x < 0.5);
    let tint = mix(base_tint, reflected_tint, reflection * tier_scale);
    let final_tint = mix(tint, foam_tint, clamp(foam, 0.0, 1.0));
    let final_alpha = clamp(water.colour_alpha.a + foam * 0.12, 0.0, 1.0);
    return vec4<f32>(final_tint, final_alpha);
}

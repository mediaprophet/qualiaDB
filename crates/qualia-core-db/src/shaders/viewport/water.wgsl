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
    @location(2) world_position: vec3<f32>,
    @location(3) shore_distance: f32,
    @location(4) shore_scale: f32,
};

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) shore: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    let phase_x = position.x * water.wave.z + water.wave.x * 0.85;
    let phase_z = position.z * water.wave.z * 0.83 - water.wave.x * 0.62;
    let sin_x = sin(phase_x);
    let cos_x = cos(phase_x);
    let sin_z = sin(phase_z);
    let cos_z = cos(phase_z);
    let shore_width = clamp(shore.y * 0.22, 0.0001, 0.008);
    let shore_fade = select(1.0, smoothstep(0.0, shore_width, shore.x), shore.y > 0.000001);
    let height = sin_x * cos_z * water.wave.y * shore_fade;
    // Analytic derivatives keep normals stable across tessellation and avoid a
    // second geometry upload just to carry a normal stream.
    let d_height_x = cos_x * cos_z * water.wave.y * water.wave.z * shore_fade;
    let d_height_z = -sin_x * sin_z * water.wave.y * water.wave.z * 0.83 * shore_fade;
    let local = position + vec3<f32>(
        0.0,
        height,
        0.0,
    );
    let world = model * vec4<f32>(local, 1.0);
    output.clip_position = camera.view_projection * world;
    output.world_position = world.xyz;
    output.shore_distance = shore.x;
    output.shore_scale = shore.y;
    output.uv = uv;
    output.stable_normal = normalize((model * vec4<f32>(-d_height_x, 1.0, -d_height_z, 0.0)).xyz);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // The render pass uses the same scene depth attachment with LessEqual and
    // depth writes disabled. This second guard makes the ownership explicit in
    // the shader contract and leaves a small tolerance for raster precision.
    // Fragment position is already rasterised: Z is device depth and W is
    // reciprocal clip W. Dividing them again incorrectly clips distant water.
    let water_depth = input.clip_position.z;
    if (water_depth > 1.0 + water.viewport.z || water_depth < 0.0) {
        discard;
    }

    let p = input.world_position.xz;
    let time = water.wave.x;
    let detail_frequency = max(water.wave.z, 0.001);
    let ripple = 0.5 + 0.5 * sin((p.x * 1.9 + p.y * 1.3) * detail_frequency + time * 1.7);
    let fine_scale = select(0.0, 0.035, water.quality.x >= 0.5);
    let fine = vec3<f32>(sin(p.x * detail_frequency * 3.1 + time * 1.2), 0.0, cos(p.y * detail_frequency * 2.7 - time * 0.9)) * fine_scale;
    let normal = normalize(input.stable_normal + fine);
    let eye = camera._padding0.yzw;
    let view_vector = eye - input.world_position;
    let view_dir = view_vector / max(length(view_vector), 0.00001);
    let n_dot_v = clamp(abs(dot(normal, view_dir)), 0.0, 1.0);
    let fresnel = 0.02 + 0.98 * pow(1.0 - n_dot_v, 5.0);
    let reflection = fresnel * water.quality.y;

    // Mesh boundary distance is a shoreline attribute, not measured bathymetry.
    // It follows curved banks and keeps foam off the middle of flat surfaces.
    let shore_width = clamp(input.shore_scale * 0.22, 0.0001, 0.008);
    let shore = select(0.0, 1.0 - smoothstep(0.0, shore_width, input.shore_distance), input.shore_scale > 0.000001);
    let foam_pattern = smoothstep(0.30, 0.80, ripple);
    let foam = shore * foam_pattern * water.quality.z * water.quality.w * 0.60;
    let depth_factor = select(0.65, 1.0 - exp(-max(input.shore_distance, 0.0) / max(input.shore_scale, 0.0001) * 2.5), input.shore_scale > 0.000001);
    let shallow = mix(water.colour_alpha.rgb, vec3<f32>(0.34, 0.47, 0.33), 0.28);
    let deep = water.colour_alpha.rgb * 0.70;
    let base_tint = mix(shallow, deep, depth_factor) * (0.96 + ripple * 0.08);
    let reflected_ray = reflect(-view_dir, normal);
    let reflected_tint = mix(vec3<f32>(0.68, 0.77, 0.78), vec3<f32>(0.30, 0.53, 0.76), clamp(reflected_ray.y, 0.0, 1.0));
    let foam_tint = vec3<f32>(0.76, 0.81, 0.73);
    let tier_scale = select(1.0, 0.92, water.quality.x < 0.5);
    let tint = mix(base_tint, reflected_tint, reflection * tier_scale);
    var sun = vec3<f32>(camera._padding1.w, camera._padding2.x, camera._padding2.y);
    if (dot(sun, sun) < 0.01) { sun = vec3<f32>(0.45, 0.8, 0.55); }
    let half_vector = normalize(normalize(sun) + view_dir + vec3<f32>(0.000001));
    let glint = pow(clamp(dot(normal, half_vector), 0.0, 1.0), 96.0) * water.quality.y * 0.55;
    let final_tint = mix(tint, foam_tint, clamp(foam, 0.0, 1.0)) + vec3<f32>(1.0, 0.91, 0.71) * glint;
    let final_alpha = clamp(water.colour_alpha.a * mix(0.38, 1.0, depth_factor) + foam * 0.12 + reflection * 0.12, 0.0, 1.0);
    return vec4<f32>(final_tint, final_alpha);
}

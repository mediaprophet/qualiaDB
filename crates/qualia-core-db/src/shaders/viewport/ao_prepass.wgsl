struct Camera {
    view_projection: mat4x4<f32>,
    tail: vec4<f32>,
    rest: array<vec4<f32>, 3>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;
struct MeshInstance {
    world_from_local: mat4x4<f32>,
    normal_from_local: mat3x3<f32>,
    semantic_id: vec2<u32>,
    orientation_sign: f32,
    _padding: u32,
};
@group(3) @binding(0) var<storage, read> mesh_instances: array<MeshInstance>;
struct AoUniform {
    viewport: vec4<f32>,
    eye: vec4<f32>,
    camera_forward: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    params: vec4<f32>,
    depth: vec4<f32>,
};
@group(0) @binding(2) var<uniform> ao: AoUniform;
struct Material {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    factors: vec4<f32>,
    flags: vec4<u32>,
    texture_flags: vec4<u32>,
    texture_flags_extra: vec4<u32>,
    coverage: vec4<u32>,
    sampler_modes: vec4<u32>,
    uv_transform: array<vec4<f32>, 12>,
};
@group(2) @binding(0) var<uniform> material: Material;
@group(2) @binding(1) var base_color_map: texture_2d<f32>;
@group(2) @binding(7) var base_color_sampler: sampler;
@group(2) @binding(8) var normal_sampler: sampler;
@group(2) @binding(9) var metallic_roughness_sampler: sampler;
@group(2) @binding(10) var occlusion_sampler: sampler;
@group(2) @binding(11) var emissive_sampler: sampler;
@group(2) @binding(12) var stylized_ramp_sampler: sampler;

fn material_uv(uv: vec2<f32>, role: u32) -> vec2<f32> {
    let offset_scale = material.uv_transform[role * 2u];
    let rotation = material.uv_transform[role * 2u + 1u];
    let scaled = uv * offset_scale.zw;
    return vec2<f32>(
        rotation.x * scaled.x - rotation.y * scaled.y,
        rotation.y * scaled.x + rotation.x * scaled.y
    ) + offset_scale.xy;
}

fn use_nearest_texel(uv: vec2<f32>, dims: vec2<u32>) -> bool {
    let record = material.sampler_modes[0] & 127u;
    let min_filter = record & 7u;
    let min_nearest = min_filter == 0u || min_filter == 2u;
    let mag_nearest = (material.coverage.w & 1u) != 0u;
    let dx = dpdx(uv) * vec2<f32>(dims);
    let dy = dpdy(uv) * vec2<f32>(dims);
    let rho = max(length(dx), length(dy));
    return (mag_nearest && rho <= 1.0) || (min_nearest && rho > 1.0);
}

fn nearest_base_color_texel(uv: vec2<f32>) -> vec3<i32> {
    let record = material.sampler_modes[0] & 127u;
    var wrapped = uv;
    let wrap_s = (record >> 3u) & 3u;
    let wrap_t = (record >> 5u) & 3u;
    if (wrap_s == 0u) { wrapped.x = clamp(wrapped.x, 0.0, 1.0); }
    if (wrap_s == 1u) {
        let phase = wrapped.x - 2.0 * floor(wrapped.x / 2.0);
        wrapped.x = select(phase, 2.0 - phase, phase > 1.0);
    }
    if (wrap_s == 2u) { wrapped.x = wrapped.x - floor(wrapped.x); }
    if (wrap_t == 0u) { wrapped.y = clamp(wrapped.y, 0.0, 1.0); }
    if (wrap_t == 1u) {
        let phase = wrapped.y - 2.0 * floor(wrapped.y / 2.0);
        wrapped.y = select(phase, 2.0 - phase, phase > 1.0);
    }
    if (wrap_t == 2u) { wrapped.y = wrapped.y - floor(wrapped.y); }
    let dims = textureDimensions(base_color_map, 0);
    var level = 0u;
    if ((record & 7u) == 2u && textureNumLevels(base_color_map) > 1u) {
        let dx = dpdx(uv) * vec2<f32>(dims);
        let dy = dpdy(uv) * vec2<f32>(dims);
        let lod = clamp(log2(max(max(length(dx), length(dy)), 0.000001)), 0.0, f32(textureNumLevels(base_color_map) - 1u));
        level = u32(floor(lod + 0.5));
    }
    let level_dims = max(dims >> vec2<u32>(level), vec2<u32>(1u));
    let pixel = min(vec2<u32>(floor(wrapped * vec2<f32>(level_dims))), level_dims - vec2<u32>(1u));
    return vec3<i32>(vec2<i32>(pixel), i32(level));
}

fn sample_base_color(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 1u) != 0u && use_nearest_texel(uv, textureDimensions(base_color_map, 0))) {
        let p = nearest_base_color_texel(uv);
        return textureLoad(base_color_map, p.xy, p.z);
    }
    return textureSample(base_color_map, base_color_sampler, uv);
}

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
};

fn oct_encode(normal: vec3<f32>) -> vec2<f32> {
    let n = normalize(normal);
    let projected = n.xy / (abs(n.x) + abs(n.y) + abs(n.z));
    var folded = projected;
    if (n.z < 0.0) {
        folded = (vec2<f32>(1.0) - abs(projected.yx))
            * select(vec2<f32>(-1.0), vec2<f32>(1.0), projected >= vec2<f32>(0.0));
    }
    return folded * 0.5 + vec2<f32>(0.5);
}

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @builtin(instance_index) instance_index: u32
) -> Output {
    var output: Output;
    let instance_model = model * mesh_instances[instance_index].world_from_local;
    let world_position = (instance_model * vec4<f32>(position, 1.0)).xyz;
    output.position = camera.view_projection * vec4<f32>(world_position, 1.0);
    output.normal = normalize((instance_model * vec4<f32>(normal, 0.0)).xyz);
    output.world_position = world_position;
    output.color = color;
    output.uv0 = uv0;
    return output;
}

// Pack octahedral world normal (RG8) and normalized linear view depth (BA16)
// into one universally renderable RGBA8 target. Code 0 is the clear/background
// sentinel; valid surface depths occupy 1..65535.
@fragment
fn fragment_main(input: Output) -> @location(0) vec4<f32> {
    if (material.coverage.x == 1u) {
        let vertex_alpha = select(1.0, input.color.a, material.flags.y != 0u);
        let base_sample = select(
            1.0,
            sample_base_color(material_uv(input.uv0, 0u)).a,
            material.texture_flags.x != 0u
        );
        let alpha = material.base_color.a * vertex_alpha * base_sample;
        if (alpha < material.factors.w) {
            discard;
        }
    }
    let eye = camera.rest[0].yzw;
    let camera_focus = camera.rest[1].xyz;
    let forward = normalize(camera_focus - eye);
    let view_depth = max(dot(input.world_position - eye, forward), 0.0);
    let normalized_depth = clamp(view_depth / ao.depth.x, 0.0, 1.0);
    let code = 1u + u32(round(normalized_depth * 65534.0));
    let high = (code >> 8u) & 255u;
    let low = code & 255u;
    let encoded_normal = oct_encode(input.normal);
    return vec4<f32>(
        encoded_normal,
        f32(high) * (1.0 / 255.0),
        f32(low) * (1.0 / 255.0)
    );
}

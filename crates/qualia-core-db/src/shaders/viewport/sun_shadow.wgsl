struct ShadowUniform {
    light_view_projection: array<mat4x4<f32>, 2>,
    params: vec4<f32>, // enabled, inverse size, receiver bias, camera split depth
};

@group(0) @binding(0) var<uniform> shadow: ShadowUniform;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;
struct MeshInstance {
    world_from_local: mat4x4<f32>,
    normal_from_local: mat3x3<f32>,
    semantic_id: vec2<u32>,
    orientation_sign: f32,
    _padding: u32,
};
@group(4) @binding(0) var<storage, read> mesh_instances: array<MeshInstance>;
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
@group(3) @binding(0) var base_color_map: texture_2d<f32>;
@group(3) @binding(6) var base_color_sampler: sampler;
@group(3) @binding(7) var normal_sampler: sampler;
@group(3) @binding(8) var metallic_roughness_sampler: sampler;
@group(3) @binding(9) var occlusion_sampler: sampler;
@group(3) @binding(10) var emissive_sampler: sampler;
@group(3) @binding(11) var stylized_ramp_sampler: sampler;

fn material_uv(uv: vec2<f32>, role: u32) -> vec2<f32> {
    let offset_scale = material.uv_transform[role * 2u];
    let rotation = material.uv_transform[role * 2u + 1u];
    let scaled = uv * offset_scale.zw;
    return vec2<f32>(
        rotation.x * scaled.x - rotation.y * scaled.y,
        rotation.y * scaled.x + rotation.x * scaled.y
    ) + offset_scale.xy;
}

fn sample_base_color(uv: vec2<f32>) -> vec4<f32> {
    let record = material.sampler_modes[0] & 127u;
    let dims = textureDimensions(base_color_map, 0);
    let dx = dpdx(uv) * vec2<f32>(dims);
    let dy = dpdy(uv) * vec2<f32>(dims);
    let rho = max(length(dx), length(dy));
    let min_filter = record & 7u;
    let min_nearest = min_filter == 0u || min_filter == 2u;
    let mag_nearest = (material.coverage.w & 1u) != 0u;
    if ((material.coverage.y & 1u) != 0u && ((mag_nearest && rho <= 1.0) || (min_nearest && rho > 1.0))) {
        let wrap_s = (record >> 3u) & 3u;
        let wrap_t = (record >> 5u) & 3u;
        var wrapped = uv;
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
        var level = 0u;
        if ((record & 7u) == 2u && textureNumLevels(base_color_map) > 1u) {
            let dx = dpdx(uv) * vec2<f32>(dims);
            let dy = dpdy(uv) * vec2<f32>(dims);
            let lod = clamp(log2(max(max(length(dx), length(dy)), 0.000001)), 0.0, f32(textureNumLevels(base_color_map) - 1u));
            level = u32(floor(lod + 0.5));
        }
        let level_dims = max(dims >> vec2<u32>(level), vec2<u32>(1u));
        let pixel = min(vec2<u32>(floor(wrapped * vec2<f32>(level_dims))), level_dims - vec2<u32>(1u));
        return textureLoad(base_color_map, vec2<i32>(pixel), i32(level));
    }
    return textureSample(base_color_map, base_color_sampler, uv);
}

struct ShadowVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv0: vec2<f32>,
    @interpolate(flat) @location(2) semantic_id: vec2<u32>,
};

@vertex
fn shadow_vertex_near(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv0: vec2<f32>,
    @builtin(instance_index) instance_index: u32
) -> ShadowVertexOutput {
    var output: ShadowVertexOutput;
    let instance = mesh_instances[instance_index];
    output.clip_position = shadow.light_view_projection[0] * model * instance.world_from_local * vec4<f32>(position, 1.0);
    output.color = color;
    output.uv0 = uv0;
    output.semantic_id = instance.semantic_id;
    return output;
}

@vertex
fn shadow_vertex_far(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv0: vec2<f32>,
    @builtin(instance_index) instance_index: u32
) -> ShadowVertexOutput {
    var output: ShadowVertexOutput;
    let instance = mesh_instances[instance_index];
    output.clip_position = shadow.light_view_projection[1] * model * instance.world_from_local * vec4<f32>(position, 1.0);
    output.color = color;
    output.uv0 = uv0;
    output.semantic_id = instance.semantic_id;
    return output;
}

@fragment
fn shadow_mask_fragment(input: ShadowVertexOutput) {
    if (material.coverage.x != 1u) {
        return;
    }
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

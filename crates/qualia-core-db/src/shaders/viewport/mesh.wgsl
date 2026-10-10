// Triangle-mesh renderer — imported OBJ / STL / GLB surfaces (Phase 1.2,
// RENDERER_IMPLEMENTATION_PLAN.md). Uses an area-weighted normal stream for
// smooth legacy direct lighting. Shares the orbit camera with projector/ambient.

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
// Per-artefact model transform (Phase 2): the kinematic-joint pose, identity when not animating.
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;
struct MeshInstance {
    world_from_local: mat4x4<f32>,
    normal_from_local: mat3x3<f32>,
    semantic_id: vec2<u32>,
    orientation_sign: f32,
    _padding: u32,
};
@group(3) @binding(0) var<storage, read> mesh_instances: array<MeshInstance>;

struct Material {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    factors: vec4<f32>, // metallic, roughness, dielectric specular, alpha cutoff
    flags: vec4<u32>,   // model: 0 physical / 1 stylized; multiply vertex colour
    texture_flags: vec4<u32>, // base, normal, metallic-roughness, occlusion
    texture_flags_extra: vec4<u32>, // emissive, stylized ramp, AO strength bits, normal scale bits
    coverage: vec4<u32>, // 0 opaque, 1 mask, 2 blend
    sampler_modes: vec4<u32>, // six packed 7-bit min-filter/U/V address records
    uv_transform: array<vec4<f32>, 12>,
};
@group(2) @binding(0) var<uniform> material: Material;
@group(2) @binding(1) var base_color_map: texture_2d<f32>;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var metallic_roughness_map: texture_2d<f32>;
@group(2) @binding(4) var occlusion_map: texture_2d<f32>;
@group(2) @binding(5) var emissive_map: texture_2d<f32>;
@group(2) @binding(6) var stylized_ramp_map: texture_2d<f32>;
@group(2) @binding(7) var base_color_sampler: sampler;
@group(2) @binding(8) var normal_sampler: sampler;
@group(2) @binding(9) var metallic_roughness_sampler: sampler;
@group(2) @binding(10) var occlusion_sampler: sampler;
@group(2) @binding(11) var emissive_sampler: sampler;
@group(2) @binding(12) var stylized_ramp_sampler: sampler;

// Fixed-size, texture-independent probe source data. The CPU only packs authored probe inputs;
// this shader performs the bounded blend so a CPU lighting result is never masqueraded as a bind.
struct EnvironmentProbe {
    position_radius: vec4<f32>,
    irradiance: vec4<f32>,
    specular_levels: array<vec4<f32>, 5>,
    dominant_valid: vec4<f32>,
};
struct EnvironmentLighting {
    metadata: vec4<u32>, // probe count, usable probe count, reserved, reserved
    direct_fallback: vec4<f32>,
    probes: array<EnvironmentProbe, 4>,
};
@group(2) @binding(13) var<uniform> environment: EnvironmentLighting;

struct ShadowUniform {
    light_view_projection: array<mat4x4<f32>, 2>,
    params: vec4<f32>, // enabled, inverse size, receiver bias, cascade split depth
};
@group(0) @binding(2) var sun_shadow_near: texture_depth_2d;
@group(0) @binding(3) var sun_shadow_far: texture_depth_2d;
@group(0) @binding(4) var sun_shadow_sampler: sampler_comparison;
@group(0) @binding(5) var<uniform> sun_shadow: ShadowUniform;

struct AoUniform {
    viewport: vec4<f32>,
    eye: vec4<f32>,
    camera_forward: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    params: vec4<f32>,
    depth: vec4<f32>,
};
@group(0) @binding(6) var dynamic_ao_map: texture_2d<f32>;
@group(0) @binding(7) var dynamic_ao_surface: texture_2d<f32>;
@group(0) @binding(8) var<uniform> dynamic_ao: AoUniform;

struct AtmosphereUniform {
    fog_color_density: vec4<f32>, // linear RGB + extinction density
    height: vec4<f32>, // base height, falloff, maximum opacity, enabled
};
@group(0) @binding(9) var<uniform> atmosphere: AtmosphereUniform;

fn material_uv(uv: vec2<f32>, role: u32) -> vec2<f32> {
    let offset_scale = material.uv_transform[role * 2u];
    let rotation = material.uv_transform[role * 2u + 1u];
    let scaled = uv * offset_scale.zw;
    return vec2<f32>(
        rotation.x * scaled.x - rotation.y * scaled.y,
        rotation.y * scaled.x + rotation.x * scaled.y
    ) + offset_scale.xy;
}

fn sampler_record(role: u32) -> u32 {
    let word = select(material.sampler_modes[0], material.sampler_modes[1], role >= 4u);
    let slot = select(role, role - 4u, role >= 4u);
    return (word >> (slot * 7u)) & 127u;
}

fn use_nearest_texel(uv: vec2<f32>, role: u32, dims: vec2<u32>) -> bool {
    let record = sampler_record(role);
    let min_filter = record & 7u;
    let min_nearest = min_filter == 0u || min_filter == 2u;
    let mag_nearest = (material.coverage.w & (1u << role)) != 0u;
    let dx = dpdx(uv) * vec2<f32>(dims);
    let dy = dpdy(uv) * vec2<f32>(dims);
    let rho = max(length(dx), length(dy));
    return (mag_nearest && rho <= 1.0) || (min_nearest && rho > 1.0);
}

fn nearest_texel(uv: vec2<f32>, role: u32, dims: vec2<u32>, levels: u32) -> vec3<i32> {
    let record = sampler_record(role);
    let min_filter = record & 7u;
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
    if (min_filter == 2u && levels > 1u) {
        let dx = dpdx(uv) * vec2<f32>(dims);
        let dy = dpdy(uv) * vec2<f32>(dims);
        let rho = max(length(dx), length(dy));
        let lod = clamp(log2(max(rho, 0.000001)), 0.0, f32(levels - 1u));
        level = u32(floor(lod + 0.5));
    }
    let level_dims = max(dims >> vec2<u32>(level), vec2<u32>(1u));
    let pixel = min(vec2<u32>(floor(wrapped * vec2<f32>(level_dims))), level_dims - vec2<u32>(1u));
    return vec3<i32>(vec2<i32>(pixel), i32(level));
}

fn sample_base_color(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 1u) != 0u && use_nearest_texel(uv, 0u, textureDimensions(base_color_map, 0))) {
        let p = nearest_texel(uv, 0u, textureDimensions(base_color_map, 0), textureNumLevels(base_color_map));
        return textureLoad(base_color_map, p.xy, p.z);
    }
    return textureSample(base_color_map, base_color_sampler, uv);
}

fn sample_normal(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 2u) != 0u && use_nearest_texel(uv, 1u, textureDimensions(normal_map, 0))) {
        let p = nearest_texel(uv, 1u, textureDimensions(normal_map, 0), textureNumLevels(normal_map));
        return textureLoad(normal_map, p.xy, p.z);
    }
    return textureSample(normal_map, normal_sampler, uv);
}

fn sample_metallic_roughness(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 4u) != 0u && use_nearest_texel(uv, 2u, textureDimensions(metallic_roughness_map, 0))) {
        let p = nearest_texel(uv, 2u, textureDimensions(metallic_roughness_map, 0), textureNumLevels(metallic_roughness_map));
        return textureLoad(metallic_roughness_map, p.xy, p.z);
    }
    return textureSample(metallic_roughness_map, metallic_roughness_sampler, uv);
}

fn sample_occlusion(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 8u) != 0u && use_nearest_texel(uv, 3u, textureDimensions(occlusion_map, 0))) {
        let p = nearest_texel(uv, 3u, textureDimensions(occlusion_map, 0), textureNumLevels(occlusion_map));
        return textureLoad(occlusion_map, p.xy, p.z);
    }
    return textureSample(occlusion_map, occlusion_sampler, uv);
}

fn sample_emissive(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 16u) != 0u && use_nearest_texel(uv, 4u, textureDimensions(emissive_map, 0))) {
        let p = nearest_texel(uv, 4u, textureDimensions(emissive_map, 0), textureNumLevels(emissive_map));
        return textureLoad(emissive_map, p.xy, p.z);
    }
    return textureSample(emissive_map, emissive_sampler, uv);
}

fn sample_stylized_ramp(uv: vec2<f32>) -> vec4<f32> {
    if ((material.coverage.y & 32u) != 0u && use_nearest_texel(uv, 5u, textureDimensions(stylized_ramp_map, 0))) {
        let p = nearest_texel(uv, 5u, textureDimensions(stylized_ramp_map, 0), textureNumLevels(stylized_ramp_map));
        return textureLoad(stylized_ramp_map, p.xy, p.z);
    }
    return textureSample(stylized_ramp_map, stylized_ramp_sampler, uv);
}

fn environment_probe_weight(world_position: vec3<f32>, probe: EnvironmentProbe) -> f32 {
    if (probe.dominant_valid.w < 0.5 || probe.position_radius.w <= 0.0) {
        return 0.0;
    }
    let delta = world_position - probe.position_radius.xyz;
    let distance_sq = dot(delta, delta);
    let radius_sq = probe.position_radius.w * probe.position_radius.w;
    if (distance_sq >= radius_sq) {
        return 0.0;
    }
    let coverage = clamp(1.0 - distance_sq / radius_sq, 0.0, 1.0);
    return (1.0 / (distance_sq + 1.0)) * coverage * coverage;
}

fn sample_environment_lighting(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32
) -> vec3<f32> {
    if (environment.metadata.y == 0u) {
        return environment.direct_fallback.rgb;
    }
    let n_dot_v = clamp(dot(normal, view_dir), 0.0, 1.0);
    let f0 = mix(vec3<f32>(0.04), base, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - n_dot_v, 5.0);
    let scaled_roughness = clamp(roughness, 0.0, 1.0) * 4.0;
    let lower = min(u32(floor(scaled_roughness)), 4u);
    let upper = min(lower + 1u, 4u);
    let level_mix = scaled_roughness - f32(lower);
    var diffuse_accum = vec3<f32>(0.0);
    var specular_accum = vec3<f32>(0.0);
    var total_weight = 0.0;
    let probe_count = min(environment.metadata.x, 4u);
    for (var index = 0u; index < 4u; index = index + 1u) {
        if (index >= probe_count) { continue; }
        let probe = environment.probes[index];
        let weight = environment_probe_weight(world_position, probe);
        if (weight <= 0.0) { continue; }
        let prefiltered = mix(
            probe.specular_levels[lower].rgb,
            probe.specular_levels[upper].rgb,
            level_mix
        );
        let n_dot_l = max(dot(normal, normalize(probe.dominant_valid.xyz)), 0.0);
        diffuse_accum += (1.0 - metallic) * base * probe.irradiance.rgb
            * (n_dot_l / 3.14159265) * ao * weight;
        specular_accum += prefiltered * fresnel * weight;
        total_weight += weight;
    }
    if (total_weight <= 1e-6) {
        return environment.direct_fallback.rgb;
    }
    return (diffuse_accum + specular_accum) / total_weight;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) sun_shadow_near_position: vec4<f32>,
    @location(5) sun_shadow_far_position: vec4<f32>,
    @location(6) camera_depth: f32,
    @location(7) world_position: vec3<f32>,
    @interpolate(flat) @location(8) semantic_id: vec2<u32>,
    @interpolate(flat) @location(9) instance_index: u32,
};

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) uv0: vec2<f32>,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var output: VertexOutput;
    let instance = mesh_instances[instance_index];
    let instance_model = model * instance.world_from_local;
    let world = (instance_model * vec4<f32>(position, 1.0)).xyz;
    output.normal = normalize((model * vec4<f32>(instance.normal_from_local * normal, 0.0)).xyz);
    let tangent_world = (instance_model * vec4<f32>(tangent.xyz, 0.0)).xyz;
    output.tangent = vec4<f32>(
        normalize(tangent_world - output.normal * dot(output.normal, tangent_world)),
        tangent.w * instance.orientation_sign
    );
    output.semantic_id = instance.semantic_id;
    output.instance_index = instance_index;
    output.color = color;
    output.uv0 = uv0;
    output.sun_shadow_near_position = sun_shadow.light_view_projection[0] * vec4<f32>(world, 1.0);
    output.sun_shadow_far_position = sun_shadow.light_view_projection[1] * vec4<f32>(world, 1.0);
    let camera_eye = camera._padding0.yzw;
    let camera_forward = normalize(camera._padding1.xyz - camera_eye);
    output.camera_depth = dot(world - camera_eye, camera_forward);
    output.world_position = world;
    output.clip_position = camera.view_projection * vec4<f32>(world, 1.0);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var n = normalize(input.normal);
    if (material.texture_flags.y != 0u) {
        let encoded = sample_normal(material_uv(input.uv0, 1u)).xyz * 2.0 - vec3<f32>(1.0);
        let normal_scale = bitcast<f32>(material.texture_flags_extra.w);
        let tangent = normalize(input.tangent.xyz);
        let bitangent = normalize(cross(n, tangent)) * input.tangent.w;
        n = normalize(tangent * (encoded.x * normal_scale)
            + bitangent * (encoded.y * normal_scale) + n * encoded.z);
    }

    // Configurable directional key/sun light:
    // _padding1.w = sun_dir_x, _padding2.x = sun_dir_y, _padding2.y = sun_dir_z
    var sun_dir = vec3<f32>(camera._padding1.w, camera._padding2.x, camera._padding2.y);
    if (dot(sun_dir, sun_dir) < 0.01) {
        sun_dir = vec3<f32>(0.45, 0.8, 0.55);
    }
    let key = normalize(sun_dir);

    // CameraState supplies defaults. Preserve explicit zero intensities so hosts
    // can disable direct and ambient illumination for diagnostic/unlit fixtures.
    let sun_int = camera._padding2.z;
    let amb_int = camera._padding2.w;

    let use_vertex_color = material.flags.y != 0u;
    let vertex_rgb = select(vec3<f32>(1.0), input.color.rgb, use_vertex_color);
    let vertex_alpha = select(1.0, input.color.a, use_vertex_color);
    let base_sample = select(
        vec4<f32>(1.0),
        sample_base_color(material_uv(input.uv0, 0u)),
        material.texture_flags.x != 0u
    );
    let emissive_sample = select(
        vec3<f32>(1.0),
        sample_emissive(material_uv(input.uv0, 4u)).rgb,
        material.texture_flags_extra.x != 0u
    );
    let base = material.base_color.rgb * vertex_rgb * base_sample.rgb;
    let sampled_alpha = material.base_color.a * vertex_alpha * base_sample.a;
    if (material.coverage.x == 1u && sampled_alpha < material.factors.w) {
        discard;
    }
    // Opaque and surviving mask fragments are fully covered. Blend surfaces preserve authored
    // base/vertex/texture alpha and are sorted in a separate read-only-depth pass.
    let alpha = select(1.0, sampled_alpha, material.coverage.x == 2u);
    let ndotl = max(dot(n, key), 0.0);
    let sun_visibility = cascaded_shadow_visibility(
        input.sun_shadow_near_position,
        input.sun_shadow_far_position,
        input.camera_depth,
        n,
        key
    );
    let ao_sample = select(
        1.0,
        sample_occlusion(material_uv(input.uv0, 3u)).r,
        material.texture_flags.w != 0u
    );
    let ao_strength = bitcast<f32>(material.texture_flags_extra.z);
    let baked_visibility = mix(1.0, ao_sample, ao_strength);
    // Combining by minimum treats baked material AO and dynamic scene AO as alternate
    // visibility bounds instead of multiplying both and counting the same crevice twice.
    let ambient_visibility = min(baked_visibility, screen_space_ao_visibility(
        input.clip_position, n, input.camera_depth
    ));
    let emissive = material.emissive.rgb * emissive_sample;

    let mr_sample = select(
        vec4<f32>(0.0, 1.0, 0.0, 1.0),
        sample_metallic_roughness(material_uv(input.uv0, 2u)),
        material.texture_flags.z != 0u
    );
    let roughness = clamp(material.factors.y * mr_sample.g, 0.045, 1.0);
    let metallic = clamp(material.factors.x * mr_sample.b, 0.0, 1.0);
    let view_dir = normalize(vec3<f32>(sin(camera.yaw), sin(camera.pitch), cos(camera.yaw)));
    let environment_radiance = sample_environment_lighting(
        input.world_position,
        n,
        view_dir,
        base,
        metallic,
        roughness,
        ambient_visibility
    );

    if (material.flags.x == 1u) {
        // Portable animated-film fallback: stable three-band diffuse with authored base and
        // emissive factors. An authored ramp texture remains a separate, explicitly gated path.
        var band = 0.20;
        if (ndotl >= 0.66) {
            band = 1.0;
        } else if (ndotl >= 0.22) {
            band = 0.62;
        }
        let facing = clamp(abs(n.z), 0.0, 1.0);
        let rim = pow(1.0 - facing, 3.0);
        let ramp = select(
            vec3<f32>(1.0),
            sample_stylized_ramp(material_uv(vec2<f32>(ndotl, 0.5), 5u)).rgb,
            material.texture_flags_extra.y != 0u
        );
        let col = environment_radiance
            + base * (amb_int * ambient_visibility + band * sun_int * sun_visibility) * ramp + emissive
            + base * rim * 0.12;
        return vec4<f32>(apply_atmosphere(col, input.world_position), alpha);
    }

    // Direct-light GGX metallic-roughness baseline. All factors are linear; the output attachment
    // owns display encoding. Environment/probe lighting is a bounded material contribution.
    let dielectric_f0 = clamp(material.factors.z, 0.0, 1.0) * 0.04;
    let half_dir = normalize(key + view_dir + vec3<f32>(0.0, 0.0, 1e-8));
    let ndotv = max(dot(n, view_dir), 1e-4);
    let ndoth = max(dot(n, half_dir), 0.0);
    let vdoth = max(dot(view_dir, half_dir), 0.0);
    let alpha_g = roughness * roughness;
    let alpha_g2 = alpha_g * alpha_g;
    let denom = ndoth * ndoth * (alpha_g2 - 1.0) + 1.0;
    let distribution = alpha_g2 / max(3.14159265 * denom * denom, 1e-6);
    let k = (roughness + 1.0) * (roughness + 1.0) * 0.125;
    let geometry_v = ndotv / (ndotv * (1.0 - k) + k);
    let geometry_l = ndotl / (ndotl * (1.0 - k) + k);
    let f0 = mix(vec3<f32>(dielectric_f0), base, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - vdoth, 5.0);
    let specular = distribution * geometry_v * geometry_l * fresnel
        / max(4.0 * ndotv * ndotl, 1e-4);
    let diffuse = (vec3<f32>(1.0) - fresnel) * base * ((1.0 - metallic) / 3.14159265);
    let ambient = base * (1.0 - metallic) * amb_int * ambient_visibility;
    let col = environment_radiance
        + ambient + (diffuse + specular) * (ndotl * sun_int * sun_visibility) + emissive;
    return vec4<f32>(apply_atmosphere(col, input.world_position), alpha);
}

// R32Uint carries a tagged visible-instance slot. The CPU resolves the slot through the
// submission-time semantic-ID snapshot, preserving the full u64 identity without shader-u64
// requirements. Alpha MASK follows forward coverage; nearly invisible BLEND fragments do not
// steal a pick from visible geometry behind them.
@fragment
fn picking_fragment_main(input: VertexOutput) -> @location(0) u32 {
    let use_vertex_color = material.flags.y != 0u;
    let vertex_alpha = select(1.0, input.color.a, use_vertex_color);
    let base_sample = select(
        vec4<f32>(1.0),
        sample_base_color(material_uv(input.uv0, 0u)),
        material.texture_flags.x != 0u
    );
    let alpha = material.base_color.a * vertex_alpha * base_sample.a;
    if (material.coverage.x == 1u && alpha < material.factors.w) {
        discard;
    }
    if (material.coverage.x == 2u && alpha < 0.05) {
        discard;
    }
    return 0x80000000u | input.instance_index;
}

fn apply_atmosphere(rgb: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let density = atmosphere.fog_color_density.w;
    if (atmosphere.height.w < 0.5 || density <= 0.0) {
        return rgb;
    }
    let eye = camera._padding0.yzw;
    let distance = length(world_position - eye);
    let mean_height = max(0.0, 0.5 * (world_position.y + eye.y) - atmosphere.height.x);
    let height_density = density * exp(-atmosphere.height.y * mean_height);
    let transmittance = exp(-height_density * distance);
    let fog_amount = clamp(1.0 - transmittance, 0.0, atmosphere.height.z);
    return mix(rgb, atmosphere.fog_color_density.rgb, fog_amount);
}

fn cascaded_shadow_visibility(
    near_clip: vec4<f32>,
    far_clip: vec4<f32>,
    camera_depth: f32,
    normal: vec3<f32>,
    light: vec3<f32>
) -> f32 {
    if (sun_shadow.params.x < 0.5 || material.flags.w == 0u) {
        return 1.0;
    }
    let split = sun_shadow.params.w;
    let overlap = clamp(split * 0.08, 0.5, 4.0);
    let near_visibility = sample_shadow_map(sun_shadow_near, near_clip, normal, light);
    if (camera_depth <= split - overlap) {
        return near_visibility;
    }
    let far_visibility = sample_shadow_map(sun_shadow_far, far_clip, normal, light);
    if (camera_depth >= split + overlap) {
        return far_visibility;
    }
    let blend = smoothstep(split - overlap, split + overlap, camera_depth);
    return mix(near_visibility, far_visibility, blend);
}

fn sample_shadow_map(
    shadow_map: texture_depth_2d,
    light_clip: vec4<f32>,
    normal: vec3<f32>,
    light: vec3<f32>
) -> f32 {
    if (light_clip.w <= 0.0) {
        return 1.0;
    }
    let ndc = light_clip.xyz / light_clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (ndc.z < 0.0 || ndc.z > 1.0 || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return 1.0;
    }
    let slope_bias = sun_shadow.params.z * (1.0 - max(dot(normal, light), 0.0));
    let compare_depth = ndc.z - slope_bias;
    let texel = sun_shadow.params.y;
    var visibility = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            visibility += textureSampleCompareLevel(
                shadow_map,
                sun_shadow_sampler,
                clamp(uv + offset, vec2<f32>(0.0), vec2<f32>(1.0)),
                compare_depth
            );
        }
    }
    return visibility / 9.0;
}

fn screen_space_ao_visibility(position: vec4<f32>, normal: vec3<f32>, depth: f32) -> f32 {
    if (dynamic_ao.params.w < 0.5) {
        return 1.0;
    }
    let ao_position = position.xy * 0.5 - vec2<f32>(0.5);
    let base = vec2<i32>(floor(ao_position));
    let fraction = fract(ao_position);
    let dims = vec2<i32>(dynamic_ao.viewport.xy);
    let n = normalize(normal);
    var total = 0.0;
    var total_weight = 0.0;
    for (var y = 0; y < 2; y = y + 1) {
        for (var x = 0; x < 2; x = x + 1) {
            let coord = clamp(base + vec2<i32>(x, y), vec2<i32>(0), dims - vec2<i32>(1));
            let surface = textureLoad(dynamic_ao_surface, coord, 0);
            let depth_code = (u32(round(surface.b * 255.0)) << 8u)
                | u32(round(surface.a * 255.0));
            if (depth_code == 0u) { continue; }
            let stored_depth = f32(depth_code - 1u) * dynamic_ao.depth.y;
            let encoded_normal = surface.rg * 2.0 - vec2<f32>(1.0);
            var stored_normal = vec3<f32>(
                encoded_normal,
                1.0 - abs(encoded_normal.x) - abs(encoded_normal.y)
            );
            let fold = clamp(-stored_normal.z, 0.0, 1.0);
            stored_normal.x += select(-fold, fold, stored_normal.x >= 0.0);
            stored_normal.y += select(-fold, fold, stored_normal.y >= 0.0);
            stored_normal = normalize(stored_normal);
            let ao = textureLoad(dynamic_ao_map, coord, 0).r;
            let wx = select(1.0 - fraction.x, fraction.x, x == 1);
            let wy = select(1.0 - fraction.y, fraction.y, y == 1);
            let bilateral = exp(-abs(stored_depth - depth) * 96.0)
                * pow(max(dot(n, stored_normal), 0.0), 16.0);
            let weight = wx * wy * bilateral;
            total += ao * weight;
            total_weight += weight;
        }
    }
    return select(1.0, total / max(total_weight, 1e-6), total_weight > 1e-5);
}
